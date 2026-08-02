#![allow(missing_docs)]

use base64_ng::{Base64, Codec, STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};
use base64_ng_tokio::{DecoderWriter, EncoderWriter};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc, task::Wake};
use tokio::io::{AsyncWrite, AsyncWriteExt};

enum WriteAction {
    Accept(usize),
    Pending,
    Error,
    Zero,
    Overreport,
}

enum UnitAction {
    Ready,
    Pending,
    Error,
}

struct AdversarialWriter {
    writes: VecDeque<WriteAction>,
    flushes: VecDeque<UnitAction>,
    shutdowns: VecDeque<UnitAction>,
    output: Vec<u8>,
}

impl AdversarialWriter {
    fn new(writes: impl IntoIterator<Item = WriteAction>) -> Self {
        Self {
            writes: writes.into_iter().collect(),
            flushes: VecDeque::new(),
            shutdowns: VecDeque::new(),
            output: Vec::new(),
        }
    }

    fn with_flushes(mut self, actions: impl IntoIterator<Item = UnitAction>) -> Self {
        self.flushes = actions.into_iter().collect();
        self
    }

    fn with_shutdowns(mut self, actions: impl IntoIterator<Item = UnitAction>) -> Self {
        self.shutdowns = actions.into_iter().collect();
        self
    }

    fn output(&self) -> &[u8] {
        &self.output
    }
}

impl AsyncWrite for AdversarialWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.writes.pop_front() {
            Some(WriteAction::Accept(limit)) => {
                let accepted = limit.min(input.len());
                self.output.extend_from_slice(&input[..accepted]);
                Poll::Ready(Ok(accepted))
            }
            Some(WriteAction::Pending) => {
                context.waker().wake_by_ref();
                Poll::Pending
            }
            Some(WriteAction::Error) => Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "adversarial write error",
            ))),
            Some(WriteAction::Zero) => Poll::Ready(Ok(0)),
            Some(WriteAction::Overreport) => Poll::Ready(Ok(input.len() + 1)),
            None => {
                self.output.extend_from_slice(input);
                Poll::Ready(Ok(input.len()))
            }
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_unit(&mut self.flushes, context, "adversarial flush error")
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        poll_unit(&mut self.shutdowns, context, "adversarial shutdown error")
    }
}

fn poll_unit(
    actions: &mut VecDeque<UnitAction>,
    context: &mut Context<'_>,
    message: &'static str,
) -> Poll<std::io::Result<()>> {
    match actions.pop_front() {
        Some(UnitAction::Pending) => {
            context.waker().wake_by_ref();
            Poll::Pending
        }
        Some(UnitAction::Error) => Poll::Ready(Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            message,
        ))),
        Some(UnitAction::Ready) | None => Poll::Ready(Ok(())),
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

fn payload(seed: usize) -> Vec<u8> {
    let len = (seed * 137 + 19) % 2300;
    (0..len)
        .map(|index| u8::try_from((index * 73 + seed * 29 + 11) % 256).unwrap())
        .collect()
}

fn write_schedule(seed: usize, output_len: usize) -> Vec<WriteAction> {
    let mut actions = Vec::with_capacity(output_len.saturating_mul(2));
    for index in 0..output_len.saturating_add(8) {
        if (index + seed).is_multiple_of(3) {
            actions.push(WriteAction::Pending);
        }
        actions.push(WriteAction::Accept((index * 5 + seed) % 11 + 1));
    }
    actions
}

async fn write_partitioned<W: AsyncWrite + Unpin>(writer: &mut W, input: &[u8], seed: usize) {
    let mut offset = 0;
    while offset < input.len() {
        let width = ((offset * 17 + seed * 7) % 43 + 1).min(input.len() - offset);
        writer
            .write_all(&input[offset..offset + width])
            .await
            .unwrap();
        offset += width;
    }
}

async fn assert_encoder_schedule<S: Codec>(codec: &Base64<S>, seed: usize) {
    let input = payload(seed);
    let expected = codec.encode_to_string(&input).unwrap().into_bytes();
    let inner = AdversarialWriter::new(write_schedule(seed, expected.len()));
    let mut writer = EncoderWriter::new(inner, codec);
    write_partitioned(&mut writer, &input, seed).await;
    writer.shutdown().await.unwrap();
    assert_eq!(writer.input_accepted(), input.len());
    assert_eq!(writer.output_committed(), expected.len());
    assert!(writer.is_finalized());
    assert!(writer.is_shutdown());
    assert_eq!(writer.into_inner().output(), expected);
}

async fn assert_decoder_schedule<S: Codec>(codec: &Base64<S>, seed: usize) {
    let expected = payload(seed);
    let encoded = codec.encode_to_string(&expected).unwrap().into_bytes();
    let inner = AdversarialWriter::new(write_schedule(seed + 5, expected.len()));
    let mut writer = DecoderWriter::new(inner, codec);
    write_partitioned(&mut writer, &encoded, seed + 3).await;
    writer.shutdown().await.unwrap();
    assert_eq!(writer.input_accepted(), encoded.len());
    assert_eq!(writer.output_committed(), expected.len());
    assert!(writer.is_finalized());
    assert!(writer.is_shutdown());
    assert_eq!(writer.into_inner().output(), expected);
}

#[tokio::test]
async fn arbitrary_partitions_and_backpressure_match_one_shot() {
    for seed in 0..24 {
        assert_encoder_schedule(&STRICT_STANDARD_PADDED, seed).await;
        assert_decoder_schedule(&STRICT_STANDARD_PADDED, seed).await;
        assert_encoder_schedule(&STRICT_URL_SAFE_UNPADDED, seed + 31).await;
        assert_decoder_schedule(&STRICT_URL_SAFE_UNPADDED, seed + 31).await;
    }
}

#[tokio::test]
async fn dropped_pending_write_futures_resume_without_loss_or_duplication() {
    let inner = AdversarialWriter::new([WriteAction::Pending, WriteAction::Accept(usize::MAX)]);
    let mut encoder = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    assert_eq!(encoder.write(b"abc").await.unwrap(), 3);

    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut pending = Box::pin(encoder.write(b"def"));
    assert!(Future::poll(pending.as_mut(), &mut context).is_pending());
    drop(pending);

    encoder.write_all(b"def").await.unwrap();
    encoder.shutdown().await.unwrap();
    assert_eq!(encoder.into_inner().output(), b"YWJjZGVm");

    let inner = AdversarialWriter::new([WriteAction::Pending, WriteAction::Accept(usize::MAX)]);
    let mut decoder = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    assert_eq!(decoder.write(b"YWJj").await.unwrap(), 4);

    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut pending = Box::pin(decoder.write(b"ZGVm"));
    assert!(Future::poll(pending.as_mut(), &mut context).is_pending());
    drop(pending);

    decoder.write_all(b"ZGVm").await.unwrap();
    decoder.shutdown().await.unwrap();
    assert_eq!(decoder.into_inner().output(), b"abcdef");
}

#[tokio::test]
async fn write_zero_and_inner_error_retain_output_for_retry() {
    for first in [WriteAction::Zero, WriteAction::Error] {
        let inner = AdversarialWriter::new([first, WriteAction::Accept(usize::MAX)]);
        let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
        writer.write_all(b"hello").await.unwrap();
        assert!(writer.flush().await.is_err());
        assert!(!writer.is_failed());
        assert_ne!(writer.buffered_output_len(), 0);
        writer.flush().await.unwrap();
        writer.shutdown().await.unwrap();
        assert_eq!(writer.into_inner().output(), b"aGVsbG8=");
    }
}

#[tokio::test]
async fn flush_and_shutdown_failures_retry_without_duplicate_output() {
    let inner = AdversarialWriter::new([])
        .with_flushes([UnitAction::Error, UnitAction::Ready])
        .with_shutdowns([UnitAction::Error, UnitAction::Ready]);
    let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    writer.write_all(b"hello").await.unwrap();
    assert!(writer.flush().await.is_err());
    writer.flush().await.unwrap();
    assert!(writer.shutdown().await.is_err());
    assert!(writer.is_finalized());
    assert!(!writer.is_shutdown());
    writer.shutdown().await.unwrap();
    assert_eq!(writer.into_inner().output(), b"aGVsbG8=");
}

#[tokio::test]
async fn dropped_pending_shutdown_resumes_without_refinalizing() {
    let inner = AdversarialWriter::new([]).with_shutdowns([UnitAction::Pending, UnitAction::Ready]);
    let mut writer = EncoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    writer.write_all(b"hello").await.unwrap();

    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut pending = Box::pin(writer.shutdown());
    assert!(Future::poll(pending.as_mut(), &mut context).is_pending());
    drop(pending);

    assert!(writer.is_finalized());
    assert!(!writer.is_shutdown());
    writer.shutdown().await.unwrap();
    assert_eq!(writer.into_inner().output(), b"aGVsbG8=");
}

#[tokio::test]
async fn decoder_preserves_only_plaintext_committed_before_later_failure() {
    let mut writer = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
    writer.write_all(b"aGVs").await.unwrap();
    writer.flush().await.unwrap();
    assert_eq!(writer.get_ref(), b"hel");

    let error = writer.write_all(b"$$$$").await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
    assert_eq!(writer.input_accepted(), 4);
    assert_eq!(writer.output_committed(), 3);
    assert_eq!(writer.buffered_output_len(), 0);
    assert_eq!(writer.pending_len(), 0);
    assert_eq!(writer.get_ref(), b"hel");
}

#[tokio::test]
async fn wrapped_writer_overreport_latches_and_clears_state() {
    let inner = AdversarialWriter::new([WriteAction::Overreport]);
    let mut writer = DecoderWriter::new(inner, &STRICT_STANDARD_PADDED);
    writer.write_all(b"aGVs").await.unwrap();
    let error = writer.flush().await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(writer.is_failed());
    assert_eq!(writer.pending_len(), 0);
    assert_eq!(writer.buffered_output_len(), 0);
}

#[tokio::test]
async fn checked_inner_recovery_rejects_retained_input_and_output() {
    let mut encoder = EncoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
    encoder.write_all(b"a").await.unwrap();
    let mut encoder = encoder.try_into_inner().unwrap_err();
    encoder.write_all(b"bc").await.unwrap();
    encoder.flush().await.unwrap();
    assert!(encoder.can_into_inner());
    let Ok(inner) = encoder.try_into_inner() else {
        panic!("encoder retained input after complete quantum was flushed");
    };
    assert_eq!(inner, b"YWJj");

    let mut decoder = DecoderWriter::new(Vec::new(), &STRICT_STANDARD_PADDED);
    decoder.write_all(b"YQ").await.unwrap();
    let decoder = decoder.try_into_inner().unwrap_err();
    assert_eq!(decoder.pending_len(), 2);
}
