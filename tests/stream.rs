#![cfg(feature = "stream")]

use base64_ng::stream::{Decoder, DecoderReader, Encoder, EncoderReader};
use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use std::io::{Cursor, Read, Write};

#[cfg(feature = "stream")]
struct ChunkedReader<'a> {
    input: &'a [u8],
    max_chunk: usize,
}

#[cfg(feature = "stream")]
impl Read for ChunkedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.input.len().min(self.max_chunk).min(output.len());
        if len == 0 {
            return Ok(0);
        }

        output[..len].copy_from_slice(&self.input[..len]);
        self.input = &self.input[len..];
        Ok(len)
    }
}

#[cfg(feature = "stream")]
struct FramedChunkedReader<'a> {
    input: &'a [u8],
    max_chunk: usize,
}

#[cfg(feature = "stream")]
impl<'a> FramedChunkedReader<'a> {
    fn remaining(&self) -> &'a [u8] {
        self.input
    }
}

#[cfg(feature = "stream")]
impl Read for FramedChunkedReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.input.len().min(self.max_chunk).min(output.len());
        if len == 0 {
            return Ok(0);
        }

        output[..len].copy_from_slice(&self.input[..len]);
        self.input = &self.input[len..];
        Ok(len)
    }
}

#[cfg(feature = "stream")]
struct PoisoningReadError<'a> {
    poison: &'a [u8],
    kind: std::io::ErrorKind,
}

#[cfg(feature = "stream")]
impl Read for PoisoningReadError<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let len = self.poison.len().min(output.len());
        output[..len].copy_from_slice(&self.poison[..len]);
        Err(std::io::Error::new(self.kind, "injected read failure"))
    }
}

#[cfg(feature = "stream")]
#[derive(Default)]
struct FailOnceWriter {
    output: Vec<u8>,
    fail_next: bool,
    fail_flush_next: bool,
}

#[cfg(feature = "stream")]
impl Write for FailOnceWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        if self.fail_next {
            self.fail_next = false;
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "injected writer failure",
            ));
        }

        self.output.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush_next {
            self.fail_flush_next = false;
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "injected flush failure",
            ));
        }

        Ok(())
    }
}

#[cfg(feature = "stream")]
struct ShortWriter {
    output: Vec<u8>,
    max_write: usize,
    write_calls: usize,
}

#[cfg(feature = "stream")]
impl Write for ShortWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        self.write_calls += 1;
        let written = input.len().min(self.max_write);
        self.output.extend_from_slice(&input[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "stream")]
struct OverReportingWriter;

#[cfg(feature = "stream")]
impl Write for OverReportingWriter {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        Ok(input.len() + 1)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "stream")]
#[test]
fn stream_engine_convenience_constructors_attach_policy() {
    let mut encoder = STANDARD.encoder_writer(Vec::new());
    assert_eq!(encoder.engine(), STANDARD);
    encoder.write_all(b"hello").unwrap();
    assert_eq!(encoder.finish().unwrap(), b"aGVsbG8=");

    let mut decoder = STANDARD.decoder_writer(Vec::new());
    assert_eq!(decoder.engine(), STANDARD);
    decoder.write_all(b"aGVsbG8=").unwrap();
    assert_eq!(decoder.finish().unwrap(), b"hello");

    let mut reader = URL_SAFE_NO_PAD.encoder_reader(&b"\xfb\xff"[..]);
    assert_eq!(reader.engine(), URL_SAFE_NO_PAD);
    let mut encoded = String::new();
    reader.read_to_string(&mut encoded).unwrap();
    assert_eq!(encoded, "-_8");

    let mut reader = URL_SAFE_NO_PAD.decoder_reader(&b"-_8"[..]);
    assert_eq!(reader.engine(), URL_SAFE_NO_PAD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_handles_chunk_boundaries() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    assert_eq!(encoder.engine(), STANDARD);
    assert!(encoder.is_padded());
    assert_eq!(encoder.pending_len(), 0);
    assert_eq!(encoder.pending_input_needed_len(), 0);
    assert!(!encoder.has_pending_input());
    encoder.write_all(b"h").unwrap();
    assert_eq!(encoder.pending_len(), 1);
    assert_eq!(encoder.pending_input_needed_len(), 2);
    assert!(encoder.has_pending_input());
    encoder.write_all(b"el").unwrap();
    assert_eq!(encoder.pending_len(), 0);
    assert_eq!(encoder.pending_input_needed_len(), 0);
    assert!(!encoder.has_pending_input());
    assert_eq!(encoder.buffered_output_len(), 4);
    assert_eq!(encoder.buffered_output_capacity(), 1024);
    assert_eq!(encoder.buffered_output_remaining_capacity(), 1020);
    assert!(encoder.has_buffered_output());
    encoder.write_all(b"lo").unwrap();
    assert_eq!(encoder.pending_len(), 2);
    assert_eq!(encoder.pending_input_needed_len(), 1);
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_supports_no_padding() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD_NO_PAD);
    assert_eq!(encoder.engine(), STANDARD_NO_PAD);
    assert!(!encoder.is_padded());
    encoder.write_all(b"he").unwrap();
    encoder.write_all(b"llo").unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_exposes_inner_writer() {
    let mut encoder = Encoder::new(Vec::new(), URL_SAFE_NO_PAD);
    assert!(encoder.get_ref().is_empty());
    encoder.write_all(b"\xfb\xff").unwrap();
    assert!(encoder.get_ref().is_empty());
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"-_8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_try_finish_keeps_adapter_available() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    assert!(!encoder.is_finalized());
    encoder.write_all(b"he").unwrap();
    assert!(encoder.has_pending_input());

    encoder.try_finish().unwrap();
    assert!(encoder.is_finalized());
    assert_eq!(encoder.get_ref(), b"aGU=");
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    let err = encoder.write(&[]).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    let err = encoder.write_all(b"llo").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let inner = encoder.finish().unwrap();
    assert_eq!(inner, b"aGU=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_try_finish_write_failure_buffers_output_for_retry() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: true,
        fail_flush_next: false,
    };
    let mut encoder = Encoder::new(writer, STANDARD);
    encoder.write_all(b"he").unwrap();

    let err = encoder.try_finish().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(encoder.is_finalized());
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    assert!(encoder.has_buffered_output());
    assert_eq!(encoder.get_ref().output, b"");

    encoder.try_finish().unwrap();
    assert!(encoder.is_finalized());
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_buffered_output());
    assert_eq!(encoder.get_ref().output, b"aGU=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_try_finish_flush_failure_does_not_reemit_final_quantum() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: false,
        fail_flush_next: true,
    };
    let mut encoder = Encoder::new(writer, STANDARD);
    encoder.write_all(b"he").unwrap();

    let err = encoder.try_finish().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(encoder.is_finalized());
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    assert_eq!(encoder.get_ref().output, b"aGU=");

    encoder.try_finish().unwrap();
    assert!(encoder.is_finalized());
    assert_eq!(encoder.get_ref().output, b"aGU=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_write_failure_preserves_pending_input() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: true,
        fail_flush_next: false,
    };
    let mut encoder = Encoder::new(writer, STANDARD);
    encoder.write_all(b"h").unwrap();
    assert_eq!(encoder.pending_len(), 1);
    assert!(encoder.has_pending_input());

    encoder.write_all(b"el").unwrap();
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    assert!(encoder.has_buffered_output());

    let err = encoder.flush().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    assert!(encoder.has_buffered_output());
    assert_eq!(encoder.get_ref().output, b"");

    encoder.flush().unwrap();
    assert_eq!(encoder.pending_len(), 0);
    assert!(!encoder.has_pending_input());
    assert!(!encoder.has_buffered_output());
    assert_eq!(encoder.get_ref().output, b"aGVs");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_direct_write_buffers_tail_bytes() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);

    let written = encoder.write(b"hello").unwrap();
    assert_eq!(written, 5);
    assert_eq!(encoder.get_ref(), b"");
    assert_eq!(encoder.buffered_output_len(), 4);
    assert!(encoder.has_buffered_output());
    assert_eq!(encoder.pending_len(), 2);
    assert!(!encoder.can_into_inner());

    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, b"aGVsbG8=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_direct_write_reports_partial_progress_for_large_input() {
    let input = vec![b'a'; 1025];
    let mut encoder = Encoder::new(Vec::new(), STANDARD);

    let written = encoder.write(&input).unwrap();
    assert_eq!(written, 768);
    assert_eq!(encoder.buffered_output_len(), 1024);
    assert_eq!(encoder.pending_len(), 0);
    assert!(encoder.has_buffered_output());

    encoder.write_all(&input[written..]).unwrap();
    let encoded = encoder.finish().unwrap();
    assert_eq!(encoded, STANDARD.encode_vec(&input).unwrap());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_drains_buffered_output_with_short_writes() {
    let writer = ShortWriter {
        output: Vec::new(),
        max_write: 2,
        write_calls: 0,
    };
    let mut encoder = Encoder::new(writer, STANDARD);

    assert_eq!(encoder.write(b"hello").unwrap(), 5);
    assert_eq!(encoder.buffered_output_len(), 4);
    assert_eq!(encoder.pending_len(), 2);
    encoder.flush().unwrap();
    assert_eq!(encoder.buffered_output_len(), 0);
    assert_eq!(encoder.get_ref().output, b"aGVs");
    assert_eq!(encoder.get_ref().write_calls, 2);

    let writer = encoder.finish().unwrap();
    assert_eq!(writer.output, b"aGVsbG8=");
    assert_eq!(writer.write_calls, 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_into_inner_still_returns_writer() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    encoder.write_all(b"he").unwrap();
    let inner = encoder.into_inner();
    assert!(inner.is_empty());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_try_into_inner_rejects_pending_input() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    assert!(encoder.can_into_inner());
    encoder.write_all(b"he").unwrap();
    assert!(!encoder.can_into_inner());

    let mut encoder = match encoder.try_into_inner() {
        Ok(_) => panic!("pending stream encoder was recovered"),
        Err(encoder) => encoder,
    };

    assert_eq!(encoder.pending_len(), 2);
    encoder.try_finish().unwrap();
    assert!(encoder.can_into_inner());
    assert_eq!(encoder.try_into_inner().unwrap(), b"aGU=");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_try_into_inner_returns_writer_without_pending_input() {
    let mut encoder = Encoder::new(Vec::new(), STANDARD);
    encoder.write_all(b"hel").unwrap();
    assert!(!encoder.can_into_inner());
    assert!(encoder.has_buffered_output());
    encoder.flush().unwrap();
    assert!(encoder.can_into_inner());

    let inner = encoder.try_into_inner().unwrap();

    assert_eq!(inner, b"aGVs");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_marks_failed_after_unrecoverable_internal_error() {
    let mut encoder = Encoder::new(OverReportingWriter, STANDARD);
    encoder.write_all(b"hel").unwrap();

    let err = encoder.flush().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(encoder.is_failed());
    assert!(!encoder.can_into_inner());
    assert_eq!(
        encoder.write(b"lo").unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_small_reads() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
    assert_eq!(reader.engine(), STANDARD);
    assert!(reader.is_padded());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.pending_input_needed_len(), 0);
    assert!(!reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.buffered_output_capacity(), 1024);
    assert_eq!(reader.buffered_output_remaining_capacity(), 1024);
    assert!(!reader.has_buffered_output());
    let mut output = [0u8; 8];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"aGVsbG8=");
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.pending_input_needed_len(), 0);
    assert!(!reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.buffered_output_remaining_capacity(), 1024);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_reports_buffered_output() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD);
    assert!(!reader.is_failed());
    assert!(!reader.is_finished());
    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert_eq!(first, [b'a']);
    assert_eq!(reader.pending_len(), 2);
    assert_eq!(reader.pending_input_needed_len(), 1);
    assert!(reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 3);
    assert_eq!(reader.buffered_output_capacity(), 1024);
    assert_eq!(reader.buffered_output_remaining_capacity(), 1021);
    assert!(reader.has_buffered_output());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());

    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"GVsbG8=");
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.pending_input_needed_len(), 0);
    assert_eq!(reader.buffered_output_remaining_capacity(), 1024);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
    assert!(!reader.is_failed());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_finished_input_before_buffer_drain() {
    let mut reader = EncoderReader::new(&b"h"[..], STANDARD);
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());

    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert_eq!(first, [b'a']);
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.buffered_output_len(), 3);
    assert!(reader.has_buffered_output());
    assert!(reader.has_finished_input());
    assert!(!reader.is_finished());

    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"A==");
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_no_padding() {
    let mut reader = EncoderReader::new(&b"hello"[..], STANDARD_NO_PAD);
    assert_eq!(reader.engine(), STANDARD_NO_PAD);
    assert!(!reader.is_padded());
    let mut encoded = Vec::new();
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"aGVsbG8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_supports_url_safe() {
    let mut reader = EncoderReader::new(&b"\xfb\xff"[..], URL_SAFE_NO_PAD);
    let mut encoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 2);
    reader.read_to_end(&mut encoded).unwrap();
    assert_eq!(encoded, b"-_8");
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_into_inner_still_returns_reader() {
    let reader = EncoderReader::new(&b"hello"[..], STANDARD);
    let inner = reader.into_inner();
    assert_eq!(inner, &b"hello"[..]);
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_try_into_inner_rejects_buffered_output() {
    let mut reader = EncoderReader::new(Cursor::new(&b"hello"[..]), STANDARD);
    assert!(!reader.can_into_inner());
    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert!(!reader.can_into_inner());

    let mut reader = match reader.try_into_inner() {
        Ok(_) => panic!("buffered encoder reader was recovered"),
        Err(reader) => reader,
    };

    assert!(reader.has_buffered_output());
    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert!(reader.can_into_inner());
    assert_eq!(reader.try_into_inner().unwrap().position(), 5);
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_try_into_inner_returns_finished_reader() {
    let mut reader = EncoderReader::new(Cursor::new(&b"hello"[..]), STANDARD);
    let mut encoded = Vec::new();
    reader.read_to_end(&mut encoded).unwrap();
    assert!(reader.can_into_inner());

    let inner = reader.try_into_inner().unwrap();

    assert_eq!(encoded, b"aGVsbG8=");
    assert_eq!(inner.position(), 5);
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_handles_fragmented_sources() {
    let input = b"any carnal pleasure.";
    let source = ChunkedReader {
        input,
        max_chunk: 1,
    };
    let mut reader = EncoderReader::new(source, STANDARD);
    let mut encoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        encoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(encoded, STANDARD.encode_vec(input).unwrap());
}

#[cfg(feature = "stream")]
#[test]
fn stream_encoder_reader_propagates_read_error_after_wiping_input_buffer() {
    let source = PoisoningReadError {
        poison: b"raw-private-key-material",
        kind: std::io::ErrorKind::Interrupted,
    };
    let mut reader = EncoderReader::new(source, STANDARD);
    let mut output = [0u8; 8];

    let err = reader.read(&mut output).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::Interrupted);
    assert_eq!(reader.pending_len(), 0);
    assert!(!reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(!reader.has_buffered_output());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_handles_chunk_boundaries() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    assert_eq!(decoder.engine(), STANDARD);
    assert!(decoder.is_padded());
    assert_eq!(decoder.pending_len(), 0);
    assert_eq!(decoder.pending_input_needed_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(!decoder.has_terminal_padding());
    decoder.write_all(b"a").unwrap();
    assert_eq!(decoder.pending_len(), 1);
    assert_eq!(decoder.pending_input_needed_len(), 3);
    assert!(decoder.has_pending_input());
    decoder.write_all(b"GVs").unwrap();
    assert_eq!(decoder.pending_len(), 0);
    assert_eq!(decoder.pending_input_needed_len(), 0);
    assert!(!decoder.has_pending_input());
    assert_eq!(decoder.buffered_output_len(), 3);
    assert_eq!(decoder.buffered_output_capacity(), 1024);
    assert_eq!(decoder.buffered_output_remaining_capacity(), 1021);
    assert!(decoder.has_buffered_output());
    decoder.write_all(b"bG8=").unwrap();
    assert!(decoder.has_terminal_padding());
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_supports_no_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD_NO_PAD);
    assert_eq!(decoder.engine(), STANDARD_NO_PAD);
    assert!(!decoder.is_padded());
    decoder.write_all(b"aGV").unwrap();
    decoder.write_all(b"sbG8").unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_finish_keeps_adapter_available() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    assert!(!decoder.is_finalized());
    decoder.write_all(b"aGk=").unwrap();
    assert!(decoder.has_terminal_padding());

    decoder.try_finish().unwrap();
    assert!(decoder.is_finalized());
    assert_eq!(decoder.get_ref(), b"hi");
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    let err = decoder.write_all(b"AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let inner = decoder.finish().unwrap();
    assert_eq!(inner, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_finish_terminal_for_unpadded_payloads() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD_NO_PAD);
    assert!(!decoder.is_finalized());
    decoder.write_all(b"aGk").unwrap();

    decoder.try_finish().unwrap();
    assert!(decoder.is_finalized());
    assert_eq!(decoder.get_ref(), b"hi");
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_terminal_padding());

    let err = decoder.write_all(b"AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_finish_flush_failure_does_not_reemit_final_bytes() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: false,
        fail_flush_next: true,
    };
    let mut decoder = Decoder::new(writer, STANDARD_NO_PAD);
    decoder.write_all(b"aGk").unwrap();

    let err = decoder.try_finish().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(decoder.is_finalized());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert_eq!(decoder.get_ref().output, b"hi");

    decoder.try_finish().unwrap();
    assert!(decoder.is_finalized());
    assert_eq!(decoder.get_ref().output, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_finish_write_failure_buffers_output_for_retry() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: true,
        fail_flush_next: false,
    };
    let mut decoder = Decoder::new(writer, STANDARD_NO_PAD);
    decoder.write_all(b"aGk").unwrap();

    let err = decoder.try_finish().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert!(decoder.is_finalized());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(decoder.has_buffered_output());
    assert_eq!(decoder.get_ref().output, b"");

    decoder.try_finish().unwrap();
    assert!(decoder.is_finalized());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(!decoder.has_buffered_output());
    assert_eq!(decoder.get_ref().output, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_write_failure_preserves_pending_input() {
    let writer = FailOnceWriter {
        output: Vec::new(),
        fail_next: true,
        fail_flush_next: false,
    };
    let mut decoder = Decoder::new(writer, STANDARD);
    decoder.write_all(b"a").unwrap();
    assert_eq!(decoder.pending_len(), 1);
    assert!(decoder.has_pending_input());

    decoder.write_all(b"Gk=").unwrap();
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(decoder.has_terminal_padding());
    assert!(decoder.has_buffered_output());

    let err = decoder.flush().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(decoder.has_terminal_padding());
    assert!(decoder.has_buffered_output());
    assert_eq!(decoder.get_ref().output, b"");

    decoder.flush().unwrap();
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(decoder.has_terminal_padding());
    assert!(!decoder.has_buffered_output());
    assert_eq!(decoder.get_ref().output, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_direct_write_processes_multiple_quads() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    let written = decoder.write(b"aGVsbG8=").unwrap();
    assert_eq!(written, 8);
    assert_eq!(decoder.buffered_output_len(), 5);
    assert!(decoder.has_terminal_padding());

    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_direct_write_continues_after_pending_quad() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    assert_eq!(decoder.write(b"a").unwrap(), 1);
    assert_eq!(decoder.pending_len(), 1);

    let written = decoder.write(b"GVsbG8=").unwrap();
    assert_eq!(written, 7);
    assert_eq!(decoder.pending_len(), 0);
    assert_eq!(decoder.buffered_output_len(), 5);
    assert!(decoder.has_terminal_padding());

    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_direct_write_reports_partial_progress_for_large_input() {
    let input = vec![b'a'; 1500];
    let encoded = STANDARD.encode_vec(&input).unwrap();
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    let written = decoder.write(&encoded).unwrap();
    assert_eq!(written, 1364);
    assert_eq!(decoder.buffered_output_len(), 1023);
    assert_eq!(decoder.pending_len(), 0);
    assert!(decoder.has_buffered_output());

    decoder.write_all(&encoded[written..]).unwrap();
    let decoded = decoder.finish().unwrap();
    assert_eq!(decoded, input);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_direct_write_reports_partial_progress() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    let written = decoder.write(b"aGk=AA").unwrap();
    assert_eq!(written, 4);
    assert_eq!(decoder.get_ref(), b"");
    assert_eq!(decoder.buffered_output_len(), 2);
    assert!(decoder.has_buffered_output());
    assert!(decoder.has_terminal_padding());
    assert!(!decoder.can_into_inner());

    decoder.flush().unwrap();
    assert_eq!(decoder.get_ref(), b"hi");
    assert!(!decoder.has_buffered_output());

    let err = decoder.write(b"AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_direct_write_marks_failed_after_deferred_error() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    let written = decoder.write(b"Zm9v$$$$").unwrap();
    assert_eq!(written, 4);
    assert!(decoder.is_failed());
    assert_eq!(decoder.buffered_output_len(), 3);

    let err = decoder.write(b"YmFy").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(decoder.is_failed());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_drains_buffered_output_with_short_writes() {
    let writer = ShortWriter {
        output: Vec::new(),
        max_write: 1,
        write_calls: 0,
    };
    let mut decoder = Decoder::new(writer, STANDARD);

    assert_eq!(decoder.write(b"aGk=AA").unwrap(), 4);
    assert_eq!(decoder.buffered_output_len(), 2);
    decoder.flush().unwrap();
    assert_eq!(decoder.buffered_output_len(), 0);
    assert_eq!(decoder.get_ref().output, b"hi");
    assert_eq!(decoder.get_ref().write_calls, 2);

    let err = decoder.write(b"AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_bad_final_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    assert!(decoder.finish().is_err());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_finish_reports_bad_final_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();

    let err = decoder.try_finish().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!decoder.is_finalized());
    assert!(decoder.is_failed());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.has_pending_input());
    assert!(!decoder.can_into_inner());
    assert!(decoder.get_ref().is_empty());

    assert_eq!(
        decoder.write_all(b"Gk=").unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(
        decoder.try_finish().unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_trailing_input_after_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    let err = decoder.write_all(b"aGk=AA").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(decoder.is_failed());
    assert!(!decoder.can_into_inner());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_rejects_short_trailing_input_after_pending_padding() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"aG").unwrap();
    let err = decoder.write_all(b"k=A").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(decoder.is_failed());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.can_into_inner());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_fails_closed_after_malformed_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);

    let err = decoder.write(b"!!!!").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(decoder.is_failed());
    assert!(!decoder.is_finalized());
    assert_eq!(decoder.pending_len(), 0);
    assert!(!decoder.can_into_inner());

    assert_eq!(
        decoder.write(b"aGk=").unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    assert_eq!(
        decoder.flush().unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_exposes_inner_writer_after_refactor() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    assert!(decoder.get_ref().is_empty());
    decoder.write_all(b"aGk=").unwrap();
    assert!(decoder.has_buffered_output());
    assert!(decoder.get_ref().is_empty());
    decoder.flush().unwrap();
    assert!(!decoder.has_buffered_output());
    assert_eq!(decoder.get_ref(), b"hi");
    let inner = decoder.finish().unwrap();
    assert_eq!(inner, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_into_inner_still_returns_writer() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"a").unwrap();
    let inner = decoder.into_inner();
    assert!(inner.is_empty());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_into_inner_rejects_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    assert!(decoder.can_into_inner());
    decoder.write_all(b"aG").unwrap();
    assert!(!decoder.can_into_inner());

    let mut decoder = match decoder.try_into_inner() {
        Ok(_) => panic!("pending stream decoder was recovered"),
        Err(decoder) => decoder,
    };

    assert_eq!(decoder.pending_len(), 2);
    decoder.write_all(b"k=").unwrap();
    assert!(!decoder.can_into_inner());
    assert!(decoder.has_buffered_output());
    decoder.flush().unwrap();
    assert!(decoder.can_into_inner());
    assert_eq!(decoder.try_into_inner().unwrap(), b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_try_into_inner_returns_writer_without_pending_input() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"aGk=").unwrap();
    assert!(!decoder.can_into_inner());
    assert!(decoder.has_buffered_output());
    decoder.flush().unwrap();
    assert!(decoder.can_into_inner());

    let inner = decoder.try_into_inner().unwrap();

    assert_eq!(inner, b"hi");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_small_reads() {
    let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
    assert_eq!(reader.engine(), STANDARD);
    assert!(reader.is_padded());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.pending_input_needed_len(), 0);
    assert!(!reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.buffered_output_capacity(), 3);
    assert_eq!(reader.buffered_output_remaining_capacity(), 3);
    assert!(!reader.has_buffered_output());
    assert!(!reader.has_terminal_padding());
    let mut output = [0u8; 5];
    let mut written = 0;
    while written < output.len() {
        let read = reader.read(&mut output[written..written + 1]).unwrap();
        if read == 0 {
            break;
        }
        written += read;
    }
    assert_eq!(&output[..written], b"hello");
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.pending_input_needed_len(), 0);
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.buffered_output_remaining_capacity(), 3);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_terminal_padding());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_reports_buffered_output() {
    let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
    assert!(!reader.is_finished());
    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert_eq!(first, [b'h']);
    assert_eq!(reader.buffered_output_len(), 2);
    assert_eq!(reader.buffered_output_capacity(), 3);
    assert_eq!(reader.buffered_output_remaining_capacity(), 1);
    assert!(reader.has_buffered_output());
    assert!(!reader.has_terminal_padding());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());

    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"ello");
    assert_eq!(reader.buffered_output_len(), 0);
    assert_eq!(reader.buffered_output_remaining_capacity(), 3);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_terminal_padding());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_terminal_padding_finishes_after_buffer_drain() {
    let mut reader = DecoderReader::new(&b"aGk="[..], STANDARD);
    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert_eq!(first, [b'h']);
    assert_eq!(reader.buffered_output_len(), 1);
    assert!(reader.has_buffered_output());
    assert!(reader.has_terminal_padding());
    assert!(reader.has_finished_input());
    assert!(!reader.is_finished());

    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert_eq!(rest, b"i");
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(!reader.has_buffered_output());
    assert!(reader.has_terminal_padding());
    assert!(reader.has_finished_input());
    assert!(reader.is_finished());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_no_padding() {
    let mut reader = DecoderReader::new(&b"aGVsbG8"[..], STANDARD_NO_PAD);
    assert_eq!(reader.engine(), STANDARD_NO_PAD);
    assert!(!reader.is_padded());
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hello");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_supports_url_safe() {
    let mut reader = DecoderReader::new(&b"-_8"[..], URL_SAFE_NO_PAD);
    let mut decoded = Vec::new();
    assert_eq!(reader.get_ref().len(), 3);
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"\xfb\xff");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_rejects_bad_final_pending_input() {
    let mut reader = DecoderReader::new(&b"a"[..], STANDARD);
    let mut decoded = Vec::new();
    assert!(reader.read_to_end(&mut decoded).is_err());
    assert!(reader.is_failed());
    assert_eq!(reader.pending_len(), 0);
    assert!(!reader.can_into_inner());

    let mut output = [0u8; 1];
    assert_eq!(
        reader.read(&mut output).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_fails_closed_after_malformed_input() {
    let mut reader = DecoderReader::new(Cursor::new(&b"!!!!aGk="[..]), STANDARD);
    let mut output = [0u8; 1];

    let err = reader.read(&mut output).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    assert!(reader.is_failed());
    assert!(!reader.can_into_inner());

    assert_eq!(
        reader.read(&mut output).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_propagates_read_error_after_wiping_input_buffer() {
    let source = PoisoningReadError {
        poison: b"aGVs",
        kind: std::io::ErrorKind::Interrupted,
    };
    let mut reader = DecoderReader::new(source, STANDARD);
    let mut output = [0u8; 3];

    let err = reader.read(&mut output).unwrap_err();

    assert_eq!(err.kind(), std::io::ErrorKind::Interrupted);
    assert_eq!(reader.pending_len(), 0);
    assert!(!reader.has_pending_input());
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(!reader.has_buffered_output());
    assert!(!reader.has_finished_input());
    assert!(!reader.is_finished());
    assert!(!reader.is_failed());
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_into_inner_still_returns_reader() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGVsbG8="[..]), STANDARD);
    let mut output = [0u8; 1];
    let read = reader.read(&mut output).unwrap();
    assert_eq!(read, 1);
    assert_eq!(output, [b'h']);

    let inner = reader.into_inner();
    assert_eq!(inner.position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_try_into_inner_rejects_buffered_output() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=NEXT"[..]), STANDARD);
    assert!(!reader.can_into_inner());
    let mut first = [0u8; 1];
    assert_eq!(reader.read(&mut first).unwrap(), 1);
    assert!(!reader.can_into_inner());

    let mut reader = match reader.try_into_inner() {
        Ok(_) => panic!("buffered decoder reader was recovered"),
        Err(reader) => reader,
    };

    assert!(reader.has_buffered_output());
    let mut rest = Vec::new();
    reader.read_to_end(&mut rest).unwrap();
    assert!(reader.can_into_inner());
    assert_eq!(reader.try_into_inner().unwrap().position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_try_into_inner_returns_finished_reader() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=NEXT"[..]), STANDARD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert!(reader.can_into_inner());

    let inner = reader.try_into_inner().unwrap();

    assert_eq!(decoded, b"hi");
    assert_eq!(inner.position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_leaves_trailing_input_after_padding_unread() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=AA"[..]), STANDARD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hi");
    assert_eq!(reader.get_ref().position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_leaves_adjacent_payload_unread_after_padding() {
    let mut reader = DecoderReader::new(Cursor::new(&b"aGk=NEXT"[..]), STANDARD);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded).unwrap();
    assert_eq!(decoded, b"hi");
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(!reader.has_buffered_output());
    assert!(!reader.has_pending_input());
    assert!(reader.has_terminal_padding());
    assert!(reader.is_finished());
    assert_eq!(reader.get_ref().position(), 4);
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_leaves_fragmented_adjacent_payload_unread() {
    let source = FramedChunkedReader {
        input: b"aGk=NEXT",
        max_chunk: 2,
    };
    let mut reader = DecoderReader::new(source, STANDARD);
    let mut decoded = Vec::new();
    let mut scratch = [0u8; 1];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        decoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(decoded, b"hi");
    assert_eq!(reader.pending_len(), 0);
    assert_eq!(reader.buffered_output_len(), 0);
    assert!(reader.has_terminal_padding());
    assert!(reader.is_finished());
    assert_eq!(reader.get_ref().remaining(), b"NEXT");
}

#[cfg(feature = "stream")]
#[test]
fn stream_decoder_reader_handles_fragmented_sources() {
    let encoded = b"YW55IGNhcm5hbCBwbGVhc3VyZS4=";
    let source = ChunkedReader {
        input: encoded,
        max_chunk: 1,
    };
    let mut reader = DecoderReader::new(source, STANDARD);
    let mut decoded = Vec::new();
    let mut scratch = [0u8; 2];

    loop {
        let read = reader.read(&mut scratch).unwrap();
        if read == 0 {
            break;
        }
        decoded.extend_from_slice(&scratch[..read]);
    }

    assert_eq!(decoded, b"any carnal pleasure.");
}

#[cfg(feature = "stream")]
#[test]
fn stream_debug_output_redacts_wrapped_io() {
    let mut encoder = Encoder::new(b"raw-secret".to_vec(), STANDARD);
    encoder.write_all(b"h").unwrap();
    let debug = format!("{encoder:?}");
    assert!(debug.contains("Encoder"));
    assert!(debug.contains("pending_len"));
    assert!(debug.contains("pending_input_needed_len"));
    assert!(debug.contains("buffered_output_len"));
    assert!(debug.contains("buffered_output_capacity"));
    assert!(debug.contains("buffered_output_remaining_capacity"));
    assert!(debug.contains("can_into_inner"));
    assert!(debug.contains("<present>"));
    assert!(!debug.contains("raw-secret"));

    let mut decoder = Decoder::new(b"decoded-secret".to_vec(), STANDARD);
    decoder.write_all(b"aGk=").unwrap();
    let debug = format!("{decoder:?}");
    assert!(debug.contains("Decoder"));
    assert!(debug.contains("pending_input_needed_len"));
    assert!(debug.contains("buffered_output_len"));
    assert!(debug.contains("buffered_output_capacity"));
    assert!(debug.contains("buffered_output_remaining_capacity"));
    assert!(debug.contains("can_into_inner"));
    assert!(debug.contains("terminal_padding"));
    assert!(!debug.contains("decoded-secret"));

    let mut decoder_reader = DecoderReader::new(Cursor::new(&b"aGVsbG8="[..]), STANDARD);
    let mut one = [0u8; 1];
    decoder_reader.read_exact(&mut one).unwrap();
    let debug = format!("{decoder_reader:?}");
    assert!(debug.contains("DecoderReader"));
    assert!(debug.contains("pending_input_needed_len"));
    assert!(debug.contains("buffered_output_len"));
    assert!(debug.contains("buffered_output_capacity"));
    assert!(debug.contains("buffered_output_remaining_capacity"));
    assert!(debug.contains("can_into_inner"));
    assert!(!debug.contains("aGVsbG8"));

    let mut encoder_reader = EncoderReader::new(Cursor::new(&b"raw-secret"[..]), STANDARD);
    encoder_reader.read_exact(&mut one).unwrap();
    let debug = format!("{encoder_reader:?}");
    assert!(debug.contains("EncoderReader"));
    assert!(debug.contains("pending_input_needed_len"));
    assert!(debug.contains("buffered_output_len"));
    assert!(debug.contains("buffered_output_capacity"));
    assert!(debug.contains("buffered_output_remaining_capacity"));
    assert!(debug.contains("can_into_inner"));
    assert!(!debug.contains("raw-secret"));
}
