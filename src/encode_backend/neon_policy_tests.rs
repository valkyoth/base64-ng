#[test]
fn automatic_policy_uses_scalar_below_measured_crossover() {
    assert!(!super::neon_auto_preferred(
        super::NEON_ENCODE_AUTO_MIN_INPUT - 1
    ));
    assert!(super::neon_auto_preferred(
        super::NEON_ENCODE_AUTO_MIN_INPUT
    ));
}
