use super::*;
use std::{hint::black_box, time::Instant};

#[cfg(test)]
#[test]
#[ignore = "opt-in paired whole-call development measurement, not hardware admission"]
fn in_place_bulk_paired_whole_call_comparison() {
    let _ = crate::initialize_backends();
    for (profile, settings) in [
        STRICT_STANDARD_PADDED.settings(),
        STRICT_URL_SAFE_PADDED.settings(),
    ]
    .into_iter()
    .enumerate()
    {
        let codec = CodecBuilder::new(*settings.alphabet()).build().unwrap();
        for len in [16, 64, 384, 4096, 65_536, 1_048_576] {
            let plain: Vec<u8> = (0..len)
                .map(|i| u8::try_from(i % 256).unwrap().wrapping_mul(73))
                .collect();
            let input = codec.encode_to_string(&plain).unwrap().into_bytes();
            let mut buffer = input.clone();
            let rounds = (1_048_576 / len).clamp(4, 4096);
            for sample in 0..7 {
                for turn in 0..2 {
                    let mode = (sample + turn) % 2;
                    let start = Instant::now();
                    for _ in 0..rounds {
                        buffer.copy_from_slice(black_box(&input));
                        let n = if mode == 0 {
                            codec
                                .decode_in_place(black_box(&mut buffer), input.len())
                                .unwrap()
                        } else {
                            let n = codec.decoded_len(black_box(&buffer)).unwrap();
                            crate::v2::in_place::decode_forward(
                                codec.settings(),
                                black_box(&mut buffer),
                                input.len(),
                            );
                            n
                        };
                        black_box(&buffer[..n]);
                    }
                    let ns = start.elapsed().as_nanos();
                    assert_eq!(&buffer[..plain.len()], plain);
                    assert_eq!(&buffer[plain.len()..], &input[plain.len()..]);
                    std::println!("in-place-bench,{profile},{len},{sample},{mode},{rounds},{ns}");
                }
            }
        }
    }
}
