#![cfg(all(
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

use crate::decode_impl::validate_before_locked_fixed_allocation;
use base64_ng::{Standard, ct};
use core::cell::Cell;

#[cfg(all(
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
    ),
    not(miri)
))]
use crate::locked_vec::validate_before_locked_vec_allocation;

#[test]
fn malformed_input_does_not_reach_allocation_boundary() {
    let called = Cell::new(false);
    let result = validate_before_locked_fixed_allocation::<Standard, true, 5, (), _>(
        &ct::STANDARD,
        b"aGVsbG8!",
        || {
            called.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!called.get());
}

#[test]
fn wrong_length_does_not_reach_allocation_boundary() {
    let called = Cell::new(false);
    let result = validate_before_locked_fixed_allocation::<Standard, true, 4, (), _>(
        &ct::STANDARD,
        b"aGVsbG8=",
        || {
            called.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!called.get());
}

#[test]
fn oversized_input_does_not_reach_fixed_allocation_boundary() {
    let called = Cell::new(false);
    let result = validate_before_locked_fixed_allocation::<Standard, true, 5, (), _>(
        &ct::STANDARD,
        b"!!!!!!!!!!!!",
        || {
            called.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!called.get());
}

#[cfg(all(
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
    ),
    not(miri)
))]
#[test]
fn malformed_input_does_not_reach_dynamic_allocation_boundary() {
    let called = Cell::new(false);
    let result = validate_before_locked_vec_allocation::<Standard, true, (), _>(
        &ct::STANDARD,
        b"aGVsbG8!",
        |_| {
            called.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!called.get());
}

#[cfg(all(
    feature = "alloc",
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
    ),
    not(miri)
))]
#[test]
fn oversized_input_does_not_reach_dynamic_allocation_boundary() {
    let called = Cell::new(false);
    let maximum_encoded =
        base64_ng::checked_encoded_len(crate::DEFAULT_SECRET_VEC_DECODE_MAX_LEN, true).unwrap();
    let input = alloc::vec![b'!'; maximum_encoded + 1];
    let result = validate_before_locked_vec_allocation::<Standard, true, (), _>(
        &ct::STANDARD,
        &input,
        |_| {
            called.set(true);
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(!called.get());
}
