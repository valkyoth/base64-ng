use super::{
    Failure, InputError, OneShotError, OperationError, STRICT_STANDARD_PADDED, Status,
    legacy::ASCII_WHITESPACE,
};

const IGNORED: [u8; 4] = *b" \t\r\n";

#[test]
fn exactly_four_ascii_bytes_are_ignored_at_every_position() {
    for byte in 0u8..=127 {
        for position in 0..=4 {
            let mut input = [0u8; 5];
            input[..position].copy_from_slice(&b"Zg=="[..position]);
            input[position] = byte;
            input[position + 1..].copy_from_slice(&b"Zg=="[position..]);

            let mut actual = [0xa5; 4];
            let actual_result =
                ASCII_WHITESPACE.decode_into(&STRICT_STANDARD_PADDED, &input, &mut actual);
            if IGNORED.contains(&byte) {
                assert_eq!(actual_result, Ok(1), "byte={byte:#04x} position={position}");
                assert_eq!(actual[0], b'f');
            } else {
                let mut expected = [0xa5; 4];
                let expected_result = STRICT_STANDARD_PADDED.decode_into(&input, &mut expected);
                assert_eq!(
                    actual_result, expected_result,
                    "byte={byte:#04x} position={position}"
                );
                assert_eq!(actual, expected, "byte={byte:#04x} position={position}");
            }
        }
    }
}

#[test]
fn whitespace_only_chunks_advance_original_positions() {
    let mut decoder = ASCII_WHITESPACE.decoder(&STRICT_STANDARD_PADDED);
    let first = decoder.update(b" \t", &mut []).unwrap();
    assert_eq!(first.progress().input_consumed(), 2);
    assert_eq!(first.status(), Status::NeedInput);

    let second = decoder.update(b"\r\nZg", &mut []).unwrap();
    assert_eq!(second.progress().input_consumed(), 4);
    let error = decoder.update(b"!", &mut []).unwrap_err();
    assert_eq!(
        error,
        OperationError::Failed(Failure::Input(InputError::InvalidByte {
            index: 6,
            byte: b'!'
        }))
    );
    assert_eq!(decoder.update(b"==", &mut []).unwrap_err(), error);
}

#[test]
fn incremental_chunks_preserve_output_and_terminal_whitespace() {
    let mut decoder = ASCII_WHITESPACE.decoder(&STRICT_STANDARD_PADDED);
    let mut output = [0u8; 3];
    assert_eq!(
        decoder.update(b"Z \n", &mut output[..0]).unwrap().status(),
        Status::NeedInput
    );
    let second = decoder.update(b"g=\t=\r\n", &mut output[..1]).unwrap();
    assert_eq!(second.progress().input_consumed(), 6);
    assert_eq!(second.progress().output_produced(), 1);
    assert_eq!(output[0], b'f');
    assert_eq!(
        decoder.finish(&mut output[1..]).unwrap().status(),
        Status::Complete
    );
}

#[test]
fn one_shot_validation_is_transactional_and_reports_original_indexes() {
    let input = b" \tZg\r!\n==";
    let mut output = [0xa5; 8];
    assert_eq!(
        ASCII_WHITESPACE.decode_into(&STRICT_STANDARD_PADDED, input, &mut output),
        Err(OneShotError::Input(InputError::InvalidByte {
            index: 5,
            byte: b'!'
        }))
    );
    assert_eq!(output, [0xa5; 8]);

    for rejected in *b"\x0b\x0c" {
        let input = [b'Z', b'g', rejected, b'=', b'='];
        assert!(
            ASCII_WHITESPACE
                .validate(&STRICT_STANDARD_PADDED, &input)
                .is_err()
        );
    }
}

#[test]
fn source_position_overflow_is_absorbing_before_whitespace_compaction() {
    let mut decoder = ASCII_WHITESPACE.decoder(&STRICT_STANDARD_PADDED);
    decoder.set_source_position_for_test(usize::MAX - 1);
    let error = decoder.update(b" \t", &mut []).unwrap_err();
    assert_eq!(error, OperationError::Failed(Failure::PositionOverflow));
    assert_eq!(decoder.update(b"Zg==", &mut []).unwrap_err(), error);
}
