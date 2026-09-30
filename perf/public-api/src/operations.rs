use base64_ng::{Base64, Codec};

pub struct Work {
    pub output: Vec<u8>,
    #[cfg(feature = "alloc")]
    pub bytes: Vec<u8>,
    #[cfg(feature = "alloc")]
    pub text: String,
    #[cfg(feature = "adapters")]
    pub runtime: tokio::runtime::Runtime,
}

impl Work {
    pub fn new(capacity: usize) -> Self {
        Self {
            output: vec![0xa5; capacity],
            #[cfg(feature = "alloc")]
            bytes: Vec::with_capacity(capacity),
            #[cfg(feature = "alloc")]
            text: String::with_capacity(capacity),
            #[cfg(feature = "adapters")]
            runtime: tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap(),
        }
    }
    pub fn result(&self, operation: &str, encode: bool, written: usize) -> &[u8] {
        #[cfg(feature = "alloc")]
        if operation == "append" && encode {
            return &self.text.as_bytes()[..written];
        }
        let _ = encode;
        match operation {
            #[cfg(feature = "alloc")]
            "owned" | "append" => &self.bytes[..written],
            #[cfg(feature = "adapters")]
            "bytes" | "tokio" => &self.bytes[..written],
            #[cfg(feature = "stream")]
            "sync" => &self.bytes[..written],
            _ => &self.output[..written],
        }
    }
}

pub fn names() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut names = vec![
        "historical",
        "canonical",
        "validate",
        "in-place",
        "incremental",
        "base64",
        "base64ct",
    ];
    #[cfg(feature = "alloc")]
    names.extend(["owned", "append"]);
    #[cfg(feature = "validation-policy")]
    names.extend(["historical-reference", "canonical-reference"]);
    #[cfg(feature = "std")]
    {
        #[cfg(feature = "stream")]
        names.push("sync");
        for backend in base64_ng::perf_evidence::EvidenceBackend::ALL {
            if backend != base64_ng::perf_evidence::EvidenceBackend::Auto && backend.is_available()
            {
                names.push(backend.as_str());
            }
        }
    }
    #[cfg(feature = "adapters")]
    names.extend(["bytes", "tokio"]);
    names
}

pub fn apply<S: Codec, A: base64_ng::Alphabet, const PAD: bool>(
    codec: &Base64<S>,
    engine: base64_ng::Engine<A, PAD>,
    operation: &str,
    encode: bool,
    input: &[u8],
    work: &mut Work,
    fragment: usize,
) -> Result<usize, ()> {
    macro_rules! result {
        ($e:expr) => {
            $e.map_err(|_| ())
        };
    }
    match operation {
        #[cfg(feature = "validation-policy")]
        "historical-reference" => {
            if encode {
                result!(engine.encode_slice(input, &mut work.output))
            } else {
                result!(engine.decode_slice_with_validation(
                    input,
                    &mut work.output,
                    base64_ng::DecodeValidation::ScalarReference
                ))
            }
        }
        #[cfg(feature = "validation-policy")]
        "canonical-reference" => {
            if encode {
                result!(codec.encode_into(input, &mut work.output))
            } else {
                result!(codec.decode_into_with_validation(
                    input,
                    &mut work.output,
                    base64_ng::DecodeValidation::ScalarReference
                ))
            }
        }
        "historical" => {
            if encode {
                result!(engine.encode_slice(input, &mut work.output))
            } else {
                result!(engine.decode_slice(input, &mut work.output))
            }
        }
        "canonical" => {
            if encode {
                result!(codec.encode_into(input, &mut work.output))
            } else {
                result!(codec.decode_into(input, &mut work.output))
            }
        }
        "validate" => {
            result!(codec.validate(input))?;
            Ok(0)
        }
        "in-place" => {
            work.output[..input.len()].copy_from_slice(input);
            if encode {
                result!(codec.encode_in_place(&mut work.output, input.len()))
            } else {
                result!(codec.decode_in_place(&mut work.output, input.len()))
            }
        }
        "incremental" => incremental(codec, encode, input, &mut work.output, fragment),
        #[cfg(feature = "alloc")]
        "owned" => {
            work.bytes = if encode {
                result!(codec.encode_to_string(input))?.into_bytes()
            } else {
                result!(codec.decode_to_vec(input))?
            };
            Ok(work.bytes.len())
        }
        #[cfg(feature = "alloc")]
        "append" => {
            work.bytes.clear();
            if encode {
                work.text.clear();
                result!(codec.encode_append(input, &mut work.text))?;
                return Ok(work.text.len());
            } else {
                result!(codec.decode_append(input, &mut work.bytes))?;
            }
            Ok(work.bytes.len())
        }
        #[cfg(feature = "stream")]
        "sync" => super::adapters::sync(engine, encode, input, &mut work.bytes, fragment),
        #[cfg(feature = "adapters")]
        "bytes" | "tokio" => {
            super::adapters::companion(codec, operation, encode, input, work, fragment)
        }
        "base64" | "base64ct" => {
            super::external::apply(codec, operation, encode, input, &mut work.output)
        }
        #[cfg(feature = "std")]
        name => super::external::exact(codec, name, encode, input, &mut work.output),
        #[cfg(not(feature = "std"))]
        _ => Err(()),
    }
}

fn incremental<S: Codec>(
    codec: &Base64<S>,
    encode: bool,
    input: &[u8],
    output: &mut [u8],
    fragment: usize,
) -> Result<usize, ()> {
    let mut read = 0;
    let mut written = 0;
    macro_rules! drive {
        ($state:expr) => {{
            let mut state = $state;
            loop {
                let end = (read + fragment).min(input.len());
                let out_end = (written + fragment).min(output.len());
                let step = if read == input.len() {
                    state.finish(&mut output[written..out_end])
                } else {
                    state.update(&input[read..end], &mut output[written..out_end])
                }
                .map_err(|_| ())?;
                read += step.progress().input_consumed();
                written += step.progress().output_produced();
                if step.status() == base64_ng::Status::Complete {
                    break;
                }
                if step.progress().input_consumed() == 0 && step.progress().output_produced() == 0 {
                    return Err(());
                }
            }
        }};
    }
    if encode {
        drive!(codec.encoder());
    } else {
        drive!(codec.decoder());
    }
    Ok(written)
}
