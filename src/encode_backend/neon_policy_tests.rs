use super::EncodeBackend;

#[test]
fn automatic_policy_uses_scalar_below_measured_crossover() {
    assert_eq!(
        super::active_encode_backend_for_input(super::NEON_ENCODE_AUTO_MIN_INPUT - 1),
        EncodeBackend::Scalar
    );
    assert_eq!(
        super::active_encode_backend_for_input(super::NEON_ENCODE_AUTO_MIN_INPUT),
        EncodeBackend::Neon
    );
}
