use base64_ng::{
    DecodeValidation, Failure, InputError, OperationError, STRICT_STANDARD_PADDED as CODEC,
};

#[test]
fn production_incremental_bulk_is_transactional_per_call_not_per_message() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let mut decoder = CODEC.decoder();
        let mut output = [0xa5; 4096];
        let first = decoder.update(b"AAAA", &mut output[..1]).unwrap();
        assert_eq!(first.progress().input_consumed(), 4);
        assert_eq!(output[0], 0);
        output.fill(0xa5);
        let mut input = [b'A'; 4100];
        input[4099] = b'!';
        let error = decoder
            .update_with_validation(&input, &mut output, policy)
            .unwrap_err();
        assert_eq!(
            error,
            OperationError::Failed(Failure::Input(InputError::InvalidByte {
                index: 4103,
                byte: b'!'
            }))
        );
        assert_eq!(decoder.source_position(), 4);
        assert_eq!(output, [0xa5; 4096]);
        assert_eq!(decoder.finish(&mut output), Err(error));
        decoder.reset();
        input[4099] = b'A';
        let step = decoder
            .update_with_validation(&input, &mut output, policy)
            .unwrap();
        assert_eq!(step.progress().input_consumed(), 4100);
        assert_eq!(step.progress().output_produced(), 3075);
        assert_eq!(output[..3075], [0; 3075]);
        assert_eq!(output[3075..], [0xa5; 1021]);
    }
}

#[test]
fn production_incremental_bulk_does_not_validate_unaccepted_suffix() {
    let mut input = [b'A'; 4100];
    input[520] = b'!';
    let mut decoder = CODEC.decoder();
    let mut output = [0xa5; 384];
    let step = decoder.update(&input, &mut output).unwrap();
    assert_eq!(step.progress().input_consumed(), 516);
    assert_eq!(step.progress().output_produced(), 384);
    assert_eq!(output, [0; 384]);
    output.fill(0xa5);
    assert!(decoder.update(&input[516..], &mut output).is_err());
    assert_eq!(output, [0xa5; 384]);
    assert_eq!(decoder.source_position(), 516);
}
