//! Ordinary data only: these buffers are not constant-time or secret storage.
use base64_ng::{Base64Ref, DecodeValidation, DecodeValidator, STRICT_STANDARD_PADDED};

fn main() {
    let codec = STRICT_STANDARD_PADDED;
    let encoded = b"aGVsbG8=";
    let mut output = [0xa5; 8];
    let (written, report) = codec
        .decode_into_with_report(encoded, &mut output, DecodeValidation::ScalarReference)
        .unwrap();
    assert_eq!(&output[..written], b"hello");
    assert_eq!(&output[written..], &[0xa5; 3]);
    assert_eq!(
        report.requested_validation(),
        DecodeValidation::ScalarReference
    );
    assert_eq!(report.validator(), DecodeValidator::ScalarReference);

    // Auto retains grammar validation, not a permanent backend authorization.
    let view = Base64Ref::parse(codec, encoded).unwrap();
    assert_eq!(view.decoded_len(), 5);
    for _ in 0..2 {
        assert_eq!(view.decode_into(&mut output).unwrap(), 5);
        assert_eq!(&output[..5], b"hello");
    }
    let before = output;
    assert!(codec.decode_into(b"!!!!", &mut output).is_err());
    assert_eq!(output, before);
}
