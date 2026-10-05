use std::io::{self, Write};

struct ShortWriter<'a> {
    bytes: &'a mut Vec<u8>,
    limit: usize,
    #[cfg(feature = "adapters")]
    pending: bool,
}
impl Write for ShortWriter<'_> {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        let n = input.len().min(self.limit);
        self.bytes.extend_from_slice(&input[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn sync<A: base64_ng::Alphabet, const PAD: bool>(
    engine: base64_ng::Engine<A, PAD>,
    encode: bool,
    input: &[u8],
    output: &mut Vec<u8>,
    fragment: usize,
) -> Result<usize, ()> {
    output.clear();
    let writer = ShortWriter {
        bytes: output,
        limit: fragment,
        #[cfg(feature = "adapters")]
        pending: false,
    };
    macro_rules! drive {
        ($adapter:expr) => {{
            let mut adapter = $adapter;
            for chunk in input.chunks(fragment) {
                adapter.write_all(chunk).map_err(|_| ())?;
            }
            adapter.finish().map_err(|_| ())?;
        }};
    }
    if encode {
        drive!(base64_ng::stream::Encoder::new(writer, engine));
    } else {
        drive!(base64_ng::stream::Decoder::new(writer, engine));
    }
    Ok(output.len())
}

#[cfg(feature = "adapters")]
impl tokio::io::AsyncWrite for ShortWriter<'_> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
        input: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        if self.pending {
            self.pending = false;
            context.waker().wake_by_ref();
            return std::task::Poll::Pending;
        }
        self.pending = true;
        std::task::Poll::Ready(self.write(input))
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[cfg(feature = "adapters")]
pub fn companion<S: base64_ng::Codec>(
    codec: &base64_ng::Base64<S>,
    operation: &str,
    encode: bool,
    input: &[u8],
    work: &mut super::operations::Work,
    fragment: usize,
) -> Result<usize, ()> {
    work.bytes.clear();
    if operation == "bytes" {
        use base64_ng_bytes::Base64BytesExt as _;
        macro_rules! drive {
            ($state:expr) => {{
                let mut state = $state;
                for mut chunk in input.chunks(fragment) {
                    state.update(&mut chunk, &mut work.bytes).map_err(|_| ())?;
                    if !chunk.is_empty() {
                        return Err(());
                    }
                }
                state.finish(&mut work.bytes).map_err(|_| ())?;
            }};
        }
        if encode {
            drive!(codec.bytes_encoder());
        } else {
            drive!(codec.bytes_decoder());
        }
    } else {
        use tokio::io::AsyncWriteExt as _;
        let writer = ShortWriter {
            bytes: &mut work.bytes,
            limit: fragment,
            pending: false,
        };
        work.runtime
            .block_on(async {
                macro_rules! drive {
                    ($adapter:expr) => {{
                        let mut adapter = $adapter;
                        for chunk in input.chunks(fragment) {
                            adapter.write_all(chunk).await?;
                        }
                        adapter.shutdown().await
                    }};
                }
                if encode {
                    drive!(base64_ng_tokio::EncoderWriter::new(writer, codec))
                } else {
                    drive!(base64_ng_tokio::DecoderWriter::new(writer, codec))
                }
            })
            .map_err(|_: io::Error| ())?;
    }
    Ok(work.bytes.len())
}
