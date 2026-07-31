#![cfg(feature = "alloc")]

use core::cell::Cell;

use super::{
    Base64, Codec, CodecBuilder, DecodePadding, EncodePadding, OneShotError,
    STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED,
    STRICT_URL_SAFE_UNPADDED, ValidatedAlphabet,
};

#[test]
fn append_round_trips_all_profiles_and_bounded_lengths() {
    exercise_profile(&STRICT_STANDARD_PADDED);
    exercise_profile(&STRICT_STANDARD_UNPADDED);
    exercise_profile(&STRICT_URL_SAFE_PADDED);
    exercise_profile(&STRICT_URL_SAFE_UNPADDED);

    let custom = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    )
    .encode_padding(EncodePadding::Unpadded)
    .decode_padding(DecodePadding::Forbid)
    .build()
    .unwrap();
    exercise_profile(&custom);
}

#[test]
fn append_success_preserves_existing_prefixes() {
    let mut encoded = std::string::String::from("prefix:");
    assert_eq!(
        STRICT_STANDARD_PADDED
            .encode_append(b"hello", &mut encoded)
            .unwrap(),
        8
    );
    assert_eq!(encoded, "prefix:aGVsbG8=");

    let mut decoded = std::vec::Vec::from(&b"prefix:"[..]);
    assert_eq!(
        STRICT_STANDARD_PADDED
            .decode_append(b"aGVsbG8=", &mut decoded)
            .unwrap(),
        5
    );
    assert_eq!(decoded, b"prefix:hello");
}

#[test]
fn reserve_and_crate_errors_restore_entry_length_and_prefix() {
    let mut encoded = std::string::String::from("prefix");
    let original = encoded.clone();
    let reserve_called = Cell::new(false);
    let error = STRICT_STANDARD_PADDED
        .encode_append_with_hooks(
            b"foobar",
            &mut encoded,
            |_, required| {
                reserve_called.set(true);
                Err(OneShotError::AllocationFailed {
                    requested: required,
                })
            },
            |_, _| Ok(()),
        )
        .unwrap_err();
    assert!(reserve_called.get());
    assert_eq!(error, OneShotError::AllocationFailed { requested: 8 });
    assert_eq!(encoded, original);

    let mut partial = std::string::String::from("prefix");
    let original = partial.clone();
    let calls = Cell::new(0usize);
    let error = STRICT_STANDARD_PADDED
        .encode_append_with_hooks(
            b"foobar",
            &mut partial,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_, _| {
                let call = calls.get();
                calls.set(call + 1);
                if call == 0 {
                    Err(OneShotError::Backend(super::BackendFault::ImpossibleState))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();
    assert!(matches!(error, OneShotError::Backend(_)));
    assert_eq!(partial, original);

    let mut decoded = std::vec::Vec::from(&b"prefix"[..]);
    let original = decoded.clone();
    let reserve_called = Cell::new(false);
    let error = STRICT_STANDARD_PADDED
        .decode_append_with_hooks(
            b"!!!!",
            &mut decoded,
            |_, _| {
                reserve_called.set(true);
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap_err();
    assert!(matches!(error, OneShotError::Input(_)));
    assert!(!reserve_called.get());
    assert_eq!(decoded, original);

    let error = STRICT_STANDARD_PADDED
        .decode_append_with_hooks(
            b"Zm9v",
            &mut decoded,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_| Err(OneShotError::Backend(super::BackendFault::ImpossibleState)),
        )
        .unwrap_err();
    assert!(matches!(error, OneShotError::Backend(_)));
    assert_eq!(decoded, original);
}

#[test]
fn unwind_rollback_restores_string_and_vec_lengths() {
    let mut encoded = std::string::String::from("prefix");
    let original = encoded.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = STRICT_STANDARD_PADDED.encode_append_with_hooks(
            b"foobar",
            &mut encoded,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_, _| panic!("injected append panic"),
        );
    }));
    assert!(result.is_err());
    assert_eq!(encoded, original);

    let mut decoded = std::vec::Vec::from(&b"prefix"[..]);
    let original = decoded.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = STRICT_STANDARD_PADDED.decode_append_with_hooks(
            b"Zm9vYmFy",
            &mut decoded,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_| panic!("injected decode panic"),
        );
    }));
    assert!(result.is_err());
    assert_eq!(decoded, original);
}

fn exercise_profile<S: Codec>(codec: &Base64<S>) {
    let mut input = [0u8; 96];
    for len in 0..=input.len() {
        fill_pattern(&mut input[..len], len);
        let mut expected = [0u8; 128];
        let expected_len = codec.encode_into(&input[..len], &mut expected).unwrap();

        let mut encoded = std::string::String::from("prefix:");
        assert_eq!(
            codec.encode_append(&input[..len], &mut encoded).unwrap(),
            expected_len
        );
        assert_eq!(encoded.as_bytes().get(7..), Some(&expected[..expected_len]));

        let mut decoded = std::vec::Vec::from(&b"prefix:"[..]);
        assert_eq!(
            codec
                .decode_append(&expected[..expected_len], &mut decoded)
                .unwrap(),
            len
        );
        assert_eq!(decoded.get(7..), Some(&input[..len]));
    }
}

fn fill_pattern(bytes: &mut [u8], seed: usize) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap_or(0)
            .wrapping_mul(89)
            .wrapping_add(u8::try_from(seed).unwrap_or(0));
    }
}
