#![no_main]

use base64::{Engine as _, engine::general_purpose};
use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    compare_canonical(data, STANDARD, &general_purpose::STANDARD);
    compare_canonical(data, STANDARD_NO_PAD, &general_purpose::STANDARD_NO_PAD);
    compare_canonical(data, URL_SAFE, &general_purpose::URL_SAFE);
    compare_canonical(data, URL_SAFE_NO_PAD, &general_purpose::URL_SAFE_NO_PAD);
});

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
