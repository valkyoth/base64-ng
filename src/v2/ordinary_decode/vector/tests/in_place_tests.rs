use super::*;
use crate::Alphabet;

#[test]
fn in_place_bulk_recovers_after_committed_chunks_and_health_changes() {
    if ready_backend(8192).is_none() {
        return;
    }
    let mut input = [b'A'; 8192];
    for (index, byte) in input.iter_mut().enumerate() {
        *byte = crate::Standard::ENCODE[index % 64];
    }
    let mut expected = input;
    crate::scalar::decode_slice::<crate::Standard, true>(&input, &mut expected).unwrap();
    for after in [0, 1, 3, 6] {
        for fault in [
            Fault::WriteReject,
            Fault::Unavailable,
            #[cfg(feature = "checked-backend")]
            Fault::WriteCorrupt,
        ] {
            for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
                inject_after(fault, after, || {
                    let mut actual = input;
                    assert_eq!(
                        crate::v2::ordinary_decode::in_place::decode(
                            CODEC.settings(),
                            &mut actual,
                            policy
                        ),
                        Ok(6144)
                    );
                    assert_eq!(actual, expected);
                    let state = STATE.with(Cell::get);
                    assert_eq!(
                        state.validation,
                        usize::from(policy == DecodeValidation::Auto)
                    );
                    if fault == Fault::Unavailable {
                        assert_eq!(state.writes, after);
                    } else {
                        assert!(state.writes > after);
                        assert_quarantined(BackendFault::OutputMismatch);
                    }
                });
            }
        }
    }
}

#[test]
fn in_place_bulk_validation_disagreements_and_late_errors_never_write() {
    if ready_backend(8192).is_none() {
        return;
    }
    let check = |fault| {
        inject(fault, || {
            let mut input = [b'A'; 8192];
            if fault != Fault::Reject {
                input[1024] = b'!';
            }
            let before = input;
            assert!(CODEC.decode_in_place(&mut input, 8192).is_err());
            assert_eq!(input, before);
            assert_eq!(STATE.with(Cell::get).writes, 0);
            assert_quarantined(BackendFault::ImpossibleState);
        });
    };
    check(Fault::Reject);
    #[cfg(feature = "checked-backend")]
    check(Fault::Accept);
    inject(Fault::WriteReject, || {
        let mut input = [b'A'; 8192];
        input[8191] = b'!';
        let before = input;
        assert!(CODEC.decode_in_place(&mut input, 8192).is_err());
        assert_eq!(input, before);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}

#[test]
fn in_place_bulk_reaches_a_real_writer_in_both_public_surfaces() {
    if ready_backend(8192).is_none() {
        return;
    }
    for legacy in [false, true] {
        inject(Fault::None, || {
            let mut input = [b'A'; 8192];
            let n = if legacy {
                crate::STANDARD.decode_in_place(&mut input).unwrap().len()
            } else {
                CODEC.decode_in_place(&mut input, 8192).unwrap()
            };
            assert_eq!(n, 6144);
            assert_eq!(input[..n], [0; 6144]);
            assert_eq!(input[n..], [b'A'; 2048]);
            assert_eq!(STATE.with(Cell::get).validation, 1);
            assert!(STATE.with(Cell::get).writes > 0);
        });
    }
}
