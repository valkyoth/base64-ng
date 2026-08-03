use crate::{
    OpenPgpErrorKind, OpenPgpLimits,
    parser::{BodyValidation, parse_raw_document},
};

#[test]
fn secret_mode_defers_every_body_symbol_position_to_the_fixed_work_decoder() {
    const TEMPLATE: &[u8] =
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----\n\nQUJDREVGR0hJSktM\n-----END PGP PRIVATE KEY BLOCK-----\n";
    let body_start = TEMPLATE
        .windows(16)
        .position(|window| window == b"QUJDREVGR0hJSktM")
        .unwrap();

    for offset in 0..16 {
        let mut input = TEMPLATE.to_vec();
        input[body_start + offset] = b'!';
        let Err(ordinary_error) =
            parse_raw_document(&input, OpenPgpLimits::default(), BodyValidation::Ordinary)
        else {
            panic!("ordinary parsing accepted malformed body byte at {offset}");
        };
        assert_eq!(ordinary_error.kind(), OpenPgpErrorKind::InvalidBody);
        let deferred = parse_raw_document(
            &input,
            OpenPgpLimits::default(),
            BodyValidation::DeferredSecret,
        )
        .unwrap();
        assert_eq!(deferred.blocks[0].body[offset], b'!');
        assert_eq!(deferred.blocks[0].body.len(), 16);
    }
}
