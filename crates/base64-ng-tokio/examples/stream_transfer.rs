//! Bounded ordinary transfer with an exact frame, backpressure and shutdown.
//! Already delivered prefixes cannot be recalled on a later decoding error.
use base64_ng::STRICT_STANDARD_PADDED as CODEC;
use base64_ng_tokio::{DecoderReader, EncoderReader};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let raw = vec![0xfb; 8193];
    let encoded_len = CODEC.encoded_len(raw.len()).map_err(io::Error::other)?;
    let (mut sender, receiver) = tokio::io::duplex(256);
    let send = async {
        let mut encoder = EncoderReader::new_exact(raw.as_slice(), &CODEC, raw.len());
        tokio::io::copy(&mut encoder, &mut sender).await?;
        sender.shutdown().await
    };
    let receive = async {
        // A protocol should validate encoded_len against its ceiling first.
        let mut decoder = DecoderReader::new_exact(receiver, &CODEC, encoded_len);
        let mut output = Vec::new();
        (&mut decoder)
            .take(u64::try_from(raw.len()).unwrap() + 1)
            .read_to_end(&mut output)
            .await?;
        if !decoder.is_complete() || output != raw {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "incomplete or oversized transfer",
            ));
        }
        Ok(())
    };
    let (upload_result, download_result) = tokio::join!(send, receive);
    upload_result?;
    download_result
}
