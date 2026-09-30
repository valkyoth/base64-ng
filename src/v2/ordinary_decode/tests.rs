extern crate std;
use super::*;
use crate::v2::rfc4648_oracle::{self as oracle, Profile};
use crate::{Base64, CodecBuilder, EncodePadding, RuntimeSpec, TrailingBits};
use std::vec::Vec;

fn profiles() -> [(Base64<RuntimeSpec>, Profile); 4] {
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
    .map(|(settings, profile)| {
        (
            CodecBuilder::new(*settings.alphabet())
                .encode_padding(settings.encode_padding())
                .decode_padding(settings.decode_padding())
                .build()
                .unwrap(),
            profile,
        )
    })
}

fn check(codec: &Base64<RuntimeSpec>, profile: Profile, input: &[u8]) {
    let reference = validate_and_measure(codec.settings(), input);
    let independent = oracle::decode(profile, input);
    assert_eq!(reference.is_ok(), independent.is_ok(), "{input:?}");
    for capacity in [0, 1, 2, 3, 6, 16] {
        let mut output = [0xa5; 16];
        let actual = codec.decode_into(input, &mut output[..capacity]);
        let expected = match reference {
            Err(error) => Err(error),
            Ok(required) if required > capacity => Err(OneShotError::OutputTooSmall {
                required,
                available: capacity,
            }),
            Ok(required) => Ok(required),
        };
        assert_eq!(actual, expected, "{input:?}");
        if let Ok(written) = actual {
            assert_eq!(&output[..written], independent.as_ref().unwrap().as_slice());
            assert!(output[written..].iter().all(|&byte| byte == 0xa5));
        } else {
            assert_eq!(output, [0xa5; 16]);
        }
    }
}

#[test]
fn exhaustive_short_bytes_preserve_diagnostics_and_destinations() {
    for (codec, profile) in profiles() {
        check(&codec, profile, b"");
        for first in 0..=255 {
            check(&codec, profile, &[first]);
            for second in 0..=255 {
                check(&codec, profile, &[first, second]);
            }
        }
    }
}

#[test]
fn every_terminal_sextet_and_padding_position_preserves_canonicality() {
    for (codec, profile) in profiles() {
        let settings = codec.settings();
        let alphabet = settings.alphabet().as_array();
        for &second in alphabet {
            check(&codec, profile, &[b'A', second, b'=', b'=']);
            for &third in alphabet {
                check(&codec, profile, &[b'A', second, third]);
                check(&codec, profile, &[b'A', second, third, b'=']);
            }
        }
        for len in 0..=12 {
            let mut input = [b'A'; 12];
            for position in 0..len {
                for byte in 0..=255 {
                    input[position] = byte;
                    check(&codec, profile, &input[..len]);
                }
                input[position] = b'A';
            }
        }
    }
}

#[test]
fn custom_and_relaxed_codecs_keep_reference_fallback() {
    let mut table = *crate::STRICT_STANDARD_PADDED
        .settings()
        .alphabet()
        .as_array();
    table.rotate_left(7);
    for alphabet in [
        table,
        *crate::STRICT_STANDARD_PADDED
            .settings()
            .alphabet()
            .as_array(),
    ] {
        for bits in [
            TrailingBits::AllowNonCanonical,
            TrailingBits::RequireCanonical,
        ] {
            let codec = CodecBuilder::from_table(alphabet)
                .unwrap()
                .encode_padding(EncodePadding::Unpadded)
                .decode_padding(DecodePadding::Indifferent)
                .trailing_bits(bits)
                .build()
                .unwrap();
            for input in [
                b"".as_slice(),
                b"Zg",
                b"Zg=",
                b"Zg==",
                b"Zh==",
                b"Zm9vZg=",
                b"!!!!",
                b"A===",
            ] {
                let mut reference_output = [0; 32];
                let mut state = codec.decoder();
                let reference = state
                    .update(input, &mut reference_output)
                    .and_then(|step| {
                        let n = step.progress().output_produced();
                        state
                            .finish(&mut reference_output[n..])
                            .map(|step| n + step.progress().output_produced())
                    })
                    .map_err(map_operation_error);
                let mut output = [0xa5; 32];
                assert_eq!(codec.decode_into(input, &mut output), reference);
                if let Ok(n) = reference {
                    assert_eq!(output[..n], reference_output[..n]);
                    assert!(output[n..].iter().all(|&b| b == 0xa5));
                } else {
                    assert_eq!(output, [0xa5; 32]);
                }
            }
        }
    }
}

#[test]
fn full_blocks_and_tails_match_the_independent_oracle() {
    for (codec, profile) in profiles() {
        for len in [
            0, 1, 2, 3, 4, 15, 16, 17, 31, 32, 33, 127, 128, 129, 1023, 1024,
        ] {
            let plain: Vec<_> = (0..len).map(|i| u8::try_from(i % 256).unwrap()).collect();
            let encoded = oracle::encode(profile, &plain);
            let mut output = std::vec![0xa5; len + 5];
            assert_eq!(codec.decode_into(&encoded, &mut output), Ok(len));
            assert_eq!(output[..len], plain);
            assert_eq!(output[len..], [0xa5; 5]);
        }
    }
}

#[test]
fn fault_injection_stops_before_capacity_checks_and_writes() {
    for (input, accepted) in [(b"!!!!".as_slice(), true), (b"Zm9v".as_slice(), false)] {
        for capacity in 0..=8 {
            let mut output = [0xa5; 8];
            let result = Preflight::classified_for_test(
                input,
                crate::STRICT_STANDARD_PADDED.settings(),
                validate_and_measure,
                accepted,
            )
            .map_err(map_preflight_error)
            .and_then(|proof| write(proof, &mut output[..capacity]));
            assert_eq!(
                result,
                Err(OneShotError::Backend(BackendFault::ImpossibleState))
            );
            assert_eq!(output, [0xa5; 8]);
        }
    }
}

#[cfg(feature = "alloc")]
#[test]
fn allocating_decode_retains_validation_across_reservation() {
    use crate::decode_validation::observation;
    let codec = crate::STRICT_STANDARD_PADDED;
    let before = observation::fast_calls();
    let output = codec
        .decode_to_vec_with_injected_reserver(b"Zm9v", 3, |output, required| {
            assert_eq!(observation::fast_calls(), before + 1);
            assert_eq!(required, 3);
            output.try_reserve_exact(required).unwrap();
            Ok(())
        })
        .unwrap();
    assert_eq!(observation::fast_calls(), before + 1);
    assert_eq!(output, b"foo");
    assert!(
        codec
            .decode_to_vec_with_injected_reserver(b"!!!!", 0, |_, _| {
                panic!("malformed input must not attempt allocation")
            })
            .is_err()
    );
}
