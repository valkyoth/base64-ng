extern crate std;
use super::*;
use std::vec;

fn compare(
    settings: crate::CodecSettings,
    input: &[u8],
    initial: usize,
    validation: DecodeValidation,
    capacity: usize,
) {
    let mut actual = DecoderState::new_padded(settings);
    let mut reference = actual.clone();
    let mut a = [0xa5; 8];
    let mut b = a;
    assert_eq!(
        actual.update(&input[..initial], &mut a[..1]),
        reference.update_reference(&input[..initial], &mut b[..1])
    );
    assert_eq!(a, b);
    let mut offset = actual.source_position();
    let mut turn = 0;
    loop {
        let capacity = if turn == 0 {
            capacity
        } else {
            [1, 2, 3, 4096][turn % 4]
        };
        let mut a = vec![0xa5; capacity + 9];
        let mut b = a.clone();
        let result =
            actual.update_with_validation(&input[offset..], &mut a[..capacity], validation);
        assert_eq!(
            result,
            reference.update_reference(&input[offset..], &mut b[..capacity])
        );
        assert_eq!(a, b);
        assert_eq!(actual, reference);
        let Ok(step) = result else {
            assert_eq!(actual.finish(&mut a), reference.finish(&mut b));
            assert_eq!(a, b);
            break;
        };
        offset += step.progress().input_consumed();
        if offset == input.len() {
            break;
        }
        turn += 1;
    }
    for capacity in [0, 1, 1, 1, 4, 8] {
        let mut a = [0xa5; 8];
        let mut b = a;
        assert_eq!(
            actual.finish(&mut a[..capacity]),
            reference.finish(&mut b[..capacity])
        );
        assert_eq!(a, b);
        assert_eq!(actual, reference);
    }
    actual.clear();
    reference.clear();
    assert_eq!(actual, reference);
}

#[test]
fn incremental_bulk_decode_preserves_progress_state_padding_and_retry() {
    for settings in [
        crate::STRICT_STANDARD_PADDED.settings(),
        crate::STRICT_STANDARD_UNPADDED.settings(),
        crate::STRICT_URL_SAFE_PADDED.settings(),
        crate::STRICT_URL_SAFE_UNPADDED.settings(),
    ] {
        for len in [511, 512, 515, 516, 519, 1024, 4095, 4096, 4100, 8196] {
            let mut input = vec![b'A'; len];
            for initial in 0..=5 {
                for capacity in [0, 1, 2, 3, 383, 384, 385, 3072, 8192] {
                    for validation in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
                        compare(settings, &input, initial, validation, capacity);
                    }
                }
            }
            input[len - 1] = b'=';
            compare(settings, &input, 3, DecodeValidation::Auto, 8192);
            input[len - 2] = b'=';
            compare(settings, &input, 0, DecodeValidation::Auto, 8192);
        }
    }
}

#[test]
fn incremental_bulk_decode_malformed_suffix_is_transactional_and_exact() {
    for position in [
        0, 1, 2, 3, 4, 15, 16, 31, 32, 511, 512, 1023, 4095, 4096, 4099,
    ] {
        for byte in [b'!', b'=', b' ', b'-', b'_', 0, 255] {
            let mut input = vec![b'A'; 4100];
            input[position] = byte;
            for initial in [0, 1, 3, 5] {
                for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
                    compare(
                        crate::STRICT_STANDARD_PADDED.settings(),
                        &input,
                        initial,
                        policy,
                        4096,
                    );
                }
            }
        }
    }
}

#[test]
fn incremental_bulk_decode_overflow_and_reference_isolation() {
    let mut decoder = crate::STRICT_STANDARD_PADDED.decoder();
    decoder.set_source_position_for_test(usize::MAX - 10);
    let mut output = [0xa5; 4096];
    assert_eq!(
        decoder.update(&[b'A'; 4100], &mut output),
        Err(OperationError::Failed(Failure::PositionOverflow))
    );
    assert_eq!(output, [0xa5; 4096]);
    let mut decoder = crate::STRICT_STANDARD_PADDED.decoder();
    let before = crate::decode_validation::observation::fast_calls();
    decoder
        .update_reference(&[b'A'; 4100], &mut output)
        .unwrap();
    assert_eq!(crate::decode_validation::observation::fast_calls(), before);
    decoder.reset();
    decoder
        .update_with_validation(
            &[b'A'; 4100],
            &mut output,
            DecodeValidation::ScalarReference,
        )
        .unwrap();
    assert_eq!(crate::decode_validation::observation::fast_calls(), before);
}

#[test]
fn incremental_bulk_custom_relaxed_and_legacy_keep_scalar_planning() {
    let mut table = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    table.rotate_left(1);
    let custom = crate::CodecBuilder::from_table(table)
        .unwrap()
        .build()
        .unwrap();
    let relaxed = crate::CodecBuilder::from_table(
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .unwrap()
    .trailing_bits(crate::TrailingBits::AllowNonCanonical)
    .build()
    .unwrap();
    let before = crate::decode_validation::observation::fast_calls();
    for settings in [custom.settings(), relaxed.settings()] {
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            compare(settings, &[b'A'; 4100], 3, policy, 4096);
        }
    }
    let mut legacy =
        DecoderState::new_legacy_ascii_whitespace(crate::STRICT_STANDARD_PADDED.settings());
    let mut reference = legacy.clone();
    let mut input = [b'A'; 4100];
    for index in (0..4096).step_by(5) {
        input[index] = b'\n';
    }
    let mut actual = [0xa5; 4096];
    let mut expected = actual;
    assert_eq!(
        legacy.update(&input, &mut actual),
        reference.update_reference(&input, &mut expected)
    );
    assert_eq!(actual, expected);
    assert_eq!(legacy, reference);
    assert_eq!(legacy.finish(&mut actual), reference.finish(&mut expected));
    assert_eq!(actual, expected);
    assert_eq!(legacy, reference);
    assert_eq!(crate::decode_validation::observation::fast_calls(), before);
}

#[test]
fn incremental_bulk_miri_proof_and_pending_boundaries() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        compare(
            crate::STRICT_STANDARD_PADDED.settings(),
            &[b'A'; 520],
            1,
            policy,
            390,
        );
        let mut invalid = [b'A'; 520];
        invalid[519] = b'!';
        compare(
            crate::STRICT_STANDARD_PADDED.settings(),
            &invalid,
            4,
            policy,
            390,
        );
    }
}
