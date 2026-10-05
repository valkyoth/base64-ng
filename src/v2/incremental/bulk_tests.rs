extern crate std;
use super::*;
use std::vec;

#[test]
fn incremental_bulk_encode_planner_matches_original_loop_and_bounds() {
    fn reference(mut tail: usize, pending: usize, input: usize, output: usize) -> usize {
        if pending > output {
            return 0;
        }
        let mut output = output - pending;
        let mut consumed = 0;
        while consumed < input {
            let copied = (3 - tail).min(input - consumed);
            consumed += copied;
            tail += copied;
            if tail != 3 {
                break;
            }
            tail = 0;
            let written = 4.min(output);
            output -= written;
            if written != 4 {
                break;
            }
        }
        consumed
    }
    for tail in 0..3 {
        for pending in 0..=4 {
            for input in 0..70 {
                for output in 0..90 {
                    assert_eq!(
                        planned_input_consumption(tail, pending, input, output),
                        reference(tail, pending, input, output)
                    );
                }
            }
            assert!(
                planned_input_consumption(tail, pending, usize::MAX, usize::MAX)
                    <= usize::MAX / 4 * 3 + 3
            );
            assert_eq!(planned_input_consumption(tail, pending, 9, usize::MAX), 9);
        }
    }
}

#[test]
fn incremental_bulk_encode_matches_scalar_state_at_every_step() {
    let mut table = *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    table.rotate_left(1);
    let custom = crate::CodecBuilder::from_table(table)
        .unwrap()
        .build()
        .unwrap();
    for settings in [
        crate::STRICT_STANDARD_PADDED.settings(),
        crate::STRICT_STANDARD_UNPADDED.settings(),
        crate::STRICT_URL_SAFE_PADDED.settings(),
        crate::STRICT_URL_SAFE_UNPADDED.settings(),
        custom.settings(),
    ] {
        let input: std::vec::Vec<_> = (0..8195).map(|i| u8::try_from(i % 251).unwrap()).collect();
        for initial in 0..3 {
            for chunks in [[8195, 8195], [191, 257], [1, 4096], [3, 193]] {
                let mut actual = EncoderState::new(settings);
                let mut reference = actual.clone();
                assert_eq!(
                    actual.update(&input[..initial], &mut []),
                    reference.update_impl::<false>(&input[..initial], &mut [])
                );
                let mut offset = initial;
                let mut turn = 0;
                while offset < input.len() {
                    let end = input.len().min(offset + chunks[turn % 2]);
                    let capacity = [0, 1, 2, 3, 4, 5, 255, 256, 1024, 16384][turn % 10];
                    let mut a = vec![0xa5; capacity + 9];
                    let mut b = a.clone();
                    let step = actual.update(&input[offset..end], &mut a[..capacity]);
                    assert_eq!(
                        step,
                        reference.update_impl::<false>(&input[offset..end], &mut b[..capacity])
                    );
                    assert_eq!(a, b);
                    assert_eq!(actual, reference);
                    offset += step.unwrap().progress().input_consumed();
                    turn += 1;
                }
                for capacity in [0, 1, 1, 1, 5, 10] {
                    let mut a = [0xa5; 10];
                    let mut b = a;
                    assert_eq!(
                        actual.finish(&mut a[..capacity]),
                        reference.finish(&mut b[..capacity])
                    );
                    assert_eq!(a, b);
                    assert_eq!(actual, reference);
                }
                actual.reset();
                reference.reset();
                assert_eq!(actual, reference);
            }
        }
    }
}

#[test]
fn incremental_bulk_encode_miri_boundaries() {
    let mut state = crate::STRICT_STANDARD_PADDED.encoder();
    state.update(&[0], &mut []).unwrap();
    let mut reference = state.clone();
    let input = [0; 200];
    let mut actual = [0xa5; 270];
    let mut expected = actual;
    assert_eq!(
        state.update(&input, &mut actual),
        reference.update_impl::<false>(&input, &mut expected)
    );
    assert_eq!(actual, expected);
    assert_eq!(state, reference);
    assert_eq!(state.finish(&mut actual), reference.finish(&mut expected));
    assert_eq!(actual, expected);
    assert_eq!(state, reference);
}
