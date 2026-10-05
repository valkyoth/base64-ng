#![cfg(feature = "stream")]

use base64_ng::{
    DecodeValidation, STANDARD,
    stream::{Decoder, DecoderReader},
};
use std::io::{self, Cursor, Read, Write};

#[test]
fn stream_bulk_finish_retains_tail_when_a_full_queue_drain_must_retry() {
    let mut decoder = Decoder::new(
        RetrySink {
            output: Vec::new(),
            failure: Some(io::ErrorKind::BrokenPipe),
        },
        base64_ng::STANDARD_NO_PAD,
    );
    decoder.write_all(&[b'A'; 1367]).unwrap();
    assert_eq!(decoder.buffered_output_len(), 1023);
    assert_eq!(decoder.pending_len(), 3);
    assert_eq!(
        decoder.try_finish().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(!decoder.is_failed());
    decoder.flush().unwrap();
    assert!(!decoder.can_into_inner());
    assert_eq!(decoder.finish().unwrap().output, vec![0; 1025]);

    let mut encoder = base64_ng::stream::Encoder::new(
        RetrySink {
            output: Vec::new(),
            failure: Some(io::ErrorKind::BrokenPipe),
        },
        STANDARD,
    );
    encoder.write_all(b"x").unwrap();
    encoder.write_all(&[0; 768]).unwrap();
    assert_eq!(encoder.buffered_output_len(), 1024);
    assert_eq!(encoder.pending_len(), 1);
    assert_eq!(
        encoder.try_finish().unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert!(!encoder.is_failed());
    encoder.flush().unwrap();
    assert!(!encoder.can_into_inner());
    let mut raw = vec![b'x'];
    raw.extend_from_slice(&[0; 768]);
    assert_eq!(
        encoder.finish().unwrap().output,
        STANDARD.encode_vec(&raw).unwrap()
    );
}

struct RetrySink {
    output: Vec<u8>,
    failure: Option<io::ErrorKind>,
}

#[test]
fn stream_bulk_invalid_tail_does_not_drain_a_nearly_full_queue() {
    for validation in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for padded in [false, true] {
            fn check<const PAD: bool>(validation: DecodeValidation) {
                let engine = base64_ng::Engine::<base64_ng::Standard, PAD>::new();
                let sink = RetrySink {
                    output: Vec::new(),
                    failure: Some(io::ErrorKind::BrokenPipe),
                };
                let mut decoder = Decoder::new(sink, engine).with_validation(validation);
                // Three trailing symbols are truncated when padded and have
                // noncanonical terminal bits when unpadded.
                let mut input = vec![b'A'; 1367];
                input[1366] = b'B';
                decoder.write_all(&input).unwrap();
                assert_eq!(decoder.buffered_output_len(), 1023);
                assert_eq!(
                    decoder.try_finish().unwrap_err().kind(),
                    io::ErrorKind::InvalidInput
                );
                assert!(decoder.is_failed());
                let sink = decoder.into_inner();
                assert!(sink.output.is_empty());
                assert_eq!(sink.failure, Some(io::ErrorKind::BrokenPipe));
            }
            if padded {
                check::<true>(validation);
            } else {
                check::<false>(validation);
            }
        }
    }
}

impl Write for RetrySink {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if let Some(kind) = self.failure.take() {
            return if kind == io::ErrorKind::WriteZero {
                Ok(0)
            } else {
                Err(kind.into())
            };
        }
        let n = input.len().min(17);
        self.output.extend_from_slice(&input[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn stream_bulk_retry_keeps_accepted_bytes_and_bounded_queue() {
    let raw = vec![0xfb; 16385];
    let encoded = STANDARD.encode_vec(&raw).unwrap();
    for validation in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for kind in [
            io::ErrorKind::Interrupted,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::WriteZero,
        ] {
            let sink = RetrySink {
                output: Vec::new(),
                failure: Some(kind),
            };
            let mut decoder = Decoder::new(sink, STANDARD).with_validation(validation);
            let accepted = decoder.write(&encoded).unwrap();
            assert_eq!(accepted, 1364);
            assert_eq!(decoder.buffered_output_len(), 1023);
            assert_eq!(decoder.buffered_output_capacity(), 1024);
            assert_eq!(decoder.flush().unwrap_err().kind(), kind);
            assert!(!decoder.is_failed());
            assert_eq!(decoder.buffered_output_len(), 1023);
            decoder.write_all(&encoded[accepted..]).unwrap();
            let sink = decoder.finish().unwrap();
            assert_eq!(sink.output, raw);
        }
    }
}

#[test]
fn stream_bulk_policy_never_expands_padded_reader_frame() {
    let raw = vec![0xfb; 4097];
    let mut wire = STANDARD.encode_vec(&raw).unwrap();
    let boundary = wire.len();
    wire.extend_from_slice(b"next frame");
    for validation in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let mut reader =
            DecoderReader::new(Cursor::new(&wire), STANDARD).with_validation(validation);
        assert_eq!(reader.buffered_output_capacity(), 3);
        let mut output = Vec::new();
        reader.read_to_end(&mut output).unwrap();
        assert_eq!(output, raw);
        let inner = reader.try_into_inner().unwrap();
        assert_eq!(usize::try_from(inner.position()).unwrap(), boundary);
    }
}
