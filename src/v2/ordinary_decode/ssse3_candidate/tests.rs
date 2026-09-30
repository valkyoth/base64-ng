use super::*;
use crate::v2::rfc4648_oracle::{self as oracle, Profile};

#[test]
fn ssse3_validation_candidate_execution_status() {
    let available = crate::simd::ssse3_validation_candidate_available();
    std::println!("SSSE3 validation native execution available: {available}");
    if std::env::var_os("BASE64_NG_REQUIRE_SSSE3_VALIDATION").is_some() {
        assert!(available, "native SSSE3/SSE4.1 execution was required");
    }
}

#[test]
fn unsupported_settings_keep_the_original_fallback() {
    let mut custom = *crate::STRICT_STANDARD_PADDED
        .settings()
        .alphabet()
        .as_array();
    custom.rotate_left(9);
    for alphabet in [
        custom,
        *crate::STRICT_STANDARD_PADDED
            .settings()
            .alphabet()
            .as_array(),
    ] {
        for bits in [
            crate::TrailingBits::RequireCanonical,
            crate::TrailingBits::AllowNonCanonical,
        ] {
            let codec = crate::CodecBuilder::from_table(alphabet)
                .unwrap()
                .encode_padding(crate::EncodePadding::Unpadded)
                .decode_padding(DecodePadding::Indifferent)
                .trailing_bits(bits)
                .build()
                .unwrap();
            for input in [
                b"Zm9vZm9vZm9vZm9vZg==".as_slice(),
                b"Zm9vZm9vZm9vZm9vZh",
                b"!",
            ] {
                for capacity in [0, 3, 24] {
                    compare(codec.settings(), input, capacity);
                }
            }
        }
    }
}

fn profiles() -> [(CodecSettings, Profile); 4] {
    [
        (
            crate::STRICT_STANDARD_PADDED.settings(),
            Profile::StandardPadded,
        ),
        (
            crate::STRICT_STANDARD_UNPADDED.settings(),
            Profile::StandardUnpadded,
        ),
        (
            crate::STRICT_URL_SAFE_PADDED.settings(),
            Profile::UrlSafePadded,
        ),
        (
            crate::STRICT_URL_SAFE_UNPADDED.settings(),
            Profile::UrlSafeUnpadded,
        ),
    ]
}

#[test]
fn all_bytes_in_all_lanes_and_multiple_invalid_lanes() {
    if !crate::simd::ssse3_validation_candidate_available() {
        return;
    }
    for (settings, _) in profiles() {
        let alphabet = settings.alphabet().as_array();
        let url_safe = alphabet[62] == b'-';
        for offset in 0..32 {
            let mut storage = [b'A'; 48];
            let block: &mut [u8; 16] = (&mut storage[offset..offset + 16]).try_into().unwrap();
            for lane in 0..16 {
                for byte in 0..=255 {
                    block[lane] = byte;
                    assert_eq!(
                        crate::simd::candidate_validate_16(block, url_safe),
                        alphabet.contains(&byte)
                    );
                }
                block[lane] = b'A';
            }
            for first in 0..16 {
                for second in 0..16 {
                    block[first] = 0x80;
                    block[second] = b'=';
                    assert!(!crate::simd::candidate_validate_16(block, url_safe));
                    block[first] = b'A';
                    block[second] = b'A';
                }
            }
        }
    }
}

fn compare(settings: CodecSettings, input: &[u8], capacity: usize) {
    let mut candidate = std::vec![0xa5; capacity + 8];
    let mut reference = candidate.clone();
    let expected = prepare(settings, input, DecodeValidation::ScalarReference)
        .and_then(|proof| write(proof, &mut reference[4..4 + capacity]));
    assert_eq!(
        decode(settings, input, &mut candidate[4..4 + capacity]),
        expected
    );
    assert_eq!(candidate, reference);
}

#[test]
fn all_block_boundaries_tails_and_capacities_match_reference_and_oracle() {
    for (settings, profile) in profiles() {
        for length in 0..=160 {
            let plain: std::vec::Vec<_> = (0..length)
                .map(|n| u8::try_from(n % 256).unwrap())
                .collect();
            let input = oracle::encode(profile, &plain);
            for offset in 0..16 {
                let mut storage = std::vec![0xa5; offset];
                storage.extend_from_slice(&input);
                let mut output = std::vec![0xa5; length + 8];
                assert_eq!(
                    decode(settings, &storage[offset..], &mut output[3..3 + length]),
                    Ok(length)
                );
                assert_eq!(&output[3..3 + length], plain);
                assert_eq!(&output[..3], &[0xa5; 3]);
                assert_eq!(&output[3 + length..], &[0xa5; 5]);
            }
            for capacity in [0, length.saturating_sub(1), length, length + 1] {
                compare(settings, &input, capacity);
            }
        }
    }
}

#[test]
fn malformed_positions_and_tail_bits_preserve_exact_errors_before_writes() {
    for (settings, _) in profiles() {
        for length in [
            0, 1, 2, 3, 4, 15, 16, 17, 18, 19, 20, 31, 32, 33, 63, 64, 65, 68,
        ] {
            let mut input = std::vec![b'A'; length];
            for position in 0..length {
                for byte in 0..=255 {
                    input[position] = byte;
                    for capacity in [0, 1, 64] {
                        compare(settings, &input, capacity);
                    }
                }
                input[position] = b'A';
            }
        }
        for second in 0..64 {
            for third in 0..64 {
                let alphabet = settings.alphabet().as_array();
                for tail in [
                    std::vec![b'A', alphabet[second], b'=', b'='],
                    std::vec![b'A', alphabet[second], alphabet[third], b'='],
                    std::vec![b'A', alphabet[second]],
                    std::vec![b'A', alphabet[second], alphabet[third]],
                ] {
                    let mut input = std::vec![b'A'; 32];
                    input.extend_from_slice(&tail);
                    compare(settings, &input, 32);
                }
            }
        }
    }
}

#[test]
fn public_auto_does_not_select_the_candidate() {
    let codec = crate::STRICT_STANDARD_PADDED;
    let input = [b'A'; 64];
    let mut output = [0; 48];
    let before = crate::decode_validation::observation::fast_calls();
    codec.decode_into(&input, &mut output).unwrap();
    assert_eq!(
        crate::decode_validation::observation::fast_calls(),
        before + 1
    );
    let before = crate::decode_validation::observation::fast_calls();
    decode(codec.settings(), &input, &mut output).unwrap();
    assert_eq!(crate::decode_validation::observation::fast_calls(), before);
}
