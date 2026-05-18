#![no_main]

use base64::{Engine as _, engine::general_purpose};
use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    validate_ground_truth_vectors();

    compare_canonical(data, STANDARD, &general_purpose::STANDARD);
    compare_canonical(data, STANDARD_NO_PAD, &general_purpose::STANDARD_NO_PAD);
    compare_canonical(data, URL_SAFE, &general_purpose::URL_SAFE);
    compare_canonical(data, URL_SAFE_NO_PAD, &general_purpose::URL_SAFE_NO_PAD);
});

fn validate_ground_truth_vectors() {
    for (plain, standard, standard_no_pad, url_safe, url_safe_no_pad) in [
        (&b""[..], &b""[..], &b""[..], &b""[..], &b""[..]),
        (b"f", b"Zg==", b"Zg", b"Zg==", b"Zg"),
        (b"fo", b"Zm8=", b"Zm8", b"Zm8=", b"Zm8"),
        (b"foo", b"Zm9v", b"Zm9v", b"Zm9v", b"Zm9v"),
        (b"foob", b"Zm9vYg==", b"Zm9vYg", b"Zm9vYg==", b"Zm9vYg"),
        (b"fooba", b"Zm9vYmE=", b"Zm9vYmE", b"Zm9vYmE=", b"Zm9vYmE"),
        (
            b"foobar",
            b"Zm9vYmFy",
            b"Zm9vYmFy",
            b"Zm9vYmFy",
            b"Zm9vYmFy",
        ),
        (b"\xfb\xff", b"+/8=", b"+/8", b"-_8=", b"-_8"),
    ] {
        assert_ground_truth(plain, STANDARD, &general_purpose::STANDARD, standard);
        assert_ground_truth(
            plain,
            STANDARD_NO_PAD,
            &general_purpose::STANDARD_NO_PAD,
            standard_no_pad,
        );
        assert_ground_truth(plain, URL_SAFE, &general_purpose::URL_SAFE, url_safe);
        assert_ground_truth(
            plain,
            URL_SAFE_NO_PAD,
            &general_purpose::URL_SAFE_NO_PAD,
            url_safe_no_pad,
        );
    }
}

fn assert_ground_truth<A, const PAD: bool>(
    plain: &[u8],
    ours: base64_ng::Engine<A, PAD>,
    reference: &general_purpose::GeneralPurpose,
    encoded: &[u8],
) where
    A: base64_ng::Alphabet,
{
    assert_eq!(ours.encode_vec(plain).unwrap(), encoded);
    assert_eq!(reference.encode(plain).as_bytes(), encoded);
    assert_eq!(ours.decode_vec(encoded).unwrap(), plain);
    assert_eq!(reference.decode(encoded).unwrap(), plain);
}

fn compare_canonical<A, const PAD: bool>(
    input: &[u8],
    ours: base64_ng::Engine<A, PAD>,
    reference: &general_purpose::GeneralPurpose,
) where
    A: base64_ng::Alphabet,
{
    let ours_encoded = ours.encode_vec(input).unwrap();
    let reference_encoded = reference.encode(input);
    assert_eq!(ours_encoded, reference_encoded.as_bytes());

    let ours_decoded = ours.decode_vec(&ours_encoded).unwrap();
    let reference_decoded = reference.decode(&reference_encoded).unwrap();
    assert_eq!(ours_decoded, reference_decoded);
    assert_eq!(ours_decoded, input);
}
