#![allow(missing_docs)]

use base64_ng::{Base64, Codec, STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED};
use base64_ng_tokio::{DecoderReader, EncoderReader};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc, task::Wake};
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

enum ReadAction {
    Data(Vec<u8>),
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
                ReadAction::Pending => 0,
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
                    self.actions
                        .push_front(ReadAction::Data(bytes.split_off(count)));
                }
                Poll::Ready(Ok(()))
            }
            Some(ReadAction::Pending) => {
                context.waker().wake_by_ref();
                Poll::Pending
            }
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

fn scheduled_actions(input: &[u8], schedule: &[usize]) -> Vec<ReadAction> {
    let mut actions = vec![ReadAction::Pending];
    let mut offset = 0;
    let mut index = 0;
    while offset < input.len() {
        let count = schedule[index % schedule.len()].min(input.len() - offset);
        actions.push(ReadAction::Data(input[offset..offset + count].to_vec()));
        actions.push(ReadAction::Pending);
        offset += count;
        index += 1;
    }
    actions
}

async fn read_one_byte_at_a_time<R: AsyncRead + Unpin>(reader: &mut R) -> Vec<u8> {
    let mut output = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        let read = reader.read(&mut byte).await.unwrap();
        if read == 0 {
            return output;
        }
        output.push(byte[0]);
    }
}

async fn assert_schedule_matches_one_shot<S: Codec + Copy>(
    codec: Base64<S>,
    input: &[u8],
    schedule: &[usize],
) {
    let expected = codec.encode_to_string(input).unwrap().into_bytes();
    let reader = ScriptedReader::new(scheduled_actions(input, schedule));
    let mut encode_reader = EncoderReader::new(reader, &codec);
    let actual = read_one_byte_at_a_time(&mut encode_reader).await;
    assert_eq!(actual, expected);
    assert_eq!(encode_reader.input_read(), input.len());
    assert_eq!(encode_reader.source_position(), input.len());
    assert_eq!(encode_reader.output_delivered(), expected.len());
    assert!(encode_reader.is_complete());

    let reader = ScriptedReader::new(scheduled_actions(&expected, schedule));
    let mut decode_reader = DecoderReader::new(reader, &codec);
    let actual = read_one_byte_at_a_time(&mut decode_reader).await;
    assert_eq!(actual, input);
    assert_eq!(decode_reader.input_read(), expected.len());
    assert_eq!(decode_reader.source_position(), expected.len());
    assert_eq!(decode_reader.output_delivered(), input.len());
    assert!(decode_reader.is_complete());
}

#[tokio::test]
async fn exact_readers_stop_without_consuming_adjacent_frames() {
    let mut raw_source = ScriptedReader::new([ReadAction::Data(b"helloNEXT".to_vec())]);
    {
        let mut encoder = EncoderReader::new_exact(&mut raw_source, &STRICT_STANDARD_PADDED, 5);
        assert_eq!(read_one_byte_at_a_time(&mut encoder).await, b"aGVsbG8=");
        assert_eq!(encoder.remaining_input(), Some(0));
        assert_eq!(encoder.input_read(), 5);
        assert!(encoder.is_complete());
    }
    assert_eq!(raw_source.remaining_data_len(), 4);

    let mut source = ScriptedReader::new([ReadAction::Data(b"aGVsbG8=NEXT".to_vec())]);
    {
        let mut decoder = DecoderReader::new_exact(&mut source, &STRICT_STANDARD_PADDED, 8);
        assert_eq!(read_one_byte_at_a_time(&mut decoder).await, b"hello");
        assert_eq!(decoder.remaining_input(), Some(0));
        assert_eq!(decoder.input_read(), 8);
        assert!(decoder.is_complete());
    }
    assert_eq!(source.remaining_data_len(), 4);
}

#[tokio::test]
async fn exact_readers_fail_on_premature_eof() {
    let mut encoder = EncoderReader::new_exact(&b"hi"[..], &STRICT_STANDARD_PADDED, 3);
    let mut output = Vec::new();
    let error = encoder.read_to_end(&mut output).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    assert!(encoder.is_failed());
    assert_eq!(encoder.input_read(), 2);
    assert_eq!(encoder.source_position(), 2);

    let mut decoder = DecoderReader::new_exact(&b"aGk"[..], &STRICT_STANDARD_PADDED, 4);
    let error = decoder.read_to_end(&mut output).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
    assert!(decoder.is_failed());
    assert_eq!(decoder.input_read(), 3);
    assert_eq!(decoder.source_position(), 3);
}

#[tokio::test]
async fn dropped_pending_read_futures_resume_without_loss_or_duplication() {
    let source = ScriptedReader::new([
        ReadAction::Data(b"h".to_vec()),
        ReadAction::Pending,
        ReadAction::Data(b"ello".to_vec()),
    ]);
    let mut encoder = EncoderReader::new(source, &STRICT_STANDARD_PADDED);
    let mut first = [0u8; 8];
    let mut future = Box::pin(encoder.read(&mut first));
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    assert!(Future::poll(future.as_mut(), &mut context).is_pending());
    drop(future);
    assert_eq!(read_one_byte_at_a_time(&mut encoder).await, b"aGVsbG8=");

    let source = ScriptedReader::new([
        ReadAction::Data(b"aG".to_vec()),
        ReadAction::Pending,
        ReadAction::Data(b"VsbG8=".to_vec()),
    ]);
    let mut decoder = DecoderReader::new(source, &STRICT_STANDARD_PADDED);
    let mut first = [0u8; 8];
    let mut future = Box::pin(decoder.read(&mut first));
    assert!(Future::poll(future.as_mut(), &mut context).is_pending());
    drop(future);
    assert_eq!(read_one_byte_at_a_time(&mut decoder).await, b"hello");
}

#[tokio::test]
async fn decoder_keeps_delivered_prefix_irrevocable_on_later_error() {
    let reader = ScriptedReader::new([
        ReadAction::Data(b"aGVs".to_vec()),
        ReadAction::Pending,
        ReadAction::Data(b"$".to_vec()),
    ]);
    let mut decoder = DecoderReader::new(reader, &STRICT_STANDARD_PADDED);
    let mut first = [0u8; 3];
    let read = decoder.read(&mut first).await.unwrap();
    assert_eq!(&first[..read], b"hel");

    let mut suffix = Vec::new();
    let error = decoder.read_to_end(&mut suffix).await.unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(first, *b"hel");
    assert!(suffix.is_empty());
    assert_eq!(decoder.input_read(), 5);
    assert_eq!(decoder.source_position(), 4);
    assert_eq!(decoder.output_delivered(), 3);
    assert!(decoder.is_failed());
}

#[tokio::test]
async fn arbitrary_chunk_and_pending_schedules_match_one_shot() {
    let input: Vec<u8> = (0..2_057)
        .map(|index| u8::try_from((index * 29 + 7) % 251).unwrap())
        .collect();
    for schedule in [&[1][..], &[2, 3, 5][..], &[17, 1, 64, 7][..], &[769, 4][..]] {
        assert_schedule_matches_one_shot(STRICT_STANDARD_PADDED, &input, schedule).await;
        assert_schedule_matches_one_shot(STRICT_URL_SAFE_UNPADDED, &input, schedule).await;
    }
}
