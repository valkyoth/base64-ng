use super::v2::{
    secret::{SecretArrayFrame, SecretDecodeError, SecretFrame, SecretInput},
    specifications::STRICT_STANDARD_PADDED,
};

#[kani::proof]
#[kani::unwind(70)]
fn secret_frame_never_releases_more_than_its_bound() {
    let input = kani::any::<[u8; 4]>();
    let mut frame = SecretArrayFrame::<3>::new(&STRICT_STANDARD_PADDED)
        .expect("strict policy and bounded frame are valid");
    let progress = frame
        .update(&SecretInput::new(&input))
        .expect("one padded quantum fits the public bound");
    assert!(progress.input_consumed() == input.len());
    assert!(progress.output_produced() == 0);
    if let Ok(secret) = frame.finish() {
        assert!(secret.len() <= 3);
        assert!(
            secret.backing_for_proof()[secret.len()..]
                .iter()
                .all(|byte| *byte == 0)
        );
    }
}

#[kani::proof]
fn secret_frame_rejects_oversized_input_before_scanning() {
    let mut frame = SecretArrayFrame::<3>::new(&STRICT_STANDARD_PADDED)
        .expect("strict policy and bounded frame are valid");
    let error = frame
        .update(&SecretInput::new(b"QUJDRA=="))
        .expect_err("eight encoded bytes exceed a three-byte frame");
    assert!(matches!(error, SecretDecodeError::InputTooLarge { .. }));
    assert!(frame.state().is_failed());
}

#[kani::proof]
#[kani::unwind(70)]
fn borrowed_secret_frame_writes_public_output_only_after_valid_gate() {
    let input = kani::any::<[u8; 4]>();
    let mut staging = [0xa5; 3];
    let mut output = [0xa5; 3];
    let accepted = {
        let mut frame = SecretFrame::new(&STRICT_STANDARD_PADDED, 3, &mut staging, &mut output)
            .expect("fixed ranges are disjoint and sufficient");
        frame
            .update(&SecretInput::new(&input))
            .expect("one padded quantum fits the public bound");
        match frame.finish() {
            Ok(secret) => {
                assert!(secret.len() <= 3);
                drop(secret);
                true
            }
            Err(SecretDecodeError::InvalidInput) => false,
            Err(error) => panic!("unexpected public-frame proof result: {error}"),
        }
    };
    assert!(staging.iter().all(|byte| *byte == 0));
    assert!(output.iter().all(|byte| *byte == 0));
    kani::cover!(accepted, "valid input reaches the release gate");
    kani::cover!(!accepted, "invalid input wipes without release");
}
