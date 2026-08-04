extern crate std;

use alloc::{format, string::String};
use core::cell::Cell;

use super::{
    ordinary::OneShotError,
    ordinary_string::Base64String,
    specifications::{
        CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
        STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
    },
};

#[test]
fn encoded_value_retains_policy_and_round_trips() {
    let encoded = Base64String::encode(STRICT_STANDARD_PADDED, b"hello").unwrap();
    assert_eq!(encoded.as_str(), "aGVsbG8=");
    assert_eq!(encoded.as_bytes(), b"aGVsbG8=");
    assert_eq!(encoded.len(), 8);
    assert!(!encoded.is_empty());
    assert_eq!(encoded.decode().unwrap(), b"hello");
    assert_eq!(encoded.settings(), STRICT_STANDARD_PADDED.settings());
    assert_eq!(encoded.codec(), &STRICT_STANDARD_PADDED);

    let empty = Base64String::encode(STRICT_STANDARD_PADDED, b"").unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.decode().unwrap(), b"");

    assert_eq!(
        Base64String::encode(STRICT_STANDARD_UNPADDED, b"f")
            .unwrap()
            .as_str(),
        "Zg"
    );
    assert_eq!(
        Base64String::encode(STRICT_URL_SAFE_PADDED, b"\xfb\xff")
            .unwrap()
            .as_str(),
        "-_8="
    );
}

#[test]
fn parsing_validates_before_returning_immutable_ownership() {
    let parsed = Base64String::parse(STRICT_STANDARD_PADDED, "Zm9v").unwrap();
    assert_eq!(parsed.decode().unwrap(), b"foo");

    let owned = String::from("Zm8=");
    let owned_pointer = owned.as_ptr();
    let adopted = Base64String::from_string(STRICT_STANDARD_PADDED, owned).unwrap();
    assert_eq!(adopted.as_bytes().as_ptr(), owned_pointer);
    assert_eq!(adopted.decode().unwrap(), b"fo");

    for malformed in ["!!!!", "Zg=", "AB==", "Zg==A", "é"] {
        assert!(matches!(
            Base64String::parse(STRICT_STANDARD_PADDED, malformed),
            Err(OneShotError::Input(_))
        ));
    }
}

#[test]
fn parsing_validates_before_fallible_reservation() {
    let reserve_called = Cell::new(false);
    let error =
        Base64String::parse_with_injected_reserver(STRICT_STANDARD_PADDED, "!!!!", |_, _| {
            reserve_called.set(true);
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(error, OneShotError::Input(_)));
    assert!(!reserve_called.get());

    let error = Base64String::parse_with_injected_reserver(
        STRICT_STANDARD_PADDED,
        "Zm9v",
        |output, required| {
            assert!(output.is_empty());
            assert_eq!(required, 4);
            Err(OneShotError::AllocationFailed {
                requested: required,
            })
        },
    )
    .unwrap_err();
    assert_eq!(error, OneShotError::AllocationFailed { requested: 4 });
}

#[test]
fn policy_is_not_inferred_from_the_text() {
    let url = Base64String::encode(STRICT_URL_SAFE_UNPADDED, b"\xfb\xff").unwrap();
    assert_eq!(url.as_str(), "-_8");
    assert_eq!(url.decode().unwrap(), b"\xfb\xff");
    assert!(Base64String::parse(STRICT_STANDARD_PADDED, url.as_str()).is_err());

    let custom = CodecBuilder::from_table(
        *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
    )
    .unwrap()
    .encode_padding(EncodePadding::Unpadded)
    .decode_padding(DecodePadding::Forbid)
    .build()
    .unwrap();
    let custom_text = Base64String::encode(custom, b"custom").unwrap();
    assert_eq!(custom_text.as_str(), "W1TxbE7r");
    assert_eq!(custom_text.decode().unwrap(), b"custom");

    let relaxed = CodecBuilder::from_table(
        *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    )
    .unwrap()
    .decode_padding(DecodePadding::Indifferent)
    .build()
    .unwrap();
    let relaxed_text = Base64String::parse(relaxed, "Zg").unwrap();
    assert_eq!(relaxed_text.decode().unwrap(), b"f");
    assert!(Base64String::parse(STRICT_STANDARD_PADDED, "Zg").is_err());
}

#[test]
fn ordinary_visibility_and_output_limits_are_explicit() {
    let encoded = Base64String::encode(STRICT_STANDARD_PADDED, b"hello").unwrap();
    assert_eq!(format!("{encoded}"), "aGVsbG8=");
    assert!(format!("{encoded:?}").contains("aGVsbG8="));
    assert_eq!(AsRef::<str>::as_ref(&encoded), "aGVsbG8=");
    assert_eq!(AsRef::<[u8]>::as_ref(&encoded), b"aGVsbG8=");
    assert_eq!(
        encoded.decode_with_limit(4),
        Err(OneShotError::AllocationLimitExceeded {
            required: 5,
            limit: 4,
        })
    );
    assert_eq!(encoded.clone().into_string(), "aGVsbG8=");
}

#[test]
fn focused_prelude_exposes_only_the_ordinary_starting_surface() {
    use crate::prelude::*;

    let encoded = Base64String::encode(STRICT_STANDARD_PADDED, b"hello").unwrap();
    let codec: Base64<_> = STRICT_STANDARD_PADDED;
    assert_eq!(encoded.decode().unwrap(), b"hello");
    assert_eq!(codec.settings(), encoded.settings());
}
