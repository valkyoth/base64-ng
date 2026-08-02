use base64_ng::StaticBackendToken;
use base64_ng::runtime::Backend;

const INPUT: [u8; 48] = [
    0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00,
    0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80,
    0x40, 0x55, 0xaa, 0x33, 0xfb, 0xff, 0xef, 0x00, 0x10, 0x83, 0x7f, 0x80, 0x40, 0x55, 0xaa, 0x33,
];
const STANDARD: &[u8; 64] =
    b"+//vABCDf4BAVaoz+//vABCDf4BAVaoz+//vABCDf4BAVaoz+//vABCDf4BAVaoz";
const URL_SAFE: &[u8; 64] =
    b"-__vABCDf4BAVaoz-__vABCDf4BAVaoz-__vABCDf4BAVaoz-__vABCDf4BAVaoz";

fn main() {
    assert!(cfg!(target_arch = "aarch64"));
    assert!(cfg!(target_endian = "little"));
    let token = StaticBackendToken::for_compiled_target()
        .expect("static AArch64 NEON KAT must pass");
    assert_eq!(token.backend(), Backend::Neon);

    let mut encoded = [0u8; 64];
    let written = token
        .encode_standard::<false>(&INPUT, &mut encoded)
        .unwrap();
    assert_eq!(&encoded[..written], STANDARD);
    let written = token
        .encode_url_safe::<false>(&INPUT, &mut encoded)
        .unwrap();
    assert_eq!(&encoded[..written], URL_SAFE);

    let mut decoded = [0u8; 48];
    let written = token
        .decode_standard::<false>(STANDARD, &mut decoded)
        .unwrap();
    assert_eq!(written, INPUT.len());
    assert_eq!(decoded, INPUT);
    let written = token
        .decode_url_safe::<false>(URL_SAFE, &mut decoded)
        .unwrap();
    assert_eq!(written, INPUT.len());
    assert_eq!(decoded, INPUT);
    println!("static no_std encode/decode: neon ok");
}
