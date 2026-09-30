use super::{Failure, Layout, Preflight};

#[test]
fn layout_bounds_match_an_independent_small_integer_model() {
    for input in 0..=256 {
        for output in 0..=256 {
            let body = if input == 0 { 0 } else { (input - 1) / 4 * 4 };
            let body_output = body / 4 * 3;
            let model = output >= body_output
                && match input - body {
                    0 => output == 0,
                    2 => output - body_output == 1,
                    3 => (1..=2).contains(&(output - body_output)),
                    4 => (1..=3).contains(&(output - body_output)),
                    _ => false,
                };
            let actual = Layout::checked(input, output);
            assert_eq!(actual.is_some(), model);
            if let Some(layout) = actual {
                assert_eq!(layout.interior_input, body);
                assert_eq!(layout.interior_output, body_output);
                assert_eq!(layout.decoded_len, output);
            }
        }
    }
}

#[test]
fn length_arithmetic_is_bounded_at_usize_max_without_allocating() {
    for input in (usize::MAX - 16)..=usize::MAX {
        let body = (input - 1) / 4 * 4;
        let output = body / 4 * 3;
        assert!(Layout::checked(input, usize::MAX).is_none());
        for tail_output in 0..=4 {
            if let Some(layout) = Layout::checked(input, output + tail_output) {
                assert!(layout.interior_input <= input);
                assert_eq!(layout.interior_input % 4, 0);
                assert!(input - layout.interior_input <= 4);
                assert!(layout.interior_output <= layout.decoded_len);
                assert!(layout.decoded_len - layout.interior_output <= 3);
            }
        }
    }
    assert!(Layout::checked(0, usize::MAX).is_none());
    assert!(Layout::checked(1, 0).is_none());
}

#[test]
fn proof_binds_source_configuration_and_writer_spans() {
    let source = *b"Zm9vYg==";
    let proof = Preflight::reference(&source, 42, |config, input| {
        assert_eq!(config, 42);
        assert!(core::ptr::eq(input, source.as_slice()));
        Ok::<_, ()>(4)
    })
    .unwrap();
    let mut output = [0xa5; 8];
    assert_eq!(
        proof
            .write(&mut output, |config, body, tail, body_out, tail_out| {
                assert_eq!(config, 42);
                assert_eq!(body, b"Zm9v");
                assert_eq!(tail, b"Yg==");
                assert_eq!((body_out.len(), tail_out.len()), (3, 1));
                body_out.copy_from_slice(b"foo");
                tail_out.copy_from_slice(b"b");
            })
            .unwrap(),
        4
    );
    assert_eq!(&output, b"foob\xa5\xa5\xa5\xa5");
}

#[test]
fn no_writer_on_capacity_error_or_inconsistent_validation_length() {
    let proof = Preflight::reference(b"Zm9v", (), |(), _| Ok::<_, ()>(3)).unwrap();
    let mut output = [0xa5; 2];
    let error = proof
        .write(&mut output, |(), _, _, _, _| panic!("must not write"))
        .unwrap_err();
    assert_eq!((error.required, error.available), (3, 2));
    assert_eq!(output, [0xa5; 2]);
    assert!(matches!(
        Preflight::reference(b"Zm9v", (), |(), _| Ok::<_, ()>(4)),
        Err(Failure::Bounds)
    ));
}

#[test]
fn classifier_disagreement_never_authorizes_a_writer() {
    for accepted in [false, true] {
        let reference = if accepted { Err(17) } else { Ok(3) };
        assert!(matches!(
            Preflight::classified_for_test(b"Zm9v", (), |(), _| reference, accepted),
            Err(Failure::ClassifierDisagreement)
        ));
    }
    assert!(matches!(
        Preflight::classified_for_test(b"!!!!", (), |(), _| Err::<usize, _>(17), false),
        Err(Failure::Input(17))
    ));
}
