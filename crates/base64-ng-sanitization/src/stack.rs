/// Maximum secret capacity allocated on the stack by this companion crate.
///
/// This matches `base64-ng`'s core secret-frame ceiling. Larger dynamic
/// secrets must use a bounded heap or protected-mapping API.
///
/// ```compile_fail
/// use base64_ng::ct;
/// use base64_ng_sanitization::CtDecodeSanitizationExt;
///
/// let _ = ct::STANDARD.decode_secret_bytes::<1025>(b"");
/// ```
///
/// ```compile_fail
/// use base64_ng::ct;
/// use base64_ng_sanitization::CtDecodeSanitizationExt;
///
/// let _ = ct::STANDARD.decode_secret_vec_staged::<1025>(b"");
/// ```
pub const MAX_SANITIZATION_STACK_SECRET_BYTES: usize = 1_024;

#[allow(clippy::manual_assert)]
pub(crate) const fn enforce_stack_secret_capacity<const N: usize>() {
    if N > MAX_SANITIZATION_STACK_SECRET_BYTES {
        panic!("sanitization secret staging exceeds the 1024-byte stack limit");
    }
}
