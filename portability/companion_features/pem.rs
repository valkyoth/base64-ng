#[test]
fn companion_round_trip_and_rejection() {
    use companion::*;
    let plain = payload();
    let label = PemLabel::new("CERTIFICATE").unwrap();
    let text = encode_pem_block_to_string(
        &label,
        &plain,
        PemLimits::default(),
        PemGenerationOptions::default(),
    )
    .unwrap();
    let doc = parse_pem_document(
        text.as_bytes(),
        PemLimits::default(),
        PemParsePolicy::Strict,
    )
    .unwrap();
    assert_eq!(doc.blocks()[0].contents(), plain);
    let invalid = text.replacen("-----END", "-----BAD", 1);
    assert!(
        parse_pem_document(
            invalid.as_bytes(),
            PemLimits::default(),
            PemParsePolicy::Strict
        )
        .is_err()
    );
    secret_boundary();
}
