use super::*;
use std::vec::Vec;

#[test]
fn forwarding_append_rechecks_backend_after_reservation() {
    if ready_backend(4096).is_none() {
        return;
    }
    let faults = [
        Fault::Unavailable,
        Fault::WriteReject,
        #[cfg(feature = "checked-backend")]
        Fault::WriteCorrupt,
    ];
    for fault in faults {
        inject(Fault::None, || {
            let mut output = Vec::from(b"prefix");
            let written = CODEC
                .decode_append_with_hooks(
                    &[b'A'; 4096],
                    &mut output,
                    |output, required| {
                        assert_eq!(STATE.with(Cell::get).validation, 1);
                        assert_eq!(STATE.with(Cell::get).writes, 0);
                        output.try_reserve_exact(required).unwrap();
                        observe(|state| state.fault = fault);
                        Ok(())
                    },
                    |_| Ok(()),
                )
                .unwrap();
            assert_eq!(written, 3072);
            assert_eq!(&output[..6], b"prefix");
            assert_eq!(&output[6..], &[0; 3072]);
            assert_eq!(STATE.with(Cell::get).validation, 1);
            if fault == Fault::Unavailable {
                assert_eq!(STATE.with(Cell::get).writes, 0);
            } else {
                assert_quarantined(BackendFault::OutputMismatch);
            }
        });
    }
}

#[test]
fn forwarding_false_rejection_precedes_wrapper_allocation() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::Reject, || {
        let input = [b'A'; 4096];
        let mut output = Vec::from(b"prefix");
        let expected = Err(OneShotError::Backend(BackendFault::ImpossibleState));
        assert_eq!(
            CODEC.decode_append_with_hooks(
                &input,
                &mut output,
                |_, _| panic!("rejected validation must not reserve"),
                |_| panic!("rejected validation must not write"),
            ),
            expected,
        );
        assert_eq!(output, b"prefix");
        assert_eq!(
            CODEC.decode_to_vec_with_injected_reserver(&input, usize::MAX, |_, _| panic!(
                "rejected validation must not reserve"
            ),),
            expected.map(|_| Vec::new()),
        );
        assert_quarantined(BackendFault::ImpossibleState);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}

#[test]
fn forwarding_historical_owned_recovers_rejected_writer() {
    if ready_backend(4096).is_none() {
        return;
    }
    inject(Fault::WriteReject, || {
        assert_eq!(
            crate::STANDARD.decode_vec(&[b'A'; 4096]).unwrap(),
            [0; 3072]
        );
        assert_eq!(STATE.with(Cell::get).validation, 1);
        assert!(STATE.with(Cell::get).writes > 0);
        assert_quarantined(BackendFault::OutputMismatch);
    });
    #[cfg(feature = "checked-backend")]
    inject(Fault::WriteCorrupt, || {
        assert_eq!(
            crate::STANDARD.decode_vec(&[b'A'; 4096]).unwrap(),
            [0; 3072]
        );
        assert_eq!(STATE.with(Cell::get).validation, 1);
        assert_quarantined(BackendFault::OutputMismatch);
    });
}

#[test]
fn historical_preflight_faults_quarantine_without_minting_a_proof() {
    if ready_backend(4096).is_none() {
        return;
    }
    let settings = CODEC.settings();
    let input = [b'A'; 4096];
    inject(Fault::Reject, || {
        assert!(crate::v2::ordinary_decode::prepare_historical(settings, &input).is_none());
        assert_eq!(STATE.with(Cell::get).writes, 0);
        assert_quarantined(BackendFault::ImpossibleState);
    });
    #[cfg(feature = "checked-backend")]
    inject(Fault::Accept, || {
        let mut invalid = input;
        invalid[2048] = b'!';
        assert!(crate::v2::ordinary_decode::prepare_historical(settings, &invalid).is_none());
        assert_eq!(STATE.with(Cell::get).writes, 0);
        assert_quarantined(BackendFault::ImpossibleState);
    });
}

#[test]
fn historical_late_rejection_uses_only_historical_reference_validation() {
    fn check<A: crate::Alphabet, const PAD: bool>(engine: crate::Engine<A, PAD>) {
        use crate::decode_validation::observation;
        let input = engine.encode_vec(&std::vec![0x5a; 65536]).unwrap();
        for position in [0, 64, input.len() - 1] {
            let mut input = input.clone();
            input[position] = b'!';
            let before = observation::canonical_calls();
            let mut expected = std::vec![0xa5; 65544];
            let reference = engine.decode_slice_with_validation(
                &input,
                &mut expected,
                DecodeValidation::ScalarReference,
            );
            let mut actual = std::vec![0xa5; 65544];
            assert_eq!(engine.decode_slice(&input, &mut actual), reference);
            assert_eq!(actual, expected);
            assert_eq!(
                engine.decode_vec(&input),
                engine.decode_vec_with_validation(&input, DecodeValidation::ScalarReference,)
            );
            assert_eq!(
                engine.validate_result(&input),
                engine.validate_result_with_validation(&input, DecodeValidation::ScalarReference,)
            );
            let expected_error = crate::validate_decode::<A, PAD>(&input).unwrap_err();
            let mut in_place = input.clone();
            assert_eq!(engine.decode_in_place(&mut in_place), Err(expected_error));
            assert_eq!(in_place, input);
            assert_eq!(observation::canonical_calls(), before);
        }
    }
    ready_backend(4096);
    check(crate::STANDARD);
    check(crate::STANDARD_NO_PAD);
    check(crate::URL_SAFE);
    check(crate::URL_SAFE_NO_PAD);
}
