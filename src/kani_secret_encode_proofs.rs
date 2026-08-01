use super::v2::{
    final_quantum_output_len_for_proof, require_disjoint_ranges_for_proof,
    secret::{SecretArrayEncoder, SecretEncodeError, SecretInput},
    specifications::{EncodePadding, STRICT_STANDARD_PADDED},
};

#[kani::proof]
fn secret_encoder_final_quantum_output_is_bounded() {
    let tail_len = kani::any::<usize>();
    kani::assume((1..=2).contains(&tail_len));
    let padding = if kani::any::<bool>() {
        EncodePadding::Padded
    } else {
        EncodePadding::Unpadded
    };
    let produced = final_quantum_output_len_for_proof(tail_len, padding);
    assert!((2..=4).contains(&produced));
    if matches!(padding, EncodePadding::Padded) {
        assert!(produced == 4);
    } else {
        assert!(produced == tail_len + 1);
    }
}

#[kani::proof]
fn secret_encoder_rejects_oversized_input_absorbingly() {
    let mut encoder =
        SecretArrayEncoder::<8>::new(&STRICT_STANDARD_PADDED, 3).expect("public bounds fit");
    let error = encoder
        .update(&SecretInput::new(b"four"))
        .expect_err("four bytes exceed a three-byte input bound");
    assert!(matches!(error, SecretEncodeError::InputTooLarge { .. }));
    assert!(encoder.state().is_failed());
}

#[kani::proof]
fn secret_encoder_overlap_preflight_fails_closed() {
    let start = kani::any::<usize>();
    let len = kani::any::<usize>();
    kani::assume(len != 0);
    let result = require_disjoint_ranges_for_proof(start, len, start, len);
    if start.checked_add(len).is_some() {
        assert!(matches!(result, Err(SecretEncodeError::OverlappingBuffers)));
    } else {
        assert!(matches!(
            result,
            Err(SecretEncodeError::AddressRangeOverflow)
        ));
    }
}
