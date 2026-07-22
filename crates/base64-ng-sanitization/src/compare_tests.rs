#[cfg(all(
    feature = "std",
    feature = "memory-lock",
    any(
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ),
        target_os = "macos",
        target_os = "ios",
        target_os = "android",
        target_os = "windows",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly",
        all(target_arch = "wasm32", feature = "wasm-compat"),
    )
))]
#[test]
fn compatibility_integrity_failure_panics() {
    let result = std::panic::catch_unwind(|| {
        crate::compare::panic_on_locked_integrity_failure(sanitization::CanaryCorruptedError);
    });

    assert!(result.is_err());
}
