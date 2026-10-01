use super::*;
use crate::v2::rfc4648_oracle as oracle;
use std::{hint::black_box, time::Instant};

#[cfg(test)]
#[test]
#[ignore = "opt-in same-process development measurement, not hardware admission"]
fn same_process_decode_comparison() {
    assert!(crate::simd::neon_available());
    for (profile, (settings, oracle_profile)) in tests::profiles().into_iter().enumerate() {
        let codec = crate::CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        for length in [0, 3, 32, 1024, 65536, 1_048_576] {
            let plain: std::vec::Vec<_> = (0..length)
                .map(|n| u8::try_from((n * 73 + 19) % 256).unwrap())
                .collect();
            let input = oracle::encode(oracle_profile, &plain);
            let mut output = std::vec![0xa5; length];
            let rounds = if length <= 32 {
                10000
            } else if length <= 1024 {
                500
            } else if length <= 65536 {
                32
            } else {
                4
            };
            for sample in 0..7 {
                // Rotate ordering within one process rather than measuring each
                // strategy in a separate thermal/cache phase.
                for index in 0..3 {
                    let mode = (sample + index) % 3;
                    let operation = |output: &mut [u8]| match mode {
                        0 => codec.decode_into(black_box(&input), black_box(output)),
                        1 => codec.decode_into_with_validation(
                            black_box(&input),
                            black_box(output),
                            DecodeValidation::ScalarReference,
                        ),
                        _ => decode(black_box(settings), black_box(&input), black_box(output)),
                    };
                    assert_eq!(operation(&mut output), Ok(length));
                    assert_eq!(output, plain);
                    let start = Instant::now();
                    for _ in 0..rounds {
                        assert_eq!(operation(&mut output), Ok(length));
                    }
                    let elapsed = start.elapsed().as_nanos();
                    assert_eq!(output, plain);
                    std::println!(
                        "neon-bench,{profile},{length},{sample},{mode},{rounds},{elapsed}"
                    );
                }
            }
        }
    }
}
