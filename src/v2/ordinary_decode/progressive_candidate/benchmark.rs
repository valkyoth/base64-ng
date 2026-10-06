use super::*;
use std::{hint::black_box, time::Instant};

#[cfg(test)]
fn incremental(settings: CodecSettings, input: &[u8], output: &mut [u8]) -> usize {
    let mut decoder = if settings.decode_padding() == DecodePadding::RequireCanonical {
        DecoderState::new_padded(settings)
    } else {
        DecoderState::new_unpadded(settings)
    };
    let step = decoder.update(input, output).unwrap();
    assert_eq!(step.progress().input_consumed(), input.len());
    let written = step.progress().output_produced();
    written
        + decoder
            .finish(&mut output[written..])
            .unwrap()
            .progress()
            .output_produced()
}

#[test]
#[ignore = "opt-in paired progressive decision measurement; not hardware admission"]
fn paired_complete_operations() {
    let _ = crate::initialize_backends();
    assert!(
        vector::select(4096).is_some(),
        "native acceleration required"
    );
    for (profile, (settings, _)) in tests::profiles().into_iter().enumerate() {
        let codec = crate::CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        for length in [4096, 16384, 65536, 1_048_576] {
            let plain: std::vec::Vec<_> = (0..length)
                .map(|n| u8::try_from((n * 73 + 19) % 256).unwrap())
                .collect();
            let mut input = std::vec![0; codec.encoded_len(length).unwrap()];
            codec.encode_into(&plain, &mut input).unwrap();
            let mut output = std::vec![0; length];
            let (_, report) = codec
                .decode_into_with_report(&input, &mut output, DecodeValidation::Auto)
                .unwrap();
            let backend = vector::select(input.len()).unwrap();
            assert_eq!(report.validator(), crate::DecodeValidator::Vector(backend));
            assert_eq!(report.output_backend(), backend);
            std::println!(
                "progressive-backend,{profile},{length},{}",
                backend.as_str()
            );
            let rounds = (4_194_304 / length).max(4);
            for sample in 0..11 {
                for index in 0..3 {
                    let mode = (sample + index) % 3;
                    let operation = |out: &mut [u8]| match mode {
                        0 => codec
                            .decode_into(black_box(&input), black_box(out))
                            .unwrap(),
                        1 => {
                            decode(black_box(settings), black_box(&input), black_box(out))
                                .unwrap()
                                .written
                        }
                        _ => incremental(black_box(settings), black_box(&input), black_box(out)),
                    };
                    assert_eq!(operation(&mut output), length);
                    assert_eq!(output, plain);
                    assert_eq!(vector::select(input.len()), Some(backend));
                    let start = Instant::now();
                    for _ in 0..rounds {
                        black_box(operation(&mut output));
                    }
                    let elapsed = start.elapsed().as_nanos();
                    assert_eq!(output, plain);
                    std::println!(
                        "progressive-bench,{profile},{length},{sample},{mode},{rounds},{elapsed}"
                    );
                }
            }
        }
    }
}
