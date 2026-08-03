#![no_main]

use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_tokio::{DecoderReader, DecoderWriter, EncoderReader, EncoderWriter};
use libfuzzer_sys::fuzz_target;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

const MAX_INPUT: usize = 4096;
const MAX_POLLS: usize = 100_000;

fuzz_target!(|data: &[u8]| {
    let maximum = data.first().map_or(1, |byte| usize::from(*byte) % 31 + 1);
    let starts_pending = data.get(1).is_some_and(|byte| byte & 1 != 0);
    let input = &data[data.len().min(2)..data.len().min(2 + MAX_INPUT)];
    let encoded = STRICT_STANDARD_PADDED.encode_to_string(input).unwrap();

    let writer_encoded = drive_encoder_writer(input, maximum, starts_pending);
    assert_eq!(writer_encoded, encoded.as_bytes());
    let writer_decoded = drive_decoder_writer(encoded.as_bytes(), maximum, starts_pending);
    assert_eq!(writer_decoded, input);

    let reader_encoded = drive_encoder_reader(input, maximum, starts_pending);
    assert_eq!(reader_encoded, encoded.as_bytes());
    let reader_decoded = drive_decoder_reader(encoded.as_bytes(), maximum, starts_pending);
    assert_eq!(reader_decoded, input);

    exercise_malformed_decoder_writer(input, maximum, starts_pending);
    exercise_cancellation_drop(input, maximum, starts_pending);
});

fn drive_encoder_writer(input: &[u8], maximum: usize, pending: bool) -> Vec<u8> {
    let mut writer = EncoderWriter::new(
        ScriptedWriter::new(maximum, pending),
        &STRICT_STANDARD_PADDED,
    );
    drive_all_writes(&mut writer, input);
    drive_shutdown(&mut writer);
    assert!(writer.is_finalized());
    assert!(writer.is_shutdown());
    assert_eq!(writer.input_accepted(), input.len());
    writer.into_inner().output
}

fn drive_decoder_writer(input: &[u8], maximum: usize, pending: bool) -> Vec<u8> {
    let mut writer = DecoderWriter::new(
        ScriptedWriter::new(maximum, pending),
        &STRICT_STANDARD_PADDED,
    );
    drive_all_writes(&mut writer, input);
    drive_shutdown(&mut writer);
    assert!(writer.is_finalized());
    assert!(writer.is_shutdown());
    assert_eq!(writer.input_accepted(), input.len());
    writer.into_inner().output
}

fn drive_all_writes<W: AsyncWrite + Unpin>(writer: &mut W, input: &[u8]) {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut offset = 0;
    let mut polls = 0;
    while offset < input.len() {
        polls += 1;
        assert!(polls <= MAX_POLLS);
        match Pin::new(&mut *writer).poll_write(&mut context, &input[offset..]) {
            Poll::Pending => {}
            Poll::Ready(Ok(accepted)) => {
                assert!(accepted != 0 && accepted <= input.len() - offset);
                offset += accepted;
            }
            Poll::Ready(Err(error)) => panic!("valid async write failed: {error}"),
        }
    }
}

fn drive_shutdown<W: AsyncWrite + Unpin>(writer: &mut W) {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    for _ in 0..MAX_POLLS {
        match Pin::new(&mut *writer).poll_shutdown(&mut context) {
            Poll::Pending => {}
            Poll::Ready(Ok(())) => return,
            Poll::Ready(Err(error)) => panic!("valid async shutdown failed: {error}"),
        }
    }
    panic!("async shutdown exceeded finite poll budget");
}

fn drive_encoder_reader(input: &[u8], maximum: usize, pending: bool) -> Vec<u8> {
    let source = ScriptedReader::new(input, maximum, pending);
    let mut reader = EncoderReader::new(source, &STRICT_STANDARD_PADDED);
    let output = drive_reader(&mut reader, maximum);
    assert!(reader.is_complete());
    assert_eq!(reader.input_read(), input.len());
    assert_eq!(reader.output_delivered(), output.len());
    output
}

fn drive_decoder_reader(input: &[u8], maximum: usize, pending: bool) -> Vec<u8> {
    let source = ScriptedReader::new(input, maximum, pending);
    let mut reader = DecoderReader::new(source, &STRICT_STANDARD_PADDED);
    let output = drive_reader(&mut reader, maximum);
    assert!(reader.is_complete());
    assert_eq!(reader.input_read(), input.len());
    assert_eq!(reader.output_delivered(), output.len());
    output
}

fn drive_reader<R: AsyncRead + Unpin>(reader: &mut R, maximum: usize) -> Vec<u8> {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut output = Vec::new();
    for _ in 0..MAX_POLLS {
        let mut storage = [0u8; 37];
        let capacity = maximum.min(storage.len()).max(1);
        let mut buffer = ReadBuf::new(&mut storage[..capacity]);
        match Pin::new(&mut *reader).poll_read(&mut context, &mut buffer) {
            Poll::Pending => {}
            Poll::Ready(Ok(())) if buffer.filled().is_empty() => return output,
            Poll::Ready(Ok(())) => output.extend_from_slice(buffer.filled()),
            Poll::Ready(Err(error)) => panic!("valid async read failed: {error}"),
        }
    }
    panic!("async reader exceeded finite poll budget");
}

fn exercise_malformed_decoder_writer(input: &[u8], maximum: usize, pending: bool) {
    let mut writer = DecoderWriter::new(
        ScriptedWriter::new(maximum, pending),
        &STRICT_STANDARD_PADDED,
    );
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    for _ in 0..MAX_POLLS.min(input.len().saturating_mul(4).saturating_add(16)) {
        match Pin::new(&mut writer).poll_write(&mut context, input) {
            Poll::Pending => {}
            Poll::Ready(Ok(_)) => return,
            Poll::Ready(Err(_)) => {
                assert!(writer.is_failed());
                assert!(matches!(
                    Pin::new(&mut writer).poll_write(&mut context, b"AAAA"),
                    Poll::Ready(Err(_))
                ));
                return;
            }
        }
    }
}

fn exercise_cancellation_drop(input: &[u8], maximum: usize, pending: bool) {
    let mut writer = EncoderWriter::new(
        ScriptedWriter::new(maximum, pending),
        &STRICT_STANDARD_PADDED,
    );
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    for _ in 0..8 {
        if !matches!(
            Pin::new(&mut writer).poll_write(&mut context, input),
            Poll::Pending
        ) {
            break;
        }
    }
    drop(writer);
}

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

struct ScriptedWriter {
    output: Vec<u8>,
    maximum: usize,
    pending_next: bool,
}

impl ScriptedWriter {
    fn new(maximum: usize, pending_next: bool) -> Self {
        Self {
            output: Vec::new(),
            maximum,
            pending_next,
        }
    }
}

impl AsyncWrite for ScriptedWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.pending_next {
            self.pending_next = false;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending_next = true;
        let accepted = input.len().min(self.maximum);
        self.output.extend_from_slice(&input[..accepted]);
        Poll::Ready(Ok(accepted))
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

struct ScriptedReader {
    input: Vec<u8>,
    position: usize,
    maximum: usize,
    pending_next: bool,
}

impl ScriptedReader {
    fn new(input: &[u8], maximum: usize, pending_next: bool) -> Self {
        Self {
            input: input.to_vec(),
            position: 0,
            maximum,
            pending_next,
        }
    }
}

impl AsyncRead for ScriptedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pending_next {
            self.pending_next = false;
            context.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending_next = true;
        let count = (self.input.len() - self.position)
            .min(self.maximum)
            .min(output.remaining());
        let end = self.position + count;
        output.put_slice(&self.input[self.position..end]);
        self.position = end;
        Poll::Ready(Ok(()))
    }
}
