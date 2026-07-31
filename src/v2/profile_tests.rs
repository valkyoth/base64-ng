use super::{
    BCRYPT_ALPHABET_NO_PAD, BINHEX_ALPHABET, BodyLineEnding, CRYPT_ALPHABET_NO_PAD,
    IMAP_MUTF7_ALPHABET_NO_PAD, MIME_BODY_STRICT, PBKDF2_ALPHABET_NO_PAD, PEM_BODY_CRLF,
    PEM_BODY_LF, STRICT_STANDARD_PADDED,
};
use crate::{BCRYPT_NO_PAD, CRYPT_NO_PAD, MIME, PEM, PEM_CRLF};

#[test]
fn named_alphabet_codecs_match_their_exact_tables() {
    assert_eq!(
        BCRYPT_ALPHABET_NO_PAD.settings().alphabet().as_array(),
        b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    );
    assert_eq!(
        CRYPT_ALPHABET_NO_PAD.settings().alphabet().as_array(),
        b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
    );
    assert_eq!(
        PBKDF2_ALPHABET_NO_PAD.settings().alphabet().as_array(),
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./"
    );
    assert_eq!(
        BINHEX_ALPHABET.as_array(),
        b"!\"#$%&'()*+,-012345689@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdefhijklmpqr"
    );
    assert_eq!(
        IMAP_MUTF7_ALPHABET_NO_PAD.settings().alphabet().as_array(),
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,"
    );
}

#[test]
fn renamed_bcrypt_and_crypt_codecs_match_legacy_engines() {
    let mut input = [0u8; 96];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap()
            .wrapping_mul(73)
            .wrapping_add(19);
    }
    for len in 0..=input.len() {
        assert_codec_matches_legacy(&BCRYPT_ALPHABET_NO_PAD, BCRYPT_NO_PAD, &input[..len]);
        assert_codec_matches_legacy(&CRYPT_ALPHABET_NO_PAD, CRYPT_NO_PAD, &input[..len]);
    }
}

#[test]
fn body_names_match_legacy_body_layout_without_claiming_containers() {
    assert_eq!(MIME_BODY_STRICT.codec(), &STRICT_STANDARD_PADDED);
    assert_eq!(MIME_BODY_STRICT.wrapping().line_width().get(), 76);
    assert_eq!(
        MIME_BODY_STRICT.wrapping().line_ending(),
        BodyLineEnding::CrLf
    );
    assert_eq!(PEM_BODY_LF.wrapping().line_width().get(), 64);
    assert_eq!(PEM_BODY_LF.wrapping().line_ending(), BodyLineEnding::Lf);
    assert_eq!(PEM_BODY_CRLF.wrapping().line_width().get(), 64);
    assert_eq!(PEM_BODY_CRLF.wrapping().line_ending(), BodyLineEnding::CrLf);

    let input = [0x5au8; 96];
    for (body, legacy) in [
        (&MIME_BODY_STRICT, &MIME),
        (&PEM_BODY_LF, &PEM),
        (&PEM_BODY_CRLF, &PEM_CRLF),
    ] {
        let mut encoded = [0u8; 128];
        let encoded_len = body.codec().encode_into(&input, &mut encoded).unwrap();
        let mut actual = [0u8; 192];
        let actual_len = body
            .wrapping()
            .insert_into(&encoded[..encoded_len], &mut actual)
            .unwrap();
        let mut expected = [0u8; 192];
        let expected_len = legacy.encode_slice(&input, &mut expected).unwrap();
        assert_eq!(&actual[..actual_len], &expected[..expected_len]);
    }
}

#[test]
fn imap_alphabet_level_example_is_not_a_modified_utf7_transform() {
    let mut output = [0u8; 8];
    let written = IMAP_MUTF7_ALPHABET_NO_PAD
        .encode_into(b"\xfb\xff", &mut output)
        .unwrap();
    assert_eq!(&output[..written], b"+,8");
}

fn assert_codec_matches_legacy<A: crate::Alphabet>(
    codec: &super::Base64<super::RuntimeSpec>,
    legacy: crate::Engine<A, false>,
    input: &[u8],
) {
    let mut expected = [0u8; 128];
    let expected_len = legacy.encode_slice(input, &mut expected).unwrap();
    let mut actual = [0u8; 128];
    let actual_len = codec.encode_into(input, &mut actual).unwrap();
    assert_eq!(&actual[..actual_len], &expected[..expected_len]);

    let mut decoded = [0u8; 96];
    assert_eq!(
        codec
            .decode_into(&actual[..actual_len], &mut decoded)
            .unwrap(),
        input.len()
    );
    assert_eq!(&decoded[..input.len()], input);
}
