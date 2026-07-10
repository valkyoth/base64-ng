#![allow(missing_docs)]

use base64_ng::{STANDARD, URL_SAFE_NO_PAD};
use base64_ng_tokio::{
    DecoderReader, EncoderReader, decode_reader_to_writer, decode_reader_to_writer_limited,
    decode_to_vec, encode_reader_to_writer, encode_reader_to_writer_limited, encode_to_vec,
};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc, task::Wake};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

enum ReadAction {
    Data(Vec<u8>),
    Error,
    Pending,
}

struct ScriptedReader {
    actions: VecDeque<ReadAction>,
}

impl ScriptedReader {
    fn new(actions: impl IntoIterator<Item = ReadAction>) -> Self {
        Self {
            actions: actions.into_iter().collect(),
        }
    }

    fn remaining_data_len(&self) -> usize {
        self.actions
            .iter()
            .map(|action| match action {
                ReadAction::Data(bytes) => bytes.len(),
                ReadAction::Error | ReadAction::Pending => 0,
            })
            .sum()
    }
}

impl AsyncRead for ScriptedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        destination: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.actions.pop_front() {
            Some(ReadAction::Data(mut bytes)) => {
                let count = bytes.len().min(destination.remaining());
                destination.put_slice(&bytes[..count]);
                if count != bytes.len() {
                    let remaining = bytes.split_off(count);
                    self.actions.push_front(ReadAction::Data(remaining));
                }
                Poll::Ready(Ok(()))
            }
            Some(ReadAction::Pending) => {
                context.waker().wake_by_ref();
                Poll::Pending
            }
            Some(ReadAction::Error) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "scripted read error",
            ))),
            None => Poll::Ready(Ok(())),
        }
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
async fn encodes_reader_to_writer() {
    let mut input = &b"hello"[..];
    let mut output = Vec::new();

    let written = encode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap();

    assert_eq!(written, 8);
    assert_eq!(output, b"aGVsbG8=");
}

#[tokio::test]
async fn decodes_reader_to_writer() {
    let mut input = &b"aGVsbG8="[..];
    let mut output = Vec::new();

    let written = decode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap();

    assert_eq!(written, 5);
    assert_eq!(output, b"hello");
}

#[tokio::test]
async fn decode_does_not_write_on_malformed_input() {
    let mut input = &b"aGVsbG8=$"[..];
    let mut output = b"untouched".to_vec();

    let error = decode_reader_to_writer(&STANDARD, &mut input, &mut output)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_encode_reports_oversized_input_before_writing() {
    let mut input = &b"hello"[..];
    let mut output = b"untouched".to_vec();

    let error = encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 4)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_decode_reports_oversized_input_before_writing() {
    let mut input = &b"aGVsbG8="[..];
    let mut output = b"untouched".to_vec();

    let error = decode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, 7)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_helpers_consume_at_most_limit_plus_one_byte() {
    let input_len = 16_384;
    let limit = 100;
    let mut input = ScriptedReader::new([ReadAction::Data(vec![b'x'; input_len])]);
    let mut output = b"untouched".to_vec();

    let error = encode_reader_to_writer_limited(&STANDARD, &mut input, &mut output, limit)
        .await
        .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(input.remaining_data_len(), input_len - limit - 1);
    assert_eq!(output, b"untouched");
}

#[tokio::test]
async fn limited_reader_helpers_round_trip_at_limit() {
    let mut input = &b"hello"[..];
    let mut encoded = Vec::new();

    let written = encode_reader_to_writer_limited(&STANDARD, &mut input, &mut encoded, 5)
        .await
        .unwrap();

    assert_eq!(written, 8);
    assert_eq!(encoded, b"aGVsbG8=");

    let mut encoded_input = &encoded[..];
    let mut decoded = Vec::new();
    let written = decode_reader_to_writer_limited(&STANDARD, &mut encoded_input, &mut decoded, 8)
        .await
        .unwrap();

    assert_eq!(written, 5);
    assert_eq!(decoded, b"hello");
}

#[tokio::test]
async fn vec_helpers_round_trip() {
    let encoded = encode_to_vec(&URL_SAFE_NO_PAD, [0xfb, 0xff]).unwrap();
    assert_eq!(encoded, b"-_8");

    let decoded = decode_to_vec(&URL_SAFE_NO_PAD, encoded).unwrap();
    assert_eq!(decoded, [0xfb, 0xff]);
}

#[tokio::test]
async fn streaming_encoder_reader_handles_one_byte_chunks() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"h".to_vec()),
        ReadAction::Data(b"e".to_vec()),
        ReadAction::Data(b"l".to_vec()),
        ReadAction::Data(b"l".to_vec()),
        ReadAction::Data(b"o".to_vec()),
    ]);
    let mut encoder = EncoderReader::new(reader, STANDARD);
    let mut output = Vec::new();

    encoder.read_to_end(&mut output).await.unwrap();

    assert_eq!(output, b"aGVsbG8=");
}

#[tokio::test]
async fn streaming_decoder_reader_handles_split_quanta() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"a".to_vec()),
        ReadAction::Data(b"G".to_vec()),
        ReadAction::Data(b"Vs".to_vec()),
        ReadAction::Data(b"b".to_vec()),
        ReadAction::Data(b"G8=".to_vec()),
    ]);
    let mut decoder = DecoderReader::new(reader, STANDARD);
    let mut output = Vec::new();

    decoder.read_to_end(&mut output).await.unwrap();

    assert_eq!(output, b"hello");
}

#[tokio::test]
async fn streaming_encoder_reader_resumes_after_pending_inside_quantum() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"h".to_vec()),
        ReadAction::Pending,
        ReadAction::Data(b"ello".to_vec()),
    ]);
    let mut encoder = EncoderReader::new(reader, STANDARD);
    let mut first = [0u8; 16];
    let mut first_buf = ReadBuf::new(&mut first);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    assert!(
        Pin::new(&mut encoder)
            .poll_read(&mut context, &mut first_buf)
            .is_pending()
    );
    assert_eq!(first_buf.filled(), b"");

    let mut output = Vec::new();
    encoder.read_to_end(&mut output).await.unwrap();

    assert_eq!(output, b"aGVsbG8=");
}

#[tokio::test]
async fn streaming_decoder_reader_resumes_after_pending_inside_quantum() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"aG".to_vec()),
        ReadAction::Pending,
        ReadAction::Data(b"VsbG8=".to_vec()),
    ]);
    let mut decoder = DecoderReader::new(reader, STANDARD);
    let mut first = [0u8; 16];
    let mut first_buf = ReadBuf::new(&mut first);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    assert!(
        Pin::new(&mut decoder)
            .poll_read(&mut context, &mut first_buf)
            .is_pending()
    );
    assert_eq!(first_buf.filled(), b"");

    let mut output = Vec::new();
    decoder.read_to_end(&mut output).await.unwrap();

    assert_eq!(output, b"hello");
}

#[tokio::test]
async fn streaming_decoder_reader_fails_closed_after_malformed_input() {
    let reader = ScriptedReader::new([ReadAction::Data(b"aGVsbG8=$".to_vec())]);
    let mut decoder = DecoderReader::new(reader, STANDARD);
    let mut output = Vec::new();

    let error = decoder.read_to_end(&mut output).await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(decoder.is_failed());
}

#[tokio::test]
async fn streaming_encoder_reader_clears_and_fails_after_inner_error_with_pending_input() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"h".to_vec()),
        ReadAction::Pending,
        ReadAction::Error,
    ]);
    let mut encoder = EncoderReader::new(reader, STANDARD);
    let mut first = [0u8; 16];
    let mut first_buf = ReadBuf::new(&mut first);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    assert!(
        Pin::new(&mut encoder)
            .poll_read(&mut context, &mut first_buf)
            .is_pending()
    );
    assert_eq!(first_buf.filled(), b"");

    let mut output = Vec::new();
    let error = encoder.read_to_end(&mut output).await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(encoder.is_failed());
    assert_eq!(output, b"");

    let error = encoder.read_to_end(&mut output).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
}

#[tokio::test]
async fn streaming_decoder_reader_clears_and_fails_after_inner_error_with_pending_input() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"aG".to_vec()),
        ReadAction::Pending,
        ReadAction::Error,
    ]);
    let mut decoder = DecoderReader::new(reader, STANDARD);
    let mut first = [0u8; 16];
    let mut first_buf = ReadBuf::new(&mut first);
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    assert!(
        Pin::new(&mut decoder)
            .poll_read(&mut context, &mut first_buf)
            .is_pending()
    );
    assert_eq!(first_buf.filled(), b"");

    let mut output = Vec::new();
    let error = decoder.read_to_end(&mut output).await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(decoder.is_failed());
    assert_eq!(output, b"");

    let error = decoder.read_to_end(&mut output).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
}
