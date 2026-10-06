//! Native Windows development probe, not a release-admission certificate.
#[cfg(target_arch = "x86_64")]
mod abi;

#[cfg(target_arch = "x86_64")]
fn main() {
    let ssse3 = std::is_x86_feature_detected!("ssse3") && std::is_x86_feature_detected!("sse4.1");
    let avx2 = std::is_x86_feature_detected!("avx2");
    let avx512 = std::is_x86_feature_detected!("avx512f")
        && std::is_x86_feature_detected!("avx512bw")
        && std::is_x86_feature_detected!("avx512vl")
        && std::is_x86_feature_detected!("avx512vbmi");
    abi::exercise();
    println!(
        "{{\"native_windows_msvc\":{},\"ssse3_sse41\":{ssse3},\"avx2\":{avx2},\"avx512_vbmi\":{avx512},\"xmm6_xmm15\":\"passed\"}}",
        cfg!(all(target_os = "windows", target_env = "msvc"))
    );
}

#[cfg(not(target_arch = "x86_64"))]
fn main() {
    panic!("this fixture requires x86_64");
}
