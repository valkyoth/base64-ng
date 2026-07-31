#[cfg(test)]
mod tests {
    use std::io::Write;

    use base64_ng::{
        Base64, Codec, CodecSettings, DecodeError, DecodedArray, EncodeError, EncodedArray,
        Engine, LineEnding, LineWrap, MIME, Profile, SecretArray, SecretBuffer,
        StrictStandardPadded, STANDARD, STRICT_STANDARD_PADDED, Standard,
    };

    const CONST_ENCODED: [u8; 8] = match STRICT_STANDARD_PADDED.encode_array(b"hello") {
        Ok(output) => output,
        Err(_) => panic!("reviewed const encode failed"),
    };
    const CONST_DECODED: [u8; 5] = match STRICT_STANDARD_PADDED.decode_array(&CONST_ENCODED) {
        Ok(output) => output,
        Err(_) => panic!("reviewed const decode failed"),
    };

    fn encode_into(input: &[u8], output: &mut [u8]) -> Result<usize, EncodeError> {
        STANDARD.encode_slice(input, output)
    }

    fn decode_into(input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        STANDARD.decode_slice_clear_tail(input, output)
    }

    fn encode_to_string(input: &[u8]) -> Result<String, EncodeError> {
        STANDARD.encode_string(input)
    }

    fn encode_to_string_infallible(input: &[u8]) -> String {
        STANDARD.encode_string_infallible(input)
    }

    fn decode_to_vec(input: &[u8]) -> Result<Vec<u8>, DecodeError> {
        STANDARD.decode_vec(input)
    }

    fn encode_in_place(
        buffer: &mut [u8],
        input_len: usize,
    ) -> Result<&mut [u8], EncodeError> {
        STANDARD.encode_in_place(buffer, input_len)
    }

    fn decode_in_place(buffer: &mut [u8]) -> Result<&mut [u8], DecodeError> {
        STANDARD.decode_in_place_clear_tail(buffer)
    }

    fn encoder<W: Write>(writer: W) -> base64_ng::stream::Encoder<W, Standard, true> {
        STANDARD.encoder_writer(writer)
    }

    fn decoder<W: Write>(writer: W) -> base64_ng::stream::Decoder<W, Standard, true> {
        STANDARD.decoder_writer(writer)
    }

    fn encode_redacted(input: &[u8]) -> Result<SecretBuffer, EncodeError> {
        STANDARD.encode_secret(input)
    }

    fn decode_redacted(input: &[u8]) -> Result<SecretBuffer, DecodeError> {
        base64_ng::ct::STANDARD.decode_secret(input)
    }

    fn generic_wrapping_policy() -> LineWrap {
        LineWrap::new(64, LineEnding::Lf)
    }

    fn strict_standard_spec_bridge() -> Profile<Standard, true> {
        Profile::from(Engine::new())
    }

    fn verify_redacted(left: &[u8], right: &[u8]) -> bool {
        base64_ng::constant_time_eq(left, right)
    }

    fn wipe_redacted(bytes: &mut [u8]) {
        base64_ng::secure_wipe(bytes);
    }

    fn codec_settings(codec: &dyn Codec) -> CodecSettings {
        codec.settings()
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn canonical_2_0_names_have_compilable_1_x_migrations() {
        let mut encoded = [0u8; 8];
        assert_eq!(encode_into(b"hello", &mut encoded).unwrap(), 8);
        assert_eq!(encode_to_string(b"hello").unwrap(), "aGVsbG8=");
        assert_eq!(encode_to_string_infallible(b"hello"), "aGVsbG8=");

        let mut decoded = [0u8; 5];
        assert_eq!(decode_into(&encoded, &mut decoded).unwrap(), 5);
        assert_eq!(decode_to_vec(&encoded).unwrap(), b"hello");

        let mut in_place_encode = *b"hello\0\0\0";
        assert_eq!(
            encode_in_place(&mut in_place_encode, 5).unwrap(),
            b"aGVsbG8="
        );

        let mut in_place_decode = *b"aGVsbG8=";
        assert_eq!(decode_in_place(&mut in_place_decode).unwrap(), b"hello");

        let mut encoder = encoder(Vec::new());
        encoder.write_all(b"hello").unwrap();
        assert_eq!(encoder.finish().unwrap(), b"aGVsbG8=");

        let mut decoder = decoder(Vec::new());
        decoder.write_all(b"aGVsbG8=").unwrap();
        assert_eq!(decoder.finish().unwrap(), b"hello");

        assert_eq!(encode_redacted(b"hello").unwrap().expose_secret(), b"aGVsbG8=");
        assert_eq!(decode_redacted(b"aGVsbG8=").unwrap().expose_secret(), b"hello");

        assert_eq!(generic_wrapping_policy().line_len(), 64);
        assert!(strict_standard_spec_bridge().validate(b"aGVsbG8="));
        assert!(MIME.validate(b"aGVsbG8="));
        assert!(verify_redacted(b"hello", b"hello"));

        let mut secret = *b"hello";
        wipe_redacted(&mut secret);
        assert_eq!(secret, [0; 5]);
    }

    #[test]
    fn transactional_2_0_surface_is_public_and_external() {
        let codec: Base64<StrictStandardPadded> = STRICT_STANDARD_PADDED;
        assert_send_sync::<Base64<StrictStandardPadded>>();
        assert_eq!(codec_settings(codec.specification()), codec.settings());

        let mut encoded = [0xa5; 12];
        let written = codec.encode_into(b"hello", &mut encoded).unwrap();
        assert_eq!(&encoded[..written], b"aGVsbG8=");
        assert_eq!(&encoded[written..], &[0xa5; 4]);

        let before = encoded;
        assert!(codec.decode_into(b"!!!!", &mut encoded).is_err());
        assert_eq!(encoded, before);

        assert_eq!(codec.encode_to_string(b"hello").unwrap(), "aGVsbG8=");
        assert_eq!(codec.decode_to_vec(b"aGVsbG8=").unwrap(), b"hello");
    }

    #[test]
    fn const_and_bounded_2_0_surface_is_public_and_external() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<EncodedArray<16>>();
        assert_copy::<DecodedArray<16>>();
        assert!(!core::mem::needs_drop::<EncodedArray<16>>());
        assert!(core::mem::needs_drop::<SecretArray<16>>());
        assert_eq!(CONST_ENCODED, *b"aGVsbG8=");
        assert_eq!(CONST_DECODED, *b"hello");

        let encoded = STRICT_STANDARD_PADDED
            .encode_bounded::<16>(b"hello")
            .unwrap();
        assert_eq!(encoded.as_bytes(), b"aGVsbG8=");
        let decoded = STRICT_STANDARD_PADDED
            .decode_bounded::<16>(encoded.as_bytes())
            .unwrap();
        assert_eq!(decoded.as_bytes(), b"hello");

        let secret = SecretArray::<8>::from_array(*b"keyxxxxx", 3).unwrap();
        assert_eq!(secret.expose_secret(), b"key");
        assert_eq!(format!("{secret}"), "<redacted secret array>");
    }

    #[test]
    fn in_place_2_0_surface_is_public_and_external() {
        let mut ordinary = [0u8; 16];
        ordinary[..6].copy_from_slice(b"secret");
        let encoded_len = STRICT_STANDARD_PADDED
            .encode_in_place(&mut ordinary, 6)
            .unwrap();
        assert_eq!(&ordinary[..encoded_len], b"c2VjcmV0");
        let decoded_len = STRICT_STANDARD_PADDED
            .decode_in_place(&mut ordinary, encoded_len)
            .unwrap();
        assert_eq!(&ordinary[..decoded_len], b"secret");

        ordinary[..8].copy_from_slice(b"c2VjcmV0");
        let mut staging = [0xa5; 8];
        let decoded_len = STRICT_STANDARD_PADDED
            .decode_in_place_staged(&mut ordinary, 8, &mut staging)
            .unwrap();
        assert_eq!(&ordinary[..decoded_len], b"secret");
        assert!(staging.iter().all(|byte| *byte == 0));
    }
}
