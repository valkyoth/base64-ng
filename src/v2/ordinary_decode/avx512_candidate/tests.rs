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
fn avx512_validation_candidate_execution_status() {
    let available = crate::simd::avx512_validation_candidate_available();
    std::println!("AVX-512 validation native execution available: {available}");
    if std::env::var_os("BASE64_NG_REQUIRE_AVX512_VALIDATION").is_some() {
        assert!(available, "native AVX-512 execution was required");
    }
}

#[test]
fn avx512_loop_rejects_inconsistent_geometry_before_writes() {
    if !crate::simd::avx512_validation_candidate_available() {
        return;
    }
    for url_safe in [false, true] {
        for input_len in 0_usize..=195 {
            for requested in [
                0,
                1,
                63,
                64,
                65,
                128,
                192,
                input_len.saturating_sub(1),
                input_len,
                input_len + 1,
                usize::MAX,
            ] {
                for capacity in [0, 1, 47, 48, 49, 95, 96, 143, 144, 150] {
                    let mut output = [0xa5; 152];
                    let result = crate::simd::test_avx512_loop_geometry(
                        &[b'A'; 195][..input_len],
                        &mut output[1..][..capacity],
                        requested,
                        url_safe,
                    );
                    let valid_geometry = requested <= input_len && requested / 64 * 48 <= capacity;
                    let written = if valid_geometry {
                        requested / 64 * 48
                    } else {
                        0
                    };
                    let expected = if valid_geometry {
                        (requested / 64 * 64, written, true)
                    } else {
                        (0, 0, false)
                    };
                    assert_eq!(result, Some(expected));
                    assert_eq!(output[0], 0xa5);
                    assert!(output[1..][..written].iter().all(|&byte| byte == 0));
                    assert!(output[1 + written..].iter().all(|&byte| byte == 0xa5));
                }
            }
        }
        for lane in 0..192 {
            let mut input = [b'A'; 192];
            input[lane] = 0xff;
            let mut output = [0xa5; 145];
            let read = lane / 64 * 64;
            let written = lane / 64 * 48;
            assert_eq!(
                crate::simd::test_avx512_loop_geometry(&input, &mut output, 192, url_safe),
                Some((read, written, false))
            );
            assert!(output[..written].iter().all(|&byte| byte == 0));
            assert!(output[written..].iter().all(|&byte| byte == 0xa5));
        }
    }
}

#[test]
fn every_byte_lane_alignment_and_mask_boundary() {
    if !crate::simd::avx512_validation_candidate_available() {
        return;
    }
    for (settings, profile) in profiles() {
        let alphabet = settings.alphabet().as_array();
        let url_safe = alphabet[62] == b'-';
        for offset in 0..64 {
            let mut storage = [b'A'; 256];
            let input = &mut storage[offset..offset + 192];
            for lane in 0..192 {
                for byte in 0..=255 {
                    input[lane] = byte;
                    assert_eq!(
                        crate::simd::candidate_validate_avx512(input, url_safe),
                        alphabet.contains(&byte)
                    );
                }
                input[lane] = b'A';
            }
        }
        // Independently exercise packing and the exact masked output store.
        let mut input = [b'A'; 64];
        for lane in 0..64 {
            for byte in 0..=255 {
                input[lane] = byte;
                let mut output = [0xa5; 50];
                let accepted =
                    crate::simd::candidate_decode_avx512(&input, &mut output[1..49], url_safe);
                assert_eq!(accepted, alphabet.contains(&byte));
                if accepted {
                    assert_eq!(&output[1..49], oracle::decode(profile, &input).unwrap());
                } else {
                    assert_eq!(output, [0xa5; 50]);
                }
                assert_eq!(output[0], 0xa5);
                assert_eq!(output[49], 0xa5);
            }
            input[lane] = b'A';
        }
        for first in [0, 15, 16, 31, 32, 47, 48, 63, 64, 127, 128, 191] {
            for second in [0, 15, 16, 31, 32, 47, 48, 63, 64, 127, 128, 191] {
                let mut input = [b'A'; 192];
                input[first] = 0x80;
                input[second] = b'=';
                assert!(!crate::simd::candidate_validate_avx512(&input, url_safe));
            }
        }
        for length in 1..64 {
            let input = &[b'A'; 64][..length];
            let mut output = [0xa5; 48];
            assert!(!crate::simd::candidate_validate_avx512(input, url_safe));
            assert!(!crate::simd::candidate_decode_avx512(
                input,
                &mut output,
                url_safe
            ));
            assert_eq!(output, [0xa5; 48]);
        }
        let mut output = [0xa5; 47];
        assert!(!crate::simd::candidate_decode_avx512(
            &[b'A'; 64],
            &mut output,
            url_safe
        ));
        assert_eq!(output, [0xa5; 47]);
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
fn boundaries_tails_and_output_offsets_match_oracle() {
    for (settings, profile) in profiles() {
        for length in (0..=200).chain([1023, 1024, 1025, 4096]) {
            let plain: std::vec::Vec<_> = (0..length)
                .map(|n| u8::try_from((n * 73 + 19) % 256).unwrap())
                .collect();
            let input = oracle::encode(profile, &plain);
            for offset in 0..64 {
                let mut storage = std::vec![0xa5; offset];
                storage.extend_from_slice(&input);
                let mut output = std::vec![0xa5; length + offset + 1];
                assert_eq!(
                    decode(
                        settings,
                        &storage[offset..],
                        &mut output[offset..offset + length]
                    ),
                    Ok(length)
                );
                assert_eq!(&output[offset..offset + length], plain);
                assert!(output[..offset].iter().all(|&byte| byte == 0xa5));
                assert_eq!(output[offset + length], 0xa5);
            }
            for capacity in [0, length.saturating_sub(1), length, length + 1] {
                compare(settings, &input, capacity);
            }
        }
    }
}

#[test]
fn malformed_positions_tail_bits_and_error_precedence() {
    for (settings, _) in profiles() {
        for length in [
            0, 1, 2, 3, 4, 31, 32, 33, 63, 64, 65, 66, 67, 68, 127, 128, 129, 191, 192, 193,
        ] {
            let mut input = std::vec![b'A'; length];
            for position in 0..length {
                for byte in 0..=255 {
                    input[position] = byte;
                    for capacity in [0, 1, 160] {
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
                    let mut input = std::vec![b'A'; 128];
                    input.extend_from_slice(&tail);
                    compare(settings, &input, 128);
                }
            }
        }
        let mut input = [b'A'; 196];
        input[63] = b'!';
        input[128] = b'=';
        input[195] = 0xff;
        compare(settings, &input, 200);
        compare(settings, &input, 0);
    }
}

#[test]
fn fallback_and_public_auto_remain_unchanged() {
    let mut custom = *crate::STRICT_STANDARD_PADDED
        .settings()
        .alphabet()
        .as_array();
    custom.rotate_left(9);
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
        let mut encoded = [0; 256];
        let length = codec.encode_into(&[0x5a; 144], &mut encoded).unwrap();
        compare(codec.settings(), &encoded[..length], 144);
        compare(codec.settings(), b"!", 10);
    }
    let codec = crate::STRICT_STANDARD_PADDED;
    let mut output = [0; 144];
    let before = crate::decode_validation::observation::fast_calls();
    codec.decode_into(&[b'A'; 192], &mut output).unwrap();
    assert_eq!(
        crate::decode_validation::observation::fast_calls(),
        before + 1
    );
    decode(codec.settings(), &[b'A'; 192], &mut output).unwrap();
    assert_eq!(
        crate::decode_validation::observation::fast_calls(),
        before + 1
    );
}
