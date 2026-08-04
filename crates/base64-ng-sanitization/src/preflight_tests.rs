use crate::{CtDecodeSanitizationExt, SanitizationDecodeError};
use base64_ng::ct;

#[test]
fn fixed_secret_bytes_reject_oversized_input_before_validation() {
    assert!(matches!(
        ct::STANDARD.decode_secret_bytes::<5>(b"!!!!!!!!!!!!"),
        Err(SanitizationDecodeError::EncodedInputLimit {
            maximum: 8,
            actual: 12
        })
    ));
}
