#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use base64_ng::{
        decode_alphabet_byte, stream::DecoderReader, stream::Encoder, BCRYPT, CRYPT,
        DecodedBuffer, EncodedBuffer, Engine, LineEnding, LineWrap, MIME, PEM, STANDARD,
        STANDARD_NO_PAD, URL_SAFE_NO_PAD,
    };

    struct ReverseAlphabet;

    impl base64_ng::Alphabet for ReverseAlphabet {
        const ENCODE: [u8; 64] =
            *b"/+9876543210zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA";

        fn decode(byte: u8) -> Option<u8> {
            decode_alphabet_byte(byte, &Self::ENCODE)
        }
    }

    const REVERSE: Engine<ReverseAlphabet, true> = Engine::new();

    base64_ng::define_alphabet! {
        struct DotSlashAlphabet = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    }

    const DOT_SLASH_NO_PAD: Engine<DotSlashAlphabet, false> = Engine::new();

    #[test]
    fn strict_standard_migration_surface_round_trips() {
        let encoded = STANDARD.encode_string(b"hello").unwrap();
        assert_eq!(encoded, "aGVsbG8=");

        let decoded = STANDARD.decode_vec(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, b"hello");

        let mut caller_owned = [0u8; 8];
        let written = STANDARD.encode_slice(b"hello", &mut caller_owned).unwrap();
        assert_eq!(&caller_owned[..written], b"aGVsbG8=");
    }

    #[test]
    fn url_safe_no_pad_migration_surface_round_trips() {
        let encoded = URL_SAFE_NO_PAD.encode_string(&[0xfb, 0xff]).unwrap();
        assert_eq!(encoded, "-_8");

        let decoded = URL_SAFE_NO_PAD.decode_vec(encoded.as_bytes()).unwrap();
        assert_eq!(decoded, [0xfb, 0xff]);
    }

    #[test]
    fn wrapped_profiles_migration_surface_round_trips() {
        let input = [0x5a; 58];
        let encoded = MIME.encode_buffer::<82>(&input).unwrap();

        assert!(MIME.validate(encoded.as_bytes()));
        assert!(!PEM.validate(encoded.as_bytes()));

        let decoded = MIME.decode_buffer::<58>(encoded.as_bytes()).unwrap();
        assert_eq!(decoded.as_bytes(), input);

        let custom_wrap = LineWrap::new(4, LineEnding::Lf);
        let wrapped = STANDARD.encode_wrapped_string(b"hello", custom_wrap).unwrap();
        assert_eq!(wrapped, "aGVs\nbG8=");
        assert_eq!(
            STANDARD
                .decode_wrapped_vec(wrapped.as_bytes(), custom_wrap)
                .unwrap(),
            b"hello"
        );
    }

    #[test]
    fn legacy_whitespace_migration_surface_is_explicit() {
        assert!(STANDARD.decode_vec(b" aG\r\nVs\tbG8= ").is_err());

        let decoded = STANDARD.decode_vec_legacy(b" aG\r\nVs\tbG8= ").unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn custom_and_named_nonstandard_profiles_round_trip() {
        let reverse = REVERSE.encode_string(b"hello").unwrap();
        assert_eq!(REVERSE.decode_vec(reverse.as_bytes()).unwrap(), b"hello");

        let dot_slash = DOT_SLASH_NO_PAD.encode_string(b"hello").unwrap();
        assert_eq!(
            DOT_SLASH_NO_PAD.decode_vec(dot_slash.as_bytes()).unwrap(),
            b"hello"
        );

        let bcrypt = BCRYPT.encode_vec(&[0xff, 0xff, 0xff]).unwrap();
        let crypt = CRYPT.encode_vec(&[0xff, 0xff, 0xff]).unwrap();
        assert_ne!(bcrypt, crypt);
        assert_eq!(BCRYPT.decode_vec(&bcrypt).unwrap(), [0xff, 0xff, 0xff]);
        assert_eq!(CRYPT.decode_vec(&crypt).unwrap(), [0xff, 0xff, 0xff]);
    }

    #[test]
    fn stack_and_secret_buffers_keep_security_boundaries_visible() {
        let encoded: EncodedBuffer<8> = STANDARD.encode_buffer(b"hello").unwrap();
        assert_eq!(encoded.as_bytes(), b"aGVsbG8=");

        let decoded: DecodedBuffer<5> = STANDARD.decode_buffer(encoded.as_bytes()).unwrap();
        assert_eq!(decoded.as_bytes(), b"hello");
        assert!(decoded.constant_time_eq_public_len(b"hello"));

        let secret = STANDARD.decode_secret(encoded.as_bytes()).unwrap();
        assert!(secret.constant_time_eq_public_len(b"hello"));
        assert_eq!(
            format!("{secret:?}"),
            r#"SecretBuffer { bytes: "<redacted>", len: 5 }"#
        );
    }

    #[test]
    fn stream_writer_and_reader_migration_surface_round_trips() {
        let mut encoder = Encoder::new(Vec::new(), STANDARD);
        encoder.write_all(b"he").unwrap();
        assert!(encoder.has_pending_input());
        encoder.write_all(b"llo").unwrap();
        assert!(encoder.has_pending_input());
        encoder.try_finish().unwrap();
        assert!(encoder.is_finalized());
        assert_eq!(encoder.finish().unwrap(), b"aGVsbG8=");

        let mut reader = DecoderReader::new(&b"aGVsbG8="[..], STANDARD);
        let mut decoded = Vec::new();
        reader.read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, b"hello");
        assert!(reader.is_finished());
    }

    #[test]
    fn checked_length_helpers_match_migration_examples() {
        assert_eq!(base64_ng::checked_encoded_len(5, true), Some(8));
        assert_eq!(
            base64_ng::checked_wrapped_encoded_len(5, true, LineWrap::new(4, LineEnding::Lf)),
            Some(9)
        );
        assert_eq!(base64_ng::decoded_capacity(8), 6);
        assert_eq!(STANDARD_NO_PAD.encoded_len(5).unwrap(), 7);
    }
}
