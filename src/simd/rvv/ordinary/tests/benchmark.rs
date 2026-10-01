use crate::{DecodeValidation, runtime::Backend};
use std::{hint::black_box, time::Instant};

#[cfg(test)]
#[test]
#[ignore = "opt-in native same-process development measurement, not release admission"]
fn rvv_same_process_decode_comparison() {
    assert_eq!(super::width(Backend::Rvv), Some(16));
    let _ = crate::initialize_backends();
    for (profile, settings) in [
        crate::STRICT_STANDARD_PADDED.settings(),
        crate::STRICT_STANDARD_UNPADDED.settings(),
        crate::STRICT_URL_SAFE_PADDED.settings(),
        crate::STRICT_URL_SAFE_UNPADDED.settings(),
    ]
    .into_iter()
    .enumerate()
    {
        let codec = crate::CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        for len in [0, 3, 32, 768, 1024, 65536, 1_048_576] {
            let plain: std::vec::Vec<_> = (0..len)
                .map(|i| u8::try_from(i % 256).unwrap().wrapping_mul(73))
                .collect();
            let input = codec.encode_to_string(&plain).unwrap().into_bytes();
            let mut output = std::vec![0xa5; len];
            let rounds = if len <= 32 {
                1000
            } else if len <= 1024 {
                50
            } else {
                2
            };
            for sample in 0..7 {
                for index in 0..2 {
                    let mode = (sample + index) % 2;
                    let policy = if mode == 0 {
                        DecodeValidation::Auto
                    } else {
                        DecodeValidation::ScalarReference
                    };
                    assert_eq!(
                        codec.decode_into_with_validation(&input, &mut output, policy),
                        Ok(len)
                    );
                    assert_eq!(output, plain);
                    let start = Instant::now();
                    for _ in 0..rounds {
                        assert_eq!(
                            codec.decode_into_with_validation(
                                black_box(&input),
                                black_box(&mut output),
                                policy
                            ),
                            Ok(len)
                        );
                    }
                    let elapsed = start.elapsed().as_nanos();
                    assert_eq!(output, plain);
                    std::println!("rvv-bench,{profile},{len},{sample},{mode},{rounds},{elapsed}");
                }
            }
        }
    }
}
