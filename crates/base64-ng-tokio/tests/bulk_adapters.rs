#![allow(missing_docs)]

use base64_ng::{DecodeValidation, STRICT_STANDARD_PADDED as CODEC};
use base64_ng_tokio::{DecoderReader, DecoderWriter, EncoderReader};
use std::{
    io,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Default)]
struct PendingSink {
    bytes: Vec<u8>,
    pending: bool,
}

impl AsyncWrite for PendingSink {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        self.pending = true;
        let count = input.len().min(17);
        self.bytes.extend_from_slice(&input[..count]);
        Poll::Ready(Ok(count))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test]
async fn bulk_writer_pending_cancellation_resume_and_policy_are_bounded() {
    let raw = vec![0xfb; 16385];
    let encoded = CODEC.encode_to_string(&raw).unwrap();
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let mut writer = DecoderWriter::new(PendingSink::default(), &CODEC).with_validation(policy);
        let accepted = writer.write(encoded.as_bytes()).await.unwrap();
        assert_eq!(accepted, 1364);
        assert!(writer.buffered_output_len() <= 1024);
        let mut cx = Context::from_waker(Waker::noop());
        assert!(
            Pin::new(&mut writer)
                .poll_write(&mut cx, &encoded.as_bytes()[accepted..])
                .is_pending()
        );
        assert_eq!(writer.input_accepted(), accepted);
        assert_eq!(writer.output_committed(), 17);
        // Dropping the pending operation, not the adapter, must retain its queue.
        writer
            .write_all(&encoded.as_bytes()[accepted..])
            .await
            .unwrap();
        writer.shutdown().await.unwrap();
        assert_eq!(writer.output_committed(), raw.len());
        assert!(writer.is_shutdown());
        assert_eq!(writer.into_inner().bytes, raw);
    }
}

#[tokio::test]
async fn bulk_reader_exact_frame_and_eof_rejection_preserve_boundaries() {
    let raw = vec![0xfb; 4097];
    let encoded = CODEC.encode_to_string(&raw).unwrap();
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let mut wire = encoded.as_bytes().to_vec();
        wire.extend_from_slice(b"adjacent");
        let mut source = wire.as_slice();
        let mut reader =
            DecoderReader::new_exact(&mut source, &CODEC, encoded.len()).with_validation(policy);
        let mut output = Vec::new();
        reader.read_to_end(&mut output).await.unwrap();
        assert_eq!(output, raw);
        assert_eq!(reader.remaining_input(), Some(0));
        drop(reader);
        assert_eq!(source, b"adjacent");

        let source = &encoded.as_bytes()[..encoded.len() - 1];
        let mut reader =
            DecoderReader::new_exact(source, &CODEC, encoded.len()).with_validation(policy);
        assert_eq!(
            reader
                .read_to_end(&mut Vec::new())
                .await
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert!(reader.is_failed());
    }
}

#[tokio::test]
async fn bulk_encoder_reader_to_decoder_reader_handles_real_duplex_backpressure() {
    let raw = vec![0xfb; 16385];
    let (mut sender, receiver) = tokio::io::duplex(37);
    let send = async {
        let mut encoder = EncoderReader::new_exact(raw.as_slice(), &CODEC, raw.len());
        tokio::io::copy(&mut encoder, &mut sender).await.unwrap();
        sender.shutdown().await.unwrap();
        assert!(encoder.is_complete());
    };
    let receive = async {
        let mut decoder = DecoderReader::new(receiver, &CODEC);
        let mut output = Vec::new();
        decoder.read_to_end(&mut output).await.unwrap();
        assert_eq!(output, raw);
    };
    tokio::join!(send, receive);
}
