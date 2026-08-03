#![allow(missing_docs)]

use base64_ng::{STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};
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
    PanicAfterAccept(usize),
    Pending,
}

struct ScriptedWriter {
    actions: VecDeque<WriteAction>,
    output: Vec<u8>,
    shutdown: bool,
    panic_flush: bool,
    panic_shutdown: bool,
}

impl ScriptedWriter {
    fn new(actions: impl IntoIterator<Item = WriteAction>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
            output: Vec::new(),
            shutdown: false,
            panic_flush: false,
            panic_shutdown: false,
        }
    }

    fn with_flush_panic(mut self) -> Self {
        self.panic_flush = true;
        self
    }

    fn with_shutdown_panic(mut self) -> Self {
        self.panic_shutdown = true;
        self
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
            Some(WriteAction::PanicAfterAccept(limit)) => {
                let count = limit.min(input.len());
                self.output.extend_from_slice(&input[..count]);
                std::panic::panic_any("injected writer panic after accept");
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
        if self.panic_flush {
            std::panic::panic_any("injected writer flush panic");
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.panic_shutdown {
            std::panic::panic_any("injected writer shutdown panic");
        }
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

#[test]
fn wrapped_writer_panics_latch_clear_and_resume_the_original_panic() {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let inner = ScriptedWriter::new([WriteAction::PanicAfterAccept(2)]);
    let mut encoder = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    assert!(matches!(
        Pin::new(&mut encoder).poll_write(&mut context, b"secret"),
        Poll::Ready(Ok(6))
    ));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Pin::new(&mut encoder).poll_flush(&mut context);
    }));
    assert!(panic.is_err());
    assert!(encoder.is_failed());
    assert_eq!(encoder.pending_len(), 0);
    assert_eq!(encoder.buffered_output_len(), 0);
    assert!(matches!(
        Pin::new(&mut encoder).poll_write(&mut context, b"retry"),
        Poll::Ready(Err(_))
    ));
    assert_eq!(encoder.get_ref().output(), b"c2");

    let base64_input = STRICT_STANDARD_PADDED.encode_to_string(b"secret").unwrap();
    let inner = ScriptedWriter::new([WriteAction::PanicAfterAccept(2)]);
    let mut decoder = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    assert!(matches!(
        Pin::new(&mut decoder).poll_write(&mut context, base64_input.as_bytes()),
        Poll::Ready(Ok(8))
    ));
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Pin::new(&mut decoder).poll_flush(&mut context);
    }));
    assert!(panic.is_err());
    assert!(decoder.is_failed());
    assert_eq!(decoder.pending_len(), 0);
    assert_eq!(decoder.buffered_output_len(), 0);
    assert_eq!(decoder.get_ref().output(), b"se");
}

#[test]
fn wrapped_flush_and_shutdown_panics_also_latch_failure() {
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let inner = ScriptedWriter::new([]).with_flush_panic();
    let mut encoder = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Pin::new(&mut encoder).poll_flush(&mut context);
    }));
    assert!(panic.is_err());
    assert!(encoder.is_failed());

    let inner = ScriptedWriter::new([]).with_shutdown_panic();
    let mut decoder = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = Pin::new(&mut decoder).poll_shutdown(&mut context);
    }));
    assert!(panic.is_err());
    assert!(decoder.is_failed());
}

#[tokio::test]
async fn streaming_encoder_writer_handles_split_writes() {
    let mut writer = EncoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);

    writer.write_all(b"he").await.unwrap();
    writer.write_all(b"llo").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), b"aGVsbG8=");
}

#[tokio::test]
async fn streaming_decoder_writer_handles_split_quanta() {
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);

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
    let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
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
    let mut writer = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);
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
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);

    let error = writer.write_all(b"aGVsbG8=$").await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
}

#[tokio::test]
async fn streaming_decoder_writer_rejects_malformed_poll_without_accepting_prefix() {
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    let error = Pin::new(&mut writer)
        .poll_write(&mut context, b"aGVs$$$$")
        .map(|result| result.unwrap_err());

    assert_eq!(
        error.map(|error| error.kind()),
        Poll::Ready(std::io::ErrorKind::InvalidData)
    );
    assert!(writer.is_failed());
    assert_eq!(writer.input_accepted(), 0);
    assert_eq!(writer.get_ref(), b"");
}

#[tokio::test]
async fn streaming_decoder_writer_rejects_incomplete_padded_tail_on_shutdown() {
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);

    writer.write_all(b"aG").await.unwrap();
    let error = writer.shutdown().await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
}

#[tokio::test]
async fn streaming_decoder_writer_supports_unpadded_tail_on_shutdown() {
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_URL_SAFE_UNPADDED);

    writer.write_all(b"aGVsbG8").await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner(), b"hello");
}

#[tokio::test]
async fn streaming_encoder_writer_propagates_inner_write_error() {
    let inner = ScriptedWriter::new([WriteAction::Error]);
    let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);

    writer.write_all(b"hello").await.unwrap();
    let error = writer.shutdown().await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(!writer.is_failed());
}

#[tokio::test]
async fn streaming_encoder_writer_round_trips_large_input_with_one_byte_backpressure() {
    let input: Vec<u8> = (0u8..=250).cycle().take(5000).collect();
    let expected = STRICT_STANDARD_PADDED
        .encode_to_string(&input)
        .unwrap()
        .into_bytes();
    let inner = ScriptedWriter::new((0..expected.len()).map(|_| WriteAction::Accept(1)));
    let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);

    writer.write_all(&input).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner().output(), expected);
}

#[tokio::test]
async fn streaming_decoder_writer_round_trips_large_input_with_one_byte_backpressure() {
    let input: Vec<u8> = (0u8..=250).cycle().take(5000).collect();
    let encoded = STRICT_STANDARD_PADDED
        .encode_to_string(&input)
        .unwrap()
        .into_bytes();
    let inner = ScriptedWriter::new((0..input.len()).map(|_| WriteAction::Accept(1)));
    let mut writer = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);

    writer.write_all(&encoded).await.unwrap();
    writer.shutdown().await.unwrap();

    assert_eq!(writer.into_inner().output(), input);
}

#[tokio::test]
async fn streaming_encoder_writer_clamps_single_large_poll_write_to_queue_capacity() {
    let input: Vec<u8> = (0u8..=250).cycle().take(2000).collect();
    let expected = STRICT_STANDARD_PADDED
        .encode_to_string(&input)
        .unwrap()
        .into_bytes();
    let mut writer = EncoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
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
    let encoded = STRICT_STANDARD_PADDED
        .encode_to_string(&input)
        .unwrap()
        .into_bytes();
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
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
