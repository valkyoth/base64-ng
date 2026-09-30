use super::*;
use crate::v2::rfc4648_oracle::{self as oracle, Profile};

pub(super) fn profiles() -> [(CodecSettings, Profile); 4] {
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
fn avx2_validation_candidate_execution_status() {
    let available = crate::simd::avx2_validation_candidate_available();
    std::println!("AVX2 validation native execution available: {available}");
    if std::env::var_os("BASE64_NG_REQUIRE_AVX2_VALIDATION").is_some() {
        assert!(available, "native AVX2 execution was required");
    }
}

#[test]
fn all_bytes_in_every_lane_offset_and_multiple_invalid_lanes() {
    if !crate::simd::avx2_validation_candidate_available() {
        return;
    }
    for (settings, _) in profiles() {
        let alphabet = settings.alphabet().as_array();
        let url_safe = alphabet[62] == b'-';
        for offset in 0..64 {
            let mut storage = [b'A'; 160];
            let blocks = &mut storage[offset..offset + 96];
            for lane in 0..96 {
                for byte in 0..=255 {
                    blocks[lane] = byte;
                    assert_eq!(
                        crate::simd::candidate_validate_avx2(blocks, url_safe),
                        alphabet.contains(&byte)
                    );
                }
                blocks[lane] = b'A';
            }
            for first in 0..32 {
                for second in 32..64 {
                    blocks[first] = 0x80;
                    blocks[second] = b'=';
                    assert!(!crate::simd::candidate_validate_avx2(blocks, url_safe));
                    blocks[first] = b'A';
                    blocks[second] = b'A';
                }
            }
        }
        for len in 1..32 {
            assert!(!crate::simd::candidate_validate_avx2(
                &[b'A'; 32][..len],
                url_safe
            ));
        }
        let mut output = [0xa5; 23];
        assert!(!crate::simd::candidate_decode_avx2(
            &[b'A'; 32],
            &mut output,
            url_safe
        ));
        assert_eq!(output, [0xa5; 23]);
    }
}

fn compare(settings: CodecSettings, input: &[u8], capacity: usize) {
    let mut output = std::vec![0xa5; capacity + 8];
    let mut reference = output.clone();
    let expected = prepare(settings, input, DecodeValidation::ScalarReference)
        .and_then(|proof| write(proof, &mut reference[3..3 + capacity]));
    assert_eq!(
        decode(settings, input, &mut output[3..3 + capacity]),
        expected
    );
    assert_eq!(output, reference);
}

#[test]
fn boundaries_tails_offsets_and_output_sentinels_match_oracle() {
    for (settings, profile) in profiles() {
        for length in (0..=192).chain([1023, 1024, 1025, 4096]) {
            let plain: std::vec::Vec<_> = (0..length)
                .map(|n| u8::try_from((n * 73 + 19) % 256).unwrap())
                .collect();
            let input = oracle::encode(profile, &plain);
            for offset in 0..32 {
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
fn malformed_positions_and_tail_bits_preserve_exact_errors() {
    for (settings, _) in profiles() {
        for length in [
            0, 1, 2, 3, 4, 15, 16, 17, 31, 32, 33, 34, 35, 36, 63, 64, 65, 95, 96, 97,
        ] {
            let mut input = std::vec![b'A'; length];
            for position in 0..length {
                for byte in 0..=255 {
                    input[position] = byte;
                    for capacity in [0, 1, 80] {
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
                    let mut input = std::vec![b'A'; 64];
                    input.extend_from_slice(&tail);
                    compare(settings, &input, 64);
                }
            }
        }
    }
}

#[test]
fn custom_relaxed_fallback_and_public_auto_remain_unchanged() {
    let mut custom = *crate::STRICT_STANDARD_PADDED
        .settings()
        .alphabet()
        .as_array();
    custom.rotate_left(9);
    let strict = crate::CodecBuilder::from_table(custom)
        .unwrap()
        .build()
        .unwrap();
    let mut encoded = [0; 48];
    let n = strict.encode_into(&[0x5a; 32], &mut encoded).unwrap();
    compare(strict.settings(), &encoded[..n], 32);
    for bits in [
        crate::TrailingBits::RequireCanonical,
        crate::TrailingBits::AllowNonCanonical,
    ] {
        let codec = crate::CodecBuilder::from_table(custom)
            .unwrap()
            .encode_padding(crate::EncodePadding::Unpadded)
            .decode_padding(DecodePadding::Indifferent)
            .trailing_bits(bits)
            .build()
            .unwrap();
        for input in [b"Zm9vZm9vZm9vZm9vZh=".as_slice(), b"!"] {
            compare(codec.settings(), input, 32);
        }
    }
    let codec = crate::STRICT_STANDARD_PADDED;
    let input = [b'A'; 96];
    let mut output = [0; 72];
    let before = crate::decode_validation::observation::fast_calls();
    codec.decode_into(&input, &mut output).unwrap();
    assert_eq!(
        crate::decode_validation::observation::fast_calls(),
        before + 1
    );
    decode(codec.settings(), &input, &mut output).unwrap();
    assert_eq!(
        crate::decode_validation::observation::fast_calls(),
        before + 1
    );
}
