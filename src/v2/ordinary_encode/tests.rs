extern crate std;
mod sinks;

use super::*;
use crate::v2::rfc4648_oracle::{self as oracle, Profile};
use crate::{Base64, Codec, CodecBuilder, DecodePadding, EncodeError, TrailingBits};
use std::{cell::Cell, vec};

std::thread_local! {
    static STATE: Cell<(u8, usize, Option<encode_backend::EncodeBackend>)> = const {
        Cell::new((0, 0, None))
    };
}

pub(super) fn result(
    result: Result<usize, EncodeError>,
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    STATE.with(|state| {
        let (fault, calls, backend) = state.get();
        state.set((fault, calls + 1, backend));
        match fault {
            1 => {
                output.fill(0xff);
                Err(EncodeError::LengthOverflow)
            }
            2 => {
                output.fill(0xff);
                Ok(usize::MAX)
            }
            _ => result,
        }
    })
}

pub(super) fn quarantine(backend: encode_backend::EncodeBackend) -> bool {
    STATE.with(|state| {
        let (fault, calls, _) = state.get();
        if fault == 0 {
            return false;
        }
        state.set((fault, calls, Some(backend)));
        true
    })
}

fn observe(fault: u8, action: impl FnOnce()) -> (usize, Option<encode_backend::EncodeBackend>) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            STATE.with(|state| state.set((0, 0, None)));
        }
    }
    STATE.with(|state| state.set((fault, 0, None)));
    let _reset = Reset;
    action();
    STATE.with(|state| {
        let (_, calls, backend) = state.get();
        (calls, backend)
    })
}

fn ready() -> encode_backend::EncodeBackend {
    let _ = crate::initialize_backends();
    let candidate = encode_backend::candidate_encode_backend();
    if candidate == encode_backend::EncodeBackend::Scalar {
        return candidate;
    }
    let start = std::time::Instant::now();
    loop {
        let backend = encode_backend::active_encode_backend_for_input(4096);
        if backend != encode_backend::EncodeBackend::Scalar {
            return backend;
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        std::thread::yield_now();
    }
}

#[test]
fn incremental_bulk_encode_recovers_rejected_and_wrong_length_writes() {
    if ready() == encode_backend::EncodeBackend::Scalar {
        return;
    }
    let input = [0; 4098];
    for fault in [1, 2] {
        let mut encoder = crate::STRICT_STANDARD_PADDED.encoder();
        encoder.update(&[0], &mut []).unwrap();
        let (calls, quarantined) = observe(fault, || {
            let mut output = [0xa5; 5500];
            let step = encoder.update(&input, &mut output).unwrap();
            assert_eq!(step.progress().input_consumed(), input.len());
            assert_eq!(step.progress().output_produced(), 5464);
            assert_eq!(output[..5464], [b'A'; 5464]);
            assert_eq!(output[5464..], [0xa5; 36]);
            assert_eq!(encoder.buffered_input_len(), 1);
        });
        assert_eq!(calls, 1);
        assert!(quarantined.is_some());
    }
}

#[test]
fn boundaries_and_all_input_bytes_match_independent_oracle() {
    fn check<S: Codec>(codec: &Base64<S>, profile: Profile) {
        for len in [
            0, 1, 2, 3, 11, 12, 13, 23, 24, 25, 47, 48, 49, 191, 192, 193, 383, 384, 385, 767, 768,
            769, 1537, 4096,
        ] {
            let input: std::vec::Vec<_> =
                (0..len).map(|i| u8::try_from(i % 256).unwrap()).collect();
            let expected = oracle::encode(profile, &input);
            #[cfg(feature = "alloc")]
            {
                let mut appended = std::string::String::from("prefix:");
                codec.encode_append(&input, &mut appended).unwrap();
                assert_eq!(&appended.as_bytes()[7..], expected);
            }
            for offset in [0, 1, 7] {
                let mut output = vec![0xa5; offset + expected.len() + 8];
                assert_eq!(
                    codec.encode_into(&input, &mut output[offset..]),
                    Ok(expected.len())
                );
                assert_eq!(&output[offset..offset + expected.len()], expected);
                assert!(
                    output[..offset]
                        .iter()
                        .chain(&output[offset + expected.len()..])
                        .all(|&b| b == 0xa5)
                );
                if !expected.is_empty() {
                    output.fill(0xa5);
                    assert!(
                        codec
                            .encode_into(&input, &mut output[offset..offset + expected.len() - 1])
                            .is_err()
                    );
                    assert!(output.iter().all(|&b| b == 0xa5));
                }
            }
        }
        let mut input = [0; 387];
        for position in 0..input.len() {
            for byte in 0..=u8::MAX {
                input[position] = byte;
                let expected = oracle::encode(profile, &input);
                let mut output = [0; 516];
                assert_eq!(codec.encode_into(&input, &mut output), Ok(516));
                assert_eq!(output.as_slice(), expected);
            }
        }
    }
    let _ = ready();
    check(&crate::STRICT_STANDARD_PADDED, Profile::StandardPadded);
    check(&crate::STRICT_STANDARD_UNPADDED, Profile::StandardUnpadded);
    check(&crate::STRICT_URL_SAFE_PADDED, Profile::UrlSafePadded);
    check(&crate::STRICT_URL_SAFE_UNPADDED, Profile::UrlSafeUnpadded);
}

#[test]
fn encode_policy_is_independent_of_decode_acceptance() {
    let _ = ready();
    let input = [0xfb; 4097];
    for (table, url) in [(Standard::ENCODE, false), (UrlSafe::ENCODE, true)] {
        for padding in [EncodePadding::Padded, EncodePadding::Unpadded] {
            let codec = CodecBuilder::from_table(table)
                .unwrap()
                .encode_padding(padding)
                .decode_padding(DecodePadding::Indifferent)
                .trailing_bits(TrailingBits::AllowNonCanonical)
                .build()
                .unwrap();
            let profile = match (url, padding) {
                (false, EncodePadding::Padded) => Profile::StandardPadded,
                (false, EncodePadding::Unpadded) => Profile::StandardUnpadded,
                (true, EncodePadding::Padded) => Profile::UrlSafePadded,
                (true, EncodePadding::Unpadded) => Profile::UrlSafeUnpadded,
            };
            let expected = oracle::encode(profile, &input);
            let mut output = vec![0; expected.len()];
            assert_eq!(codec.encode_into(&input, &mut output), Ok(expected.len()));
            assert_eq!(output, expected);
        }
    }
}

#[test]
fn public_route_executes_admitted_backend_and_recovers_rejected_writes() {
    let backend = ready();
    if backend == encode_backend::EncodeBackend::Scalar {
        return;
    }
    let input = [0; 4097];
    let expected = oracle::encode(Profile::StandardPadded, &input);
    for fault in [0, 1, 2] {
        let (calls, quarantined) = observe(fault, || {
            let mut output = vec![0xa5; expected.len() + 8];
            assert_eq!(
                crate::STRICT_STANDARD_PADDED.encode_into(&input, &mut output),
                Ok(expected.len())
            );
            assert_eq!(&output[..expected.len()], expected);
            assert_eq!(&output[expected.len()..], &[0xa5; 8]);
            assert_eq!(encode_backend::last_test_execution(), backend);
        });
        assert_eq!(calls, 1);
        assert_eq!(quarantined, (fault != 0).then_some(backend));
    }
    for len in 0..192 {
        assert_eq!(
            observe(1, || {
                let mut output = [0; 256];
                crate::STRICT_STANDARD_PADDED
                    .encode_into(&input[..len], &mut output)
                    .unwrap();
            })
            .0,
            0
        );
    }
    assert_eq!(
        observe(1, || {
            assert!(
                crate::STRICT_STANDARD_PADDED
                    .encode_into(&input, &mut [])
                    .is_err()
            );
        })
        .0,
        0
    );
    let mut custom = Standard::ENCODE;
    custom.swap(0, 1);
    let codec = CodecBuilder::from_table(custom).unwrap().build().unwrap();
    assert_eq!(
        observe(1, || {
            let mut output = vec![0; expected.len()];
            codec.encode_into(&input, &mut output).unwrap();
            let custom_expected: std::vec::Vec<_> = expected
                .iter()
                .map(|&b| match b {
                    b'A' => b'B',
                    b'B' => b'A',
                    _ => b,
                })
                .collect();
            assert_eq!(output, custom_expected);
        })
        .0,
        0
    );
}

#[cfg(feature = "alloc")]
#[test]
fn allocating_and_buffered_surfaces_use_dispatch_without_changing_chunks() {
    let backend = ready();
    let input = [0xfbu8; 1537];
    let codec = crate::STRICT_URL_SAFE_UNPADDED;
    let expected = oracle::encode(Profile::UrlSafeUnpadded, &input);
    let (calls, _) = observe(0, || {
        assert_eq!(codec.encode_to_string(&input).unwrap().as_bytes(), expected);
        assert_eq!(
            codec.encode_bounded::<2052>(&input).unwrap().as_bytes(),
            expected
        );
        let mut appended = std::string::String::from("prefix:");
        codec.encode_append(&input, &mut appended).unwrap();
        assert_eq!(&appended.as_bytes()[7..], expected);
        let mut formatted = std::string::String::new();
        codec.encode_to_fmt(&input, &mut formatted).unwrap();
        assert_eq!(formatted.as_bytes(), expected);
        assert_eq!(
            std::format!("{}", codec.display(&input).unwrap()).as_bytes(),
            expected
        );
        let actual: std::vec::Vec<_> =
            crate::v2::chunks::BufferedChunks::new(codec.settings(), &input).collect();
        let original: std::vec::Vec<_> = codec.encoded_chunks(&input).unwrap().collect();
        assert_eq!(actual.len(), original.len());
        for (actual, original) in actual.iter().zip(&original) {
            assert_eq!(actual.as_bytes(), original.as_bytes());
        }
    });
    assert_eq!(calls > 0, backend != encode_backend::EncodeBackend::Scalar);
    assert_eq!(
        observe(0, || {
            assert!(codec.encode_to_string_with_limit(&input, 1).is_err());
            assert!(
                codec
                    .encode_to_string_with_injected_reserver(&input, usize::MAX, |_, required| Err(
                        crate::OneShotError::AllocationFailed {
                            requested: required
                        }
                    ))
                    .is_err()
            );
        })
        .0,
        0
    );
}
