#![allow(missing_docs)]

use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_tokio::{decode_reader_to_writer_limited, encode_reader_to_writer_limited};
use core::{
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
};
use std::sync::Arc;
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    task::JoinHandle,
};

struct OneByteReader {
    byte: u8,
    remaining: usize,
}

impl AsyncRead for OneByteReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.remaining != 0 && output.remaining() != 0 {
            output.put_slice(&[self.byte]);
            self.remaining -= 1;
        }
        Poll::Ready(Ok(()))
    }
}

#[derive(Default)]
struct OneByteWriter {
    output: Vec<u8>,
}

impl AsyncWrite for OneByteWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if let Some(byte) = input.first() {
            self.output.push(*byte);
            Poll::Ready(Ok(1))
        } else {
            Poll::Ready(Ok(0))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn spawn_ready_observer() -> (Arc<AtomicBool>, JoinHandle<()>) {
    let ran = Arc::new(AtomicBool::new(false));
    let observer = Arc::clone(&ran);
    let task = tokio::spawn(async move {
        observer.store(true, Ordering::SeqCst);
    });
    (ran, task)
}

async fn assert_observer_ran(ran: &AtomicBool, task: JoinHandle<()>) {
    assert!(ran.load(Ordering::SeqCst));
    task.await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn always_ready_one_byte_collection_remains_cooperative() {
    let (ran, task) = spawn_ready_observer();
    let mut reader = OneByteReader {
        byte: b'a',
        remaining: 512,
    };
    let mut output = Vec::new();

    encode_reader_to_writer_limited(&STRICT_STANDARD_PADDED, &mut reader, &mut output, 512)
        .await
        .unwrap();

    assert_observer_ran(&ran, task).await;
    assert_eq!(output.len(), 684);
}

#[tokio::test(flavor = "current_thread")]
async fn large_incremental_transforms_remain_cooperative() {
    let input = vec![0x5a; 96_000];
    let mut encoded = Vec::new();
    let (ran, task) = spawn_ready_observer();

    encode_reader_to_writer_limited(
        &STRICT_STANDARD_PADDED,
        &mut &input[..],
        &mut encoded,
        input.len(),
    )
    .await
    .unwrap();

    assert_observer_ran(&ran, task).await;

    let mut decoded = Vec::new();
    let (ran, task) = spawn_ready_observer();
    decode_reader_to_writer_limited(
        &STRICT_STANDARD_PADDED,
        &mut &encoded[..],
        &mut decoded,
        encoded.len(),
    )
    .await
    .unwrap();

    assert_observer_ran(&ran, task).await;
    assert_eq!(decoded, input);
}

#[tokio::test(flavor = "current_thread")]
async fn always_ready_one_byte_output_remains_cooperative() {
    let input = [0x36; 128];
    let mut writer = OneByteWriter::default();
    let (ran, task) = spawn_ready_observer();

    encode_reader_to_writer_limited(
        &STRICT_STANDARD_PADDED,
        &mut &input[..],
        &mut writer,
        input.len(),
    )
    .await
    .unwrap();

    assert_observer_ran(&ran, task).await;
    assert_eq!(writer.output.len(), 172);
}
