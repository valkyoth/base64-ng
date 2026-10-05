use super::*;

#[cfg(feature = "stream")]
#[test]
fn stream_bulk_quarantines_faults_without_hiding_errors_or_partial_stores() {
    use std::io::Write;
    if ready_backend(1360).is_none() {
        return;
    }
    inject(Fault::Reject, || {
        let mut decoder = crate::stream::Decoder::new(Vec::new(), crate::STANDARD);
        assert!(decoder.write(&[b'A'; 1364]).is_err());
        assert!(decoder.is_failed());
        assert_eq!(decoder.buffered_output_len(), 0);
        assert_eq!(decoder.get_ref().as_slice(), []);
        assert_quarantined(BackendFault::ImpossibleState);
    });
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        inject(Fault::WriteReject, || {
            let mut decoder =
                crate::stream::Decoder::new(Vec::new(), crate::STANDARD).with_validation(policy);
            assert_eq!(decoder.write(&[b'A'; 1364]).unwrap(), 1364);
            assert_eq!(
                STATE.with(Cell::get).validation,
                usize::from(policy == DecodeValidation::Auto)
            );
            assert!(STATE.with(Cell::get).writes > 0);
            assert_quarantined(BackendFault::OutputMismatch);
            assert_eq!(decoder.finish().unwrap(), vec![0; 1023]);
        });
    }
}

#[cfg(fuzzing)]
#[test]
fn incremental_fuzz_scalar_oracle_never_validates_or_writes_through_bulk() {
    if ready_backend(8192).is_none() {
        return;
    }
    inject(Fault::WriteReject, || {
        let mut decoder = CODEC.decoder();
        let mut output = [0xa5; 6160];
        let step = decoder
            .update_scalar_oracle(&[b'A'; 8196], &mut output)
            .unwrap();
        assert_eq!(step.progress().input_consumed(), 8196);
        assert_eq!(step.progress().output_produced(), 6147);
        assert_eq!(&output[..6147], &[0; 6147]);
        assert_eq!(&output[6147..], &[0xa5; 13]);
        assert_eq!(STATE.with(Cell::get).validation, 0);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}

#[test]
fn incremental_bulk_decode_recovers_writes_after_pending_prefix() {
    if ready_backend(8192).is_none() {
        return;
    }
    let input = [b'A'; 8196];
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for fault in [
            Fault::Unavailable,
            Fault::WriteReject,
            #[cfg(feature = "checked-backend")]
            Fault::WriteCorrupt,
        ] {
            let mut decoder = CODEC.decoder();
            decoder.update(b"AAAA", &mut [0; 1]).unwrap();
            let mut reference = decoder.clone();
            let mut expected = [0xa5; 6160];
            let expected_step = reference.update_reference(&input, &mut expected).unwrap();
            inject(fault, || {
                let mut actual = [0xa5; 6160];
                assert_eq!(
                    decoder.update_with_validation(&input, &mut actual, policy),
                    Ok(expected_step)
                );
                assert_eq!(actual, expected);
                assert_eq!(decoder, reference);
                let state = STATE.with(Cell::get);
                assert_eq!(
                    state.validation,
                    usize::from(policy == DecodeValidation::Auto)
                );
                if fault == Fault::Unavailable {
                    assert_eq!(state.writes, 0);
                } else {
                    assert!(state.writes > 0);
                    assert_quarantined(BackendFault::OutputMismatch);
                }
            });
        }
    }
}

#[test]
fn incremental_bulk_validation_faults_precede_all_output_and_position_commit() {
    if ready_backend(8192).is_none() {
        return;
    }
    check_validation_fault(Fault::Reject);
    #[cfg(feature = "checked-backend")]
    check_validation_fault(Fault::Accept);
}

fn check_validation_fault(fault: Fault) {
    let mut decoder = CODEC.decoder();
    decoder.update(b"AAAA", &mut [0; 1]).unwrap();
    inject(fault, || {
        let mut input = [b'A'; 8196];
        if fault != Fault::Reject {
            input[100] = b'!';
        }
        let mut output = [0xa5; 6160];
        let error = decoder.update(&input, &mut output).unwrap_err();
        assert_eq!(
            error,
            crate::OperationError::Failed(crate::Failure::Backend(BackendFault::ImpossibleState))
        );
        assert_eq!(output, [0xa5; 6160]);
        assert_eq!(decoder.source_position(), 4);
        assert_eq!(decoder.update(&[], &mut output), Err(error));
        assert_eq!(decoder.finish(&mut output), Err(error));
        assert_eq!(output, [0xa5; 6160]);
        assert_eq!(STATE.with(Cell::get).writes, 0);
        assert_quarantined(BackendFault::ImpossibleState);
    });
}

#[test]
fn incremental_bulk_late_invalid_tail_discards_the_planned_writer() {
    if ready_backend(8192).is_none() {
        return;
    }
    inject(Fault::WriteReject, || {
        let mut input = [b'A'; 8196];
        input[8195] = b'!';
        let mut output = [0xa5; 6160];
        let mut decoder = CODEC.decoder();
        assert!(decoder.update(&input, &mut output).is_err());
        assert_eq!(decoder.source_position(), 0);
        assert_eq!(output, [0xa5; 6160]);
        assert_eq!(STATE.with(Cell::get).validation, 1);
        assert_eq!(STATE.with(Cell::get).writes, 0);
    });
}
