#[test]
fn companion_round_trip_and_rejection() {
    use companion::Base64BytesExt;
    let codec = base64_ng::STRICT_STANDARD_PADDED;
    let plain = payload();
    let encoded = codec.encode_buf(plain.as_slice()).unwrap();
    assert_eq!(&codec.decode_buf(encoded.clone()).unwrap()[..], plain);
    let mut invalid = encoded.to_vec();
    *invalid.last_mut().unwrap() = b'!';
    assert!(codec.decode_buf(invalid.as_slice()).is_err());
    secret_boundary();
}
