use std::vec;

#[test]
fn companion_round_trip_and_rejection() {
    use companion::*;
    let limits = Base64MultibaseLimits::new(32768, 32768, 32768);
    let plain = payload();
    for encoding in [
        Base64MultibaseEncoding::Base64,
        Base64MultibaseEncoding::Base64Pad,
        Base64MultibaseEncoding::Base64Url,
        Base64MultibaseEncoding::Base64UrlPad,
    ] {
        let mut encoded = vec![0; 16384];
        let n = encode_base64_multibase_into(encoding, &plain, &mut encoded, limits).unwrap();
        encoded.truncate(n);
        let mut output = vec![0xa5; plain.len()];
        decode_base64_multibase_into(&encoded, &mut output, limits).unwrap();
        assert_eq!(output, plain);
        *encoded.last_mut().unwrap() = b'!';
        output.fill(0xa5);
        assert!(decode_base64_multibase_into(&encoded, &mut output, limits).is_err());
        assert!(output.iter().all(|&byte| byte == 0xa5));
    }
    secret_boundary();
}
