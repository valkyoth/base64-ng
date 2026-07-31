#[allow(dead_code, unexpected_cfgs)]
#[path = "../../src/v2/alphabet.rs"]
mod alphabet;
#[allow(dead_code)]
#[path = "../../src/v2/specifications.rs"]
mod specifications;

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use base64::alphabet::Alphabet;
    use base64::engine::DecodePaddingMode;
    use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
    use base64_ng::{CodecSettings as PublicCodecSettings, compat};

    use super::specifications::{
        CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
        STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, TrailingBits,
    };

    const STANDARD_TABLE: [u8; 64] =
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const CUSTOM_TABLE: [u8; 64] =
        *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    #[test]
    fn accepted_runtime_policies_match_pinned_base64_configuration() {
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
                    let Ok(codec) = CodecBuilder::from_table(STANDARD_TABLE)
                        .unwrap()
                        .encode_padding(encode)
                        .decode_padding(decode)
                        .trailing_bits(trailing)
                        .build()
                    else {
                        continue;
                    };
                    let settings = codec.settings();
                    let external = external_engine(settings);
                    let encoded = external.encode(b"f");
                    assert_eq!(encoded.ends_with('='), encode == EncodePadding::Padded);

                    assert_decode(&external, b"Zg==", accepts_padded(decode));
                    assert_decode(&external, b"Zg", accepts_unpadded(decode));
                    assert_decode(&external, b"Zg=", decode == DecodePadding::Indifferent);
                    assert_decode(
                        &external,
                        b"Zh==",
                        accepts_padded(decode)
                            && trailing == TrailingBits::AllowNonCanonical,
                    );
                    assert_decode(
                        &external,
                        b"Zh",
                        accepts_unpadded(decode)
                            && trailing == TrailingBits::AllowNonCanonical,
                    );
                    for malformed in [
                        &b"Z"[..],
                        &b"Zg==="[..],
                        &b"Z!=="[..],
                        &b"Zg==\n"[..],
                    ] {
                        assert!(
                            external.decode(malformed).is_err(),
                            "accepted malformed {malformed:?} under {settings:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn runtime_custom_alphabet_mapping_matches_pinned_base64() {
        let codec = CodecBuilder::from_table(CUSTOM_TABLE)
            .unwrap()
            .build()
            .unwrap();
        let external = external_engine(codec.settings());

        let encoded = external.encode([0xfb, 0xff]);
        assert_eq!(
            encoded.as_bytes(),
            &[CUSTOM_TABLE[62], CUSTOM_TABLE[63], CUSTOM_TABLE[60], b'=']
        );
    }

    #[test]
    fn strict_presets_reject_every_relaxed_only_case() {
        for settings in [
            STRICT_STANDARD_PADDED.settings(),
            STRICT_STANDARD_UNPADDED.settings(),
            STRICT_URL_SAFE_PADDED.settings(),
            STRICT_URL_SAFE_UNPADDED.settings(),
        ] {
            let external = external_engine(settings);
            for whitespace in [0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x20] {
                for position in 0..=4 {
                    let mut input = b"Zg==".to_vec();
                    input.insert(position, whitespace);
                    assert!(external.decode(input).is_err());
                }
            }
            for malformed in [
                &b"Z"[..],
                &b"Zh"[..],
                &b"Zh=="[..],
                &b"=Zg="[..],
                &b"Z=g="[..],
                &b"Zg="[..],
                &b"Zg==="[..],
                &b"Zg==A"[..],
            ] {
                assert!(external.decode(malformed).is_err());
            }
        }
        assert!(
            external_engine(STRICT_STANDARD_PADDED.settings())
                .decode(b"AA_A")
                .is_err()
        );
        assert!(
            external_engine(STRICT_URL_SAFE_PADDED.settings())
                .decode(b"AA/A")
                .is_err()
        );
    }

    #[test]
    fn strict_presets_exhaust_short_padding_counts_and_positions() {
        for settings in [
            STRICT_STANDARD_PADDED.settings(),
            STRICT_STANDARD_UNPADDED.settings(),
            STRICT_URL_SAFE_PADDED.settings(),
            STRICT_URL_SAFE_UNPADDED.settings(),
        ] {
            let external = external_engine(settings);
            let alphabet_byte = settings.alphabet().as_array()[0];
            for len in 1..=8 {
                for padding_mask in 0usize..(1usize << len) {
                    let mut input = vec![alphabet_byte; len];
                    for (index, byte) in input.iter_mut().enumerate() {
                        if padding_mask & (1usize << index) != 0 {
                            *byte = b'=';
                        }
                    }
                    let expected = match settings.decode_padding() {
                        DecodePadding::RequireCanonical => {
                            let padding = input.iter().rev().take_while(|byte| **byte == b'=').count();
                            len % 4 == 0
                                && padding <= 2
                                && padding < len
                                && input[..len - padding].iter().all(|byte| *byte != b'=')
                        }
                        DecodePadding::Forbid => {
                            !input.contains(&b'=') && len % 4 != 1
                        }
                        DecodePadding::Indifferent => unreachable!("strict preset is indifferent"),
                    };
                    assert_eq!(
                        external.decode(&input).is_ok(),
                        expected,
                        "{settings:?} accepted/rejected {input:?} unexpectedly"
                    );
                }
            }
        }
    }

    #[test]
    fn strict_presets_exhaust_every_unused_trailing_bit_value() {
        for strict in [
            STRICT_STANDARD_PADDED.settings(),
            STRICT_STANDARD_UNPADDED.settings(),
            STRICT_URL_SAFE_PADDED.settings(),
            STRICT_URL_SAFE_UNPADDED.settings(),
        ] {
            let strict_engine = external_engine(strict);
            let relaxed = CodecBuilder::new(*strict.alphabet())
                .encode_padding(strict.encode_padding())
                .decode_padding(strict.decode_padding())
                .trailing_bits(TrailingBits::AllowNonCanonical)
                .build()
                .unwrap();
            let relaxed_engine = external_engine(relaxed.settings());
            let table = strict.alphabet().as_array();

            for second in 0..64 {
                let padded = [table[0], table[second], b'=', b'='];
                let unpadded = &padded[..2];
                let input = if strict.decode_padding() == DecodePadding::Forbid {
                    unpadded
                } else {
                    &padded
                };
                assert_eq!(strict_engine.decode(input).is_ok(), second & 15 == 0);
                assert!(relaxed_engine.decode(input).is_ok());
            }

            for third in 0..64 {
                let padded = [table[0], table[0], table[third], b'='];
                let unpadded = &padded[..3];
                let input = if strict.decode_padding() == DecodePadding::Forbid {
                    unpadded
                } else {
                    &padded
                };
                assert_eq!(strict_engine.decode(input).is_ok(), third & 3 == 0);
                assert!(relaxed_engine.decode(input).is_ok());
            }
        }
    }

    #[test]
    fn named_compatibility_presets_match_pinned_base64() {
        let presets = [
            compat::STANDARD_PADDED_PADDING_INDIFFERENT,
            compat::STANDARD_UNPADDED_PADDING_INDIFFERENT,
            compat::STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
            compat::STANDARD_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
            compat::STANDARD_PADDED_FULL_COMPATIBILITY,
            compat::STANDARD_UNPADDED_FULL_COMPATIBILITY,
            compat::URL_SAFE_PADDED_PADDING_INDIFFERENT,
            compat::URL_SAFE_UNPADDED_PADDING_INDIFFERENT,
            compat::URL_SAFE_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
            compat::URL_SAFE_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
            compat::URL_SAFE_PADDED_FULL_COMPATIBILITY,
            compat::URL_SAFE_UNPADDED_FULL_COMPATIBILITY,
        ];
        let corpus = [
            &b""[..],
            &b"Z"[..],
            &b"Zg"[..],
            &b"Zh"[..],
            &b"Zg="[..],
            &b"Zh="[..],
            &b"Zg=="[..],
            &b"Zh=="[..],
            &b"Zg==="[..],
            &b"Zm8"[..],
            &b"Zm9"[..],
            &b"Zm8="[..],
            &b"Zm9="[..],
            &b"AA-A"[..],
            &b"AA_A"[..],
            &b"AA+A"[..],
            &b"AA/A"[..],
        ];

        for codec in presets {
            let external = external_public_engine(codec.settings());
            for input in corpus {
                assert_eq!(
                    codec.decode_to_vec(input).ok(),
                    external.decode(input).ok(),
                    "settings={:?}, input={input:?}",
                    codec.settings()
                );
            }
        }
    }

    fn external_engine(settings: super::specifications::CodecSettings) -> GeneralPurpose {
        let table = core::str::from_utf8(settings.alphabet().as_array()).unwrap();
        let alphabet = Alphabet::new(table).unwrap();
        let config = GeneralPurposeConfig::new()
            .with_encode_padding(settings.encode_padding() == EncodePadding::Padded)
            .with_decode_padding_mode(match settings.decode_padding() {
                DecodePadding::RequireCanonical => DecodePaddingMode::RequireCanonical,
                DecodePadding::Forbid => DecodePaddingMode::RequireNone,
                DecodePadding::Indifferent => DecodePaddingMode::Indifferent,
            })
            .with_decode_allow_trailing_bits(
                settings.trailing_bits() == TrailingBits::AllowNonCanonical,
            );
        GeneralPurpose::new(&alphabet, config)
    }

    fn external_public_engine(settings: PublicCodecSettings) -> GeneralPurpose {
        let table = core::str::from_utf8(settings.alphabet().as_array()).unwrap();
        let alphabet = Alphabet::new(table).unwrap();
        let config = GeneralPurposeConfig::new()
            .with_encode_padding(settings.encode_padding() == base64_ng::EncodePadding::Padded)
            .with_decode_padding_mode(match settings.decode_padding() {
                base64_ng::DecodePadding::RequireCanonical => {
                    DecodePaddingMode::RequireCanonical
                }
                base64_ng::DecodePadding::Forbid => DecodePaddingMode::RequireNone,
                base64_ng::DecodePadding::Indifferent => DecodePaddingMode::Indifferent,
            })
            .with_decode_allow_trailing_bits(
                settings.trailing_bits() == base64_ng::TrailingBits::AllowNonCanonical,
            );
        GeneralPurpose::new(&alphabet, config)
    }

    fn accepts_padded(policy: DecodePadding) -> bool {
        matches!(
            policy,
            DecodePadding::RequireCanonical | DecodePadding::Indifferent
        )
    }

    fn accepts_unpadded(policy: DecodePadding) -> bool {
        matches!(policy, DecodePadding::Forbid | DecodePadding::Indifferent)
    }

    fn assert_decode(engine: &GeneralPurpose, input: &[u8], accepted: bool) {
        assert_eq!(engine.decode(input).is_ok(), accepted, "{input:?}");
    }
}
