#![allow(missing_docs)]

use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
use base64_ng_tokio::{DecoderWriter, EncoderWriter};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc, task::Wake};
use tokio::io::{AsyncWrite, AsyncWriteExt};

enum WriteAction {
    Accept(usize),
    Error,
    Pending,
}

struct ScriptedWriter {
    actions: VecDeque<WriteAction>,
    output: Vec<u8>,
    shutdown: bool,
}

impl ScriptedWriter {
    fn new(actions: impl IntoIterator<Item = WriteAction>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
            output: Vec::new(),
            shutdown: false,
        }
    }

    fn output(&self) -> &[u8] {
        &self.output
    }
}

impl AsyncWrite for ScriptedWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.actions.pop_front() {
            Some(WriteAction::Accept(limit)) => {
                let count = limit.min(input.len());
                self.output.extend_from_slice(&input[..count]);
                Poll::Ready(Ok(count))
            }
            Some(WriteAction::Pending) => {
                context.waker().wake_by_ref();
                Poll::Pending
            }
            Some(WriteAction::Error) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "scripted write error",
            ))),
            None => {
                self.output.extend_from_slice(input);
                Poll::Ready(Ok(input.len()))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.shutdown = true;
        Poll::Ready(Ok(()))
    }
}

struct NoopWake;

#[allow(clippy::manual_noop_waker)]
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn noop_waker() -> std::task::Waker {
    std::task::Waker::from(Arc::new(NoopWake))
}

#[tokio::test]
async fn streaming_encoder_writer_handles_split_writes() {
    let mut writer = EncoderWriter::new(Vec::new(), STANDARD);

    writer.write_all(b"he").await.unwrap();
    writer.write_all(b"llo").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), b"aGVsbG8=");
}

#[tokio::test]
async fn streaming_decoder_writer_handles_split_quanta() {
    let mut writer = DecoderWriter::new(Vec::new(), STANDARD);

    writer.write_all(b"aG").await.unwrap();
    writer.write_all(b"VsbG8=").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), b"hello");
}

#[tokio::test]
async fn streaming_encoder_writer_resumes_after_pending_shutdown_drain() {
    let inner = ScriptedWriter::new([
        WriteAction::Accept(2),
        WriteAction::Pending,
        WriteAction::Accept(usize::MAX),
    ]);
    let mut writer = EncoderWriter::new(inner, STANDARD);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    writer.write_all(b"hello").await.unwrap();

    assert!(
        Pin::new(&mut writer)
            .poll_shutdown(&mut context)
            .is_pending()
    );
    assert_eq!(writer.get_ref().output(), b"aG");

    writer.shutdown().await.unwrap();

    let inner = writer.into_inner();
    assert_eq!(inner.output(), b"aGVsbG8=");
}

#[tokio::test]
async fn streaming_decoder_writer_resumes_after_pending_shutdown_drain() {
    let inner = ScriptedWriter::new([
        WriteAction::Accept(1),
        WriteAction::Pending,
        WriteAction::Accept(usize::MAX),
    ]);
    let mut writer = DecoderWriter::new(inner, STANDARD);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    writer.write_all(b"aGVsbG8=").await.unwrap();

    assert!(
        Pin::new(&mut writer)
            .poll_shutdown(&mut context)
            .is_pending()
    );
    assert_eq!(writer.get_ref().output(), b"h");

    writer.shutdown().await.unwrap();

    let inner = writer.into_inner();
    assert_eq!(inner.output(), b"hello");
}

#[tokio::test]
async fn streaming_decoder_writer_fails_closed_after_malformed_input() {
    let mut writer = DecoderWriter::new(Vec::new(), STANDARD);

    let error = writer.write_all(b"aGVsbG8=$").await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
}

#[tokio::test]
async fn streaming_decoder_writer_reports_accepted_prefix_before_invalid_quad() {
    let mut writer = DecoderWriter::new(Vec::new(), STANDARD);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let accepted = Pin::new(&mut writer)
        .poll_write(&mut context, b"aGVs$$$$")
        .map(|result| result.unwrap());

    assert_eq!(accepted, Poll::Ready(4));
    assert!(!writer.is_failed());
    assert_eq!(writer.get_ref(), b"");

    let error = writer.write_all(b"$$$$").await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
    assert_eq!(writer.get_ref(), b"hel");
}

#[tokio::test]
async fn streaming_decoder_writer_rejects_incomplete_padded_tail_on_shutdown() {
    let mut writer = DecoderWriter::new(Vec::new(), STANDARD);

    writer.write_all(b"aG").await.unwrap();
    let error = writer.shutdown().await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
}

#[tokio::test]
async fn streaming_decoder_writer_supports_unpadded_tail_on_shutdown() {
    let mut writer = DecoderWriter::new(Vec::new(), URL_SAFE_NO_PAD);

    writer.write_all(b"aGVsbG8").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), b"hello");
}

#[tokio::test]
async fn streaming_encoder_writer_propagates_inner_write_error() {
    let inner = ScriptedWriter::new([WriteAction::Error]);
    let mut writer = EncoderWriter::new(inner, STANDARD);

    writer.write_all(b"hello").await.unwrap();
    let error = writer.shutdown().await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(!writer.is_failed());
}

#[tokio::test]
async fn streaming_encoder_writer_round_trips_large_input_with_one_byte_backpressure() {
    let input: Vec<u8> = (0u8..=250).cycle().take(5000).collect();
    let expected = STANDARD.encode_vec(&input).unwrap();
    let inner = ScriptedWriter::new((0..expected.len()).map(|_| WriteAction::Accept(1)));
    let mut writer = EncoderWriter::new(inner, STANDARD);

    writer.write_all(&input).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner().output(), expected);
}

#[tokio::test]
async fn streaming_decoder_writer_round_trips_large_input_with_one_byte_backpressure() {
    let input: Vec<u8> = (0u8..=250).cycle().take(5000).collect();
    let encoded = STANDARD.encode_vec(&input).unwrap();
    let inner = ScriptedWriter::new((0..input.len()).map(|_| WriteAction::Accept(1)));
    let mut writer = DecoderWriter::new(inner, STANDARD);

    writer.write_all(&encoded).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner().output(), input);
}

#[tokio::test]
async fn streaming_encoder_writer_clamps_single_large_poll_write_to_queue_capacity() {
    let input: Vec<u8> = (0u8..=250).cycle().take(2000).collect();
    let expected = STANDARD.encode_vec(&input).unwrap();
    let mut writer = EncoderWriter::new(Vec::new(), STANDARD);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let accepted = Pin::new(&mut writer)
        .poll_write(&mut context, &input)
        .map(|result| result.unwrap());

    assert_eq!(accepted, Poll::Ready(768));
    assert_eq!(writer.buffered_output_len(), 1024);

    writer.write_all(&input[768..]).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), expected);
}

#[tokio::test]
async fn streaming_decoder_writer_clamps_single_large_poll_write_to_queue_capacity() {
    let input: Vec<u8> = (0u8..=250).cycle().take(2000).collect();
    let encoded = STANDARD.encode_vec(&input).unwrap();
    let mut writer = DecoderWriter::new(Vec::new(), STANDARD);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let accepted = Pin::new(&mut writer)
        .poll_write(&mut context, &encoded)
        .map(|result| result.unwrap());

    assert_eq!(accepted, Poll::Ready(1364));
    assert_eq!(writer.buffered_output_len(), 1023);

    writer.write_all(&encoded[1364..]).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), input);
}
