use std::vec::Vec;

fn payload() -> Vec<u8> {
    (0..8192).map(|i| (i as u8).wrapping_mul(73)).collect()
}

fn secret_boundary() {
    let report = base64_ng::runtime::backend_report();
    assert_eq!(
        report.secret_decode_backend.backend.as_str(),
        "scalar-constant-time-oriented"
    );
}
