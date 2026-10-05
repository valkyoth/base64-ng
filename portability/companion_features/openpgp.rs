#[test]
fn companion_round_trip_and_rejection() {
    use companion::*;
    let plain = payload();
    let text = encode_armor_to_string(
        ArmorType::Message,
        &[],
        &plain,
        OpenPgpLimits::default(),
        GenerationOptions::new(ChecksumGeneration::Omit),
    )
    .unwrap();
    let doc = parse_armor_document(
        text.as_bytes(),
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap();
    assert_eq!(doc.blocks()[0].contents(), plain);
    let invalid = text.replacen("-----END", "-----BAD", 1);
    assert!(
        parse_armor_document(
            invalid.as_bytes(),
            OpenPgpLimits::default(),
            ChecksumPolicy::Rfc9580
        )
        .is_err()
    );
    secret_boundary();
}
