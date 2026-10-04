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
