use base64_ng::runtime::Backend;
use base64_ng::StaticBackendToken;

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
    let backend = compiled_backend();
    assert!(runtime_supports(backend), "host lacks compiled backend");
    // SAFETY: The runtime probe above proves this thread's CPU supports the
    // exact compile-time-selected backend. The process does not migrate the
    // test thread before the token and operations are dropped.
    let token = unsafe { StaticBackendToken::assume_supported(backend) }
        .expect("static backend KAT must pass");
    let mut output = [0u8; 64];
    let written = token.encode_standard::<false>(&INPUT, &mut output).unwrap();
    assert_eq!(written, STANDARD.len());
    assert_eq!(&output[..written], STANDARD);
    let written = token.encode_url_safe::<false>(&INPUT, &mut output).unwrap();
    assert_eq!(written, URL_SAFE.len());
    assert_eq!(&output[..written], URL_SAFE);
    println!("static no_std encode: {} ok", backend.as_str());
}

const fn compiled_backend() -> Backend {
    if cfg!(all(
        target_feature = "avx512f",
        target_feature = "avx512bw",
        target_feature = "avx512vl",
        target_feature = "avx512vbmi"
    )) {
        Backend::Avx512Vbmi
    } else if cfg!(target_feature = "avx2") {
        Backend::Avx2
    } else {
        Backend::Ssse3Sse41
    }
}

fn runtime_supports(backend: Backend) -> bool {
    match backend {
        Backend::Avx512Vbmi => {
            std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512bw")
                && std::is_x86_feature_detected!("avx512vl")
                && std::is_x86_feature_detected!("avx512vbmi")
        }
        Backend::Avx2 => std::is_x86_feature_detected!("avx2"),
        Backend::Ssse3Sse41 => {
            std::is_x86_feature_detected!("ssse3") && std::is_x86_feature_detected!("sse4.1")
        }
        _ => false,
    }
}
