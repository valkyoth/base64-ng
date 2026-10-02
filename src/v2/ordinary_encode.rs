//! Exact-family ordinary encoding through the existing admitted dispatch.

use super::specifications::{CodecSettings, EncodePadding};
use crate::{Alphabet, BackendFault, Standard, UrlSafe, encode_backend};

// The caller has checked the exact output geometry before entering this path.
// Rejected writes are completely overwritten by its infallible table writer.
pub(super) fn write(settings: CodecSettings, input: &[u8], output: &mut [u8]) -> bool {
    // Canonical table encoding is cheaper than dispatch for short messages.
    if input.len() < 192 {
        return false;
    }
    let alphabet = settings.alphabet().as_array();
    let url = if alphabet == &Standard::ENCODE {
        false
    } else if alphabet == &UrlSafe::ENCODE {
        true
    } else {
        return false;
    };
    let backend = encode_backend::active_encode_backend_for_input(input.len());
    if backend == encode_backend::EncodeBackend::Scalar {
        return false;
    }
    // Decode padding and trailing-bit acceptance do not determine encoding.
    let padded = settings.encode_padding() == EncodePadding::Padded;
    let result = match (url, padded) {
        (false, true) => encode_backend::encode_selected::<Standard, true>(backend, input, output),
        (false, false) => {
            encode_backend::encode_selected::<Standard, false>(backend, input, output)
        }
        (true, true) => encode_backend::encode_selected::<UrlSafe, true>(backend, input, output),
        (true, false) => encode_backend::encode_selected::<UrlSafe, false>(backend, input, output),
    };
    #[cfg(test)]
    let result = tests::result(result, output);
    if result == Ok(output.len()) {
        return true;
    }
    #[cfg(test)]
    if tests::quarantine(backend) {
        return false;
    }
    super::backend_health::quarantine(
        crate::runtime::OperationKind::Encode,
        backend.reported(),
        BackendFault::ImpossibleState,
    );
    false
}

#[cfg(test)]
mod tests;
