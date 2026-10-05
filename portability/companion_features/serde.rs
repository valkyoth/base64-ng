#[test]
fn companion_round_trip_and_rejection() {
    let plain = payload();
    let value = companion::Base64Standard::new(plain.clone());
    let json = serde_json::to_string(&value).unwrap();
    let decoded: companion::Base64Standard = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.as_bytes(), plain);
    let invalid = json.replacen('=', "!", 1);
    assert!(serde_json::from_str::<companion::Base64Standard>(&invalid).is_err());
    secret_boundary();
}
