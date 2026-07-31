use super::{
    alphabet::ValidatedAlphabet,
    specifications::{
        Base64, Codec, CodecBuilder, CodecBuilderError, DecodePadding, EncodePadding, RuntimeSpec,
        STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
        STRICT_URL_SAFE_UNPADDED, TrailingBits,
    },
};
use crate::{Alphabet, Engine, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

const STANDARD_TABLE: [u8; 64] =
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const CUSTOM_TABLE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

#[test]
fn strict_presets_match_legacy_rfc4648_vectors() {
    let vectors: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"f", b"Zg=="),
        (b"fo", b"Zm8="),
        (b"foo", b"Zm9v"),
        (b"foob", b"Zm9vYg=="),
        (b"fooba", b"Zm9vYmE="),
        (b"foobar", b"Zm9vYmFy"),
    ];
    assert_strict_preset(STRICT_STANDARD_PADDED.settings(), STANDARD, vectors);
    assert_strict_preset(
        STRICT_STANDARD_UNPADDED.settings(),
        STANDARD_NO_PAD,
        vectors,
    );
    assert_strict_preset(STRICT_URL_SAFE_PADDED.settings(), URL_SAFE, vectors);
    assert_strict_preset(
        STRICT_URL_SAFE_UNPADDED.settings(),
        URL_SAFE_NO_PAD,
        vectors,
    );
}

fn assert_strict_preset<A: Alphabet, const PAD: bool>(
    settings: super::specifications::CodecSettings,
    legacy: Engine<A, PAD>,
    vectors: &[(&[u8], &[u8])],
) {
    assert_eq!(settings.alphabet().as_array(), &A::ENCODE);
    assert_eq!(
        settings.encode_padding(),
        if PAD {
            EncodePadding::Padded
        } else {
            EncodePadding::Unpadded
        }
    );
    assert_eq!(settings.trailing_bits(), TrailingBits::RequireCanonical);
    assert!(settings.permits_secret_processing());
    for &(plain, padded) in vectors {
        let expected_len = if PAD {
            padded.len()
        } else {
            padded
                .iter()
                .position(|byte| *byte == b'=')
                .unwrap_or(padded.len())
        };
        let mut encoded = [0u8; 16];
        let written = legacy.encode_slice(plain, &mut encoded).unwrap();
        assert_eq!(written, expected_len);
        let mut decoded = [0u8; 8];
        let decoded_len = legacy
            .decode_slice(&encoded[..written], &mut decoded)
            .unwrap();
        assert_eq!(&decoded[..decoded_len], plain);
    }
}

#[test]
fn builder_accepts_exactly_self_consistent_policy_combinations() {
    let alphabet = ValidatedAlphabet::new(CUSTOM_TABLE).unwrap();
    let mut accepted = 0;
    let mut rejected = 0;
    for encode in [EncodePadding::Padded, EncodePadding::Unpadded] {
        for decode in [
            DecodePadding::RequireCanonical,
            DecodePadding::Forbid,
            DecodePadding::Indifferent,
        ] {
            for trailing in [
                TrailingBits::RequireCanonical,
                TrailingBits::AllowNonCanonical,
            ] {
                let result = CodecBuilder::new(alphabet)
                    .encode_padding(encode)
                    .decode_padding(decode)
                    .trailing_bits(trailing)
                    .build();
                let expected = match (encode, decode) {
                    (EncodePadding::Padded, DecodePadding::Forbid) => {
                        Err(CodecBuilderError::EncodedPaddingRejected)
                    }
                    (EncodePadding::Unpadded, DecodePadding::RequireCanonical) => {
                        Err(CodecBuilderError::EncodedPaddingRequired)
                    }
                    _ => Ok(()),
                };
                assert_eq!(
                    result.as_ref().map(|_| ()).map_err(|error| *error),
                    expected
                );
                if result.is_ok() {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
            }
        }
    }
    assert_eq!((accepted, rejected), (8, 4));
}

#[test]
fn runtime_codec_owns_alphabet_without_allocation() {
    let source = CUSTOM_TABLE;
    let codec = CodecBuilder::from_slice(&source)
        .unwrap()
        .decode_padding(DecodePadding::Indifferent)
        .build()
        .unwrap();
    let settings = codec.settings();

    assert_eq!(settings.alphabet().as_array(), &CUSTOM_TABLE);
    assert_eq!(settings.decode_padding(), DecodePadding::Indifferent);
    assert!(!settings.permits_secret_processing());
    assert_eq!(core::mem::size_of::<ValidatedAlphabet>(), 64);
    assert!(core::mem::size_of::<Base64<RuntimeSpec>>() <= 72);
}

#[test]
fn consumer_trait_is_object_safe_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    fn settings_through_object(codec: &dyn Codec) -> super::specifications::CodecSettings {
        codec.settings()
    }

    assert_send_sync::<Base64<RuntimeSpec>>();
    assert_eq!(
        core::mem::size_of_val(STRICT_STANDARD_PADDED.specification()),
        0
    );
    assert_eq!(
        settings_through_object(STRICT_STANDARD_PADDED.specification()),
        STRICT_STANDARD_PADDED.settings()
    );
}

#[test]
fn relaxed_modes_never_qualify_for_secret_processing() {
    let alphabet = ValidatedAlphabet::new(STANDARD_TABLE).unwrap();
    for decode in [
        DecodePadding::RequireCanonical,
        DecodePadding::Forbid,
        DecodePadding::Indifferent,
    ] {
        for trailing in [
            TrailingBits::RequireCanonical,
            TrailingBits::AllowNonCanonical,
        ] {
            let encode = if decode == DecodePadding::Forbid {
                EncodePadding::Unpadded
            } else {
                EncodePadding::Padded
            };
            let settings = CodecBuilder::new(alphabet)
                .encode_padding(encode)
                .decode_padding(decode)
                .trailing_bits(trailing)
                .build()
                .unwrap()
                .settings();
            assert_eq!(
                settings.permits_secret_processing(),
                decode != DecodePadding::Indifferent && trailing == TrailingBits::RequireCanonical
            );
        }
    }
}

#[test]
fn const_builder_produces_an_owned_runtime_codec() {
    const CODEC: Base64<RuntimeSpec> = match CodecBuilder::from_table(CUSTOM_TABLE) {
        Ok(builder) => match builder
            .encode_padding(EncodePadding::Unpadded)
            .decode_padding(DecodePadding::Forbid)
            .build()
        {
            Ok(codec) => codec,
            Err(_) => panic!("valid const policies rejected"),
        },
        Err(_) => panic!("valid const alphabet rejected"),
    };

    assert_eq!(CODEC.settings().alphabet().as_array(), &CUSTOM_TABLE);
}
