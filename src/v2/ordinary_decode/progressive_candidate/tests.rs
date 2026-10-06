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
fn canonical_inputs_offsets_and_complete_output_match_independent_oracle() {
    let _ = crate::initialize_backends();
    for (settings, profile) in profiles() {
        for len in (0..=100).chain([767, 768, 769, 1535, 1536, 1537, 4096, 8192]) {
            let plain: std::vec::Vec<_> = (0..len)
                .map(|i| u8::try_from((i * 73 + 19) % 256).unwrap())
                .collect();
            let encoded = oracle::encode(profile, &plain);
            for offset in 0..32 {
                let mut storage = std::vec![0x80; offset];
                storage.extend_from_slice(&encoded);
                let mut output = std::vec![0xa5; len + offset + 8];
                assert_eq!(
                    decode(
                        settings,
                        &storage[offset..],
                        &mut output[offset..offset + len]
                    ),
                    Ok(Progress {
                        consumed: encoded.len(),
                        written: len
                    })
                );
                assert_eq!(&output[offset..offset + len], plain);
                assert!(
                    output[..offset]
                        .iter()
                        .chain(&output[offset + len..])
                        .all(|&b| b == 0xa5)
                );
            }
        }
    }
}

#[test]
fn every_byte_in_first_middle_final_blocks_and_tail_preserves_prefix_accounting() {
    for (settings, profile) in profiles() {
        let mut input = std::vec![b'A'; BLOCK * 3 + 4];
        for position in (0..32)
            .chain(BLOCK..BLOCK + 32)
            .chain(2 * BLOCK..2 * BLOCK + 32)
            .chain(3 * BLOCK..input.len())
        {
            for byte in 0..=255 {
                input[position] = byte;
                let mut output = std::vec![0xa5; input.len() / 4 * 3 + 8];
                let result = decode(settings, &input, &mut output);
                if let Ok(expected) = oracle::decode(profile, &input) {
                    let progress = result.unwrap();
                    assert_eq!(progress.written, expected.len());
                    assert_eq!(&output[..progress.written], expected);
                    assert!(output[progress.written..].iter().all(|&b| b == 0xa5));
                } else {
                    let error = result.unwrap_err();
                    assert_eq!(
                        error.failure,
                        Failure::Input(validate_and_measure(settings, &input).unwrap_err())
                    );
                    assert_eq!(error.progress.consumed, position / BLOCK * BLOCK);
                    assert_eq!(error.progress.written, error.progress.consumed / 4 * 3);
                    assert!(output[..error.progress.written].iter().all(|&b| b == 0));
                    assert!(output[error.progress.written..].iter().all(|&b| b == 0xa5));
                }
            }
            input[position] = b'A';
        }
    }
}

#[test]
fn tail_canonicality_truncation_and_padding_after_committed_prefix() {
    for (settings, profile) in profiles() {
        for second in 0..64 {
            for third in 0..64 {
                let alphabet = settings.alphabet().as_array();
                for tail in [
                    std::vec![b'A', alphabet[second], b'=', b'='],
                    std::vec![b'A', alphabet[second], alphabet[third], b'='],
                    std::vec![b'A', alphabet[second]],
                    std::vec![b'A', alphabet[second], alphabet[third]],
                ] {
                    let mut input = std::vec![b'A'; BLOCK];
                    input.extend_from_slice(&tail);
                    let mut output = [0xa5; DECODED + 8];
                    let result = decode(settings, &input, &mut output);
                    if let Ok(expected) = oracle::decode(profile, &input) {
                        let progress = result.unwrap();
                        assert_eq!(&output[..progress.written], expected);
                        assert!(output[progress.written..].iter().all(|&b| b == 0xa5));
                    } else {
                        let error = result.unwrap_err();
                        assert_eq!(
                            error.progress,
                            Progress {
                                consumed: BLOCK,
                                written: DECODED
                            }
                        );
                        assert_eq!(
                            error.failure,
                            Failure::Input(validate_and_measure(settings, &input).unwrap_err())
                        );
                        assert_eq!(&output[..DECODED], &[0; DECODED]);
                        assert_eq!(&output[DECODED..], &[0xa5; 8]);
                    }
                }
            }
        }
        for tail in [b"A".as_slice(), b"====", b"AA==AAAA", b"AAA=AAAA", b"AA\nA"] {
            let mut input = std::vec![b'A'; BLOCK];
            input.extend_from_slice(tail);
            assert!(decode(settings, &input, &mut [0xa5; DECODED + 8]).is_err());
        }
    }
}

#[test]
fn all_capacities_are_block_atomic_and_output_full_is_retryable() {
    for (settings, profile) in profiles() {
        let plain = std::vec![0x53; 2 * DECODED + 2];
        let input = oracle::encode(profile, &plain);
        for capacity in 0..=plain.len() + 1 {
            let mut output = std::vec![0xa5; capacity + 8];
            let result = decode(settings, &input, &mut output[..capacity]);
            let progress = match result {
                Ok(progress) => progress,
                Err(error) => {
                    assert!(matches!(error.failure, Failure::OutputFull { .. }));
                    let p = error.progress;
                    let mut rest = std::vec![0; plain.len() - p.written];
                    let retry = decode(settings, &input[p.consumed..], &mut rest).unwrap();
                    assert_eq!(retry.written + p.written, plain.len());
                    assert_eq!(rest, plain[p.written..]);
                    p
                }
            };
            assert_eq!(&output[..progress.written], &plain[..progress.written]);
            assert!(output[progress.written..].iter().all(|&b| b == 0xa5));
        }
    }
    let mut input = [b'A'; BLOCK + 4];
    input[0] = b'!';
    let error = decode(crate::STRICT_STANDARD_PADDED.settings(), &input, &mut []).unwrap_err();
    assert_eq!(error.failure, Failure::OutputFull { minimum: DECODED });
}

struct Faulty {
    calls: usize,
    mode: u8,
    quarantined: bool,
}
impl Kernel for Faulty {
    fn decode(&mut self, family: Family, input: &[u8], output: &mut [u8]) -> Option<bool> {
        self.calls += 1;
        if self.calls == 2 {
            if self.mode == 0 {
                return None;
            }
            output.fill(0x5a);
            return Some(self.mode == 2);
        }
        Some(scalar(family, input, output).is_ok())
    }
    fn quarantine(&mut self) {
        self.quarantined = true;
    }
}

#[test]
fn unavailable_and_suspect_kernels_never_expose_unverified_blocks() {
    for checked in [false, true] {
        for mode in 0..=2 {
            if mode == 2 && !checked {
                continue;
            }
            let mut kernel = Faulty {
                calls: 0,
                mode,
                quarantined: false,
            };
            let mut output = [0xa5; 2 * DECODED + 3];
            let input = [b'A'; 2 * BLOCK + 4];
            let settings = crate::STRICT_STANDARD_PADDED.settings();
            let result = if checked {
                decode_with::<true>(settings, &input, &mut output, &mut kernel)
            } else {
                decode_with::<false>(settings, &input, &mut output, &mut kernel)
            };
            if mode == 0 {
                assert!(result.is_ok());
                assert_eq!(output, [0; 2 * DECODED + 3]);
                assert!(!kernel.quarantined);
            } else {
                assert_eq!(
                    result,
                    Err(Rejected {
                        progress: Progress {
                            consumed: BLOCK,
                            written: DECODED
                        },
                        failure: Failure::Backend,
                    })
                );
                assert!(kernel.quarantined);
                assert_eq!(&output[..DECODED], &[0; DECODED]);
                assert!(output[DECODED..].iter().all(|&b| b == 0xa5));
            }
        }
    }
}

#[test]
fn unsupported_codecs_do_not_mutate_and_transactional_api_stays_transactional() {
    let mut alphabet = *crate::STRICT_STANDARD_PADDED
        .settings()
        .alphabet()
        .as_array();
    alphabet.rotate_left(1);
    let custom = crate::CodecBuilder::from_table(alphabet)
        .unwrap()
        .build()
        .unwrap();
    let mut output = [0xa5; 2048];
    assert_eq!(
        decode(custom.settings(), b"AAAA", &mut output)
            .unwrap_err()
            .failure,
        Failure::Unsupported
    );
    let mut input = [b'A'; BLOCK + 4];
    input[BLOCK] = b'!';
    assert!(
        crate::STRICT_STANDARD_PADDED
            .decode_into(&input, &mut output)
            .is_err()
    );
    assert_eq!(output, [0xa5; 2048]);
}
