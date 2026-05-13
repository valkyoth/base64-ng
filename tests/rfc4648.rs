use base64_ng::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD, decoded_capacity};

#[test]
fn rfc4648_standard_round_trips() {
    let cases: &[&[u8]] = &[
        b"",
        b"f",
        b"fo",
        b"foo",
        b"foob",
        b"fooba",
        b"foobar",
        b"The quick brown fox jumps over the lazy dog",
    ];

    for case in cases {
        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD.encode_slice(case, &mut encoded).unwrap();
        let mut decoded = [0u8; 128];
        let decoded_len = STANDARD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], *case);
    }
}

#[test]
fn unpadded_round_trips() {
    for input_len in 0..64 {
        let mut input = [0u8; 64];
        for (index, byte) in input.iter_mut().enumerate() {
            *byte = (index * 37 + input_len) as u8;
        }

        let mut encoded = [0u8; 128];
        let encoded_len = STANDARD_NO_PAD
            .encode_slice(&input[..input_len], &mut encoded)
            .unwrap();
        assert!(!encoded[..encoded_len].contains(&b'='));

        let mut decoded = [0u8; 64];
        let decoded_len = STANDARD_NO_PAD
            .decode_slice(&encoded[..encoded_len], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], &input[..input_len]);
    }
}

#[test]
fn url_safe_alphabet_is_distinct() {
    let mut padded = [0u8; 8];
    let padded_len = URL_SAFE.encode_slice(b"\xfb\xff", &mut padded).unwrap();
    assert_eq!(&padded[..padded_len], b"-_8=");

    let mut unpadded = [0u8; 8];
    let unpadded_len = URL_SAFE_NO_PAD
        .encode_slice(b"\xfb\xff", &mut unpadded)
        .unwrap();
    assert_eq!(&unpadded[..unpadded_len], b"-_8");
}

#[test]
fn decoded_capacity_is_upper_bound() {
    for encoded_len in 0..128 {
        assert!(decoded_capacity(encoded_len) <= encoded_len / 4 * 3 + 2);
    }
}
