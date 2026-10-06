//! Private experiment, not a public contract or a production decoding route.
use super::*;
use crate::runtime::{Backend, OperationKind};

const BLOCK: usize = 1024;
const DECODED: usize = BLOCK / 4 * 3;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Progress {
    consumed: usize,
    written: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Failure {
    Unsupported,
    Input(OneShotError),
    OutputFull { minimum: usize },
    Backend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rejected {
    progress: Progress,
    failure: Failure,
}

// An injected kernel never changes process-wide health in correctness tests.
trait Kernel {
    fn decode(&mut self, family: Family, input: &[u8], output: &mut [u8]) -> Option<bool>;
    fn quarantine(&mut self);
}

struct Native(Option<Backend>);

impl Kernel for Native {
    fn decode(&mut self, family: Family, input: &[u8], output: &mut [u8]) -> Option<bool> {
        let backend = self.0?;
        if !crate::v2::backend_health::admit(OperationKind::StrictDecode, backend) {
            return None;
        }
        Some(crate::simd::ordinary::decode(
            backend,
            input,
            output,
            family == Family::UrlSafe,
        ))
    }

    fn quarantine(&mut self) {
        if let Some(backend) = self.0.take() {
            vector::quarantine(backend, BackendFault::OutputMismatch);
        }
    }
}

fn scalar(family: Family, input: &[u8], output: &mut [u8]) -> Result<usize, crate::DecodeError> {
    match family {
        Family::Standard => crate::scalar::decode_slice::<crate::Standard, false>(input, output),
        Family::UrlSafe => crate::scalar::decode_slice::<crate::UrlSafe, false>(input, output),
    }
}

fn invalid(settings: CodecSettings, input: &[u8]) -> Failure {
    // Malformed input alone may incur a full reference scan for exact absolute
    // diagnostics. It runs at most once and cannot change committed output.
    match validate_and_measure(settings, input) {
        Err(error) => Failure::Input(error),
        Ok(_) => Failure::Backend,
    }
}

fn stage<const CHECKED: bool>(
    kernel: &mut impl Kernel,
    family: Family,
    input: &[u8],
    staged: &mut [u8; DECODED],
) -> Result<(), Failure> {
    let accelerated = kernel.decode(family, input, staged);
    if !CHECKED && accelerated == Some(true) {
        return Ok(());
    }
    let mut reference = [0; DECODED];
    let result = scalar(family, input, &mut reference);
    let result = match (accelerated, result) {
        (None, Ok(DECODED)) => {
            staged.copy_from_slice(&reference);
            Ok(())
        }
        (Some(true), Ok(DECODED)) if staged == &reference => Ok(()),
        (None | Some(false), Err(_)) => Err(Failure::Input(OneShotError::Input(
            crate::InputError::InvalidLength,
        ))),
        _ => {
            kernel.quarantine();
            Err(Failure::Backend)
        }
    };
    crate::wipe_bytes(&mut reference);
    result
}

fn decode_with<const CHECKED: bool>(
    settings: CodecSettings,
    input: &[u8],
    output: &mut [u8],
    kernel: &mut impl Kernel,
) -> Result<Progress, Rejected> {
    let mut progress = Progress::default();
    let Some(family) = Family::for_settings(settings) else {
        return Err(Rejected {
            progress,
            failure: Failure::Unsupported,
        });
    };
    // Reserve at least the final quantum for the existing canonical tail rules.
    while input.len() - progress.consumed > BLOCK {
        if output.len() - progress.written < DECODED {
            return Err(Rejected {
                progress,
                failure: Failure::OutputFull { minimum: DECODED },
            });
        }
        let mut staged = [0; DECODED];
        let result = stage::<CHECKED>(
            kernel,
            family,
            &input[progress.consumed..progress.consumed + BLOCK],
            &mut staged,
        );
        if result.is_ok() {
            output[progress.written..progress.written + DECODED].copy_from_slice(&staged);
        }
        // Ordinary scratch has no secret-storage or automatic-wipe guarantee.
        if let Err(failure) = result {
            let failure = if matches!(failure, Failure::Input(_)) {
                invalid(settings, input)
            } else {
                failure
            };
            return Err(Rejected { progress, failure });
        }
        progress.consumed += BLOCK;
        progress.written += DECODED;
    }
    let tail = &input[progress.consumed..];
    let proof = prepare(settings, tail, DecodeValidation::Auto).map_err(|_| Rejected {
        progress,
        failure: invalid(settings, input),
    })?;
    let minimum = proof.len();
    if output.len() - progress.written < minimum {
        return Err(Rejected {
            progress,
            failure: Failure::OutputFull { minimum },
        });
    }
    let written = write(proof, &mut output[progress.written..]).map_err(|_| Rejected {
        progress,
        failure: Failure::Backend,
    })?;
    Ok(Progress {
        consumed: input.len(),
        written: progress.written + written,
    })
}

fn decode(settings: CodecSettings, input: &[u8], output: &mut [u8]) -> Result<Progress, Rejected> {
    decode_with::<{ cfg!(feature = "checked-backend") }>(
        settings,
        input,
        output,
        &mut Native(vector::select(input.len())),
    )
}

#[cfg(test)]
mod benchmark;
#[cfg(test)]
mod tests;
