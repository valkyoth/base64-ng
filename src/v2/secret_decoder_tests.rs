#![cfg(feature = "secrets")]

extern crate std;

use std::{format, vec::Vec};

use super::{
    CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, TrailingBits, ValidatedAlphabet,
    compat::STANDARD_PADDED_PADDING_INDIFFERENT,
    secret::{SecretArrayFrame, SecretDecodeError, SecretFrame, SecretInput},
    secret_decoder::require_disjoint_ranges_for_test,
};

fn decode_array<const N: usize, S: super::Codec>(
    codec: &super::Base64<S>,
    chunks: &[&[u8]],
) -> Result<Vec<u8>, SecretDecodeError> {
    let mut frame = SecretArrayFrame::<N>::new(codec)?;
    for chunk in chunks {
        let progress = frame.update(&SecretInput::new(chunk))?;
        assert_eq!(progress.input_consumed(), chunk.len());
        assert_eq!(progress.output_produced(), 0);
    }
    Ok(frame.finish()?.declassify().as_bytes().to_vec())
}

#[test]
fn secret_frames_match_strict_scalar_profiles_and_chunkings() {
    let cases: &[&[u8]] = &[
        b"",
        b"f",
        b"fo",
        b"foo",
        b"foob",
        b"fooba",
        b"foobar",
        b"bounded secret frame input",
    ];
    for input in cases {
        let mut encoded = [0u8; 64];
        let written = STRICT_STANDARD_PADDED
            .encode_into(input, &mut encoded)
            .unwrap();
        let split = written.min(1);
        assert_eq!(
            decode_array::<64, _>(
                &STRICT_STANDARD_PADDED,
                &[&encoded[..split], &encoded[split..written]],
            )
            .unwrap(),
            *input
        );
        let written = STRICT_STANDARD_UNPADDED
            .encode_into(input, &mut encoded)
            .unwrap();
        assert_eq!(
            decode_array::<64, _>(&STRICT_STANDARD_UNPADDED, &[&encoded[..written]]).unwrap(),
            *input
        );
        let written = STRICT_URL_SAFE_PADDED
            .encode_into(input, &mut encoded)
            .unwrap();
        assert_eq!(
            decode_array::<64, _>(&STRICT_URL_SAFE_PADDED, &[&encoded[..written]]).unwrap(),
            *input
        );
        let written = STRICT_URL_SAFE_UNPADDED
            .encode_into(input, &mut encoded)
            .unwrap();
        assert_eq!(
            decode_array::<64, _>(&STRICT_URL_SAFE_UNPADDED, &[&encoded[..written]]).unwrap(),
            *input
        );
    }
}

#[test]
fn borrowed_frame_withholds_output_until_the_result_gate() {
    let mut staging = [0xa5; 16];
    let mut output = [0xa5; 16];
    let mut frame =
        SecretFrame::new(&STRICT_STANDARD_PADDED, 16, &mut staging, &mut output).unwrap();
    frame.update(&SecretInput::new(b"c2VjcmV0")).unwrap();
    assert_eq!(frame.state().input_len(), 8);
    let secret_output = frame.finish().unwrap();
    assert_eq!(secret_output.expose_secret().as_bytes(), b"secret");
    drop(secret_output);
    assert_eq!(staging, [0; 16]);
    assert_eq!(output, [0; 16]);
}

#[test]
fn borrowed_frame_rejects_unsupported_policy_before_mutating_storage() {
    let mut staging = [0xa5; 16];
    let mut output = [0x5a; 16];
    assert!(matches!(
        SecretFrame::new(
            &STANDARD_PADDED_PADDING_INDIFFERENT,
            16,
            &mut staging,
            &mut output,
        ),
        Err(SecretDecodeError::UnsupportedPolicy)
    ));
    assert_eq!(staging, [0xa5; 16]);
    assert_eq!(output, [0x5a; 16]);
}

#[test]
fn malformed_input_is_opaque_absorbing_and_wipes_borrowed_storage() {
    let mut staging = [0xa5; 16];
    let mut output = [0xa5; 16];
    let error = {
        let mut frame =
            SecretFrame::new(&STRICT_STANDARD_PADDED, 16, &mut staging, &mut output).unwrap();
        frame.update(&SecretInput::new(b"c2Vj!mV0")).unwrap();
        frame.finish().unwrap_err()
    };
    assert_eq!(error, SecretDecodeError::InvalidInput);
    assert_eq!(format!("{error}"), "invalid secret base64 input");
    assert_eq!(staging, [0; 16]);
    assert_eq!(output, [0; 16]);

    let mut frame = SecretArrayFrame::<3>::new(&STRICT_STANDARD_PADDED).unwrap();
    let oversized = frame.update(&SecretInput::new(b"QUJDRA==")).unwrap_err();
    assert_eq!(
        oversized,
        SecretDecodeError::InputTooLarge {
            input_len: 8,
            maximum_encoded_len: 4,
        }
    );
    assert!(frame.state().is_failed());
    assert_eq!(
        frame.update(&SecretInput::new(b"")),
        Err(SecretDecodeError::Failed)
    );
    let (private, public) = frame.storage_for_test();
    assert_eq!(private, &[0; 3]);
    assert_eq!(public, &[0; 3]);
}

#[test]
fn oversized_later_update_wipes_pending_secret_state_immediately() {
    let mut frame = SecretArrayFrame::<6>::new(&STRICT_STANDARD_PADDED).unwrap();
    frame.update(&SecretInput::new(b"QUJD")).unwrap();

    assert_eq!(
        frame.update(&SecretInput::new(b"RA===")),
        Err(SecretDecodeError::InputTooLarge {
            input_len: 9,
            maximum_encoded_len: 8,
        })
    );
    assert!(frame.state().is_failed());
    assert!(frame.state().pending_is_clear_for_test());
    let (staging, output) = frame.storage_for_test();
    assert_eq!(staging, &[0; 6]);
    assert_eq!(output, &[0; 6]);
}

#[test]
fn equal_public_lengths_receive_equal_symbol_scan_work() {
    for input in [b"QUJDRA==".as_slice(), b"QU!DRA==", b"====RA=="] {
        let mut frame = SecretArrayFrame::<6>::new(&STRICT_STANDARD_PADDED).unwrap();
        frame.update(&SecretInput::new(&input[..3])).unwrap();
        frame.update(&SecretInput::new(&input[3..])).unwrap();
        assert_eq!(frame.state().symbol_scans_for_test(), input.len() * 64);
        let _ = frame.finish();
    }

    let mut frame = SecretArrayFrame::<3>::new(&STRICT_STANDARD_PADDED).unwrap();
    assert!(matches!(
        frame.update(&SecretInput::new(b"QUJDRA==")),
        Err(SecretDecodeError::InputTooLarge { .. })
    ));
    assert_eq!(frame.state().symbol_scans_for_test(), 0);
}

#[test]
fn padded_capacity_is_checked_opaquely_at_finish() {
    assert_eq!(
        decode_array::<1, _>(&STRICT_STANDARD_PADDED, &[b"Zg=="]).unwrap(),
        b"f"
    );
    assert_eq!(
        decode_array::<1, _>(&STRICT_STANDARD_PADDED, &[b"Zm9v"]),
        Err(SecretDecodeError::InvalidInput)
    );
}

#[test]
fn malformed_classes_and_positions_never_release_plaintext() {
    let canonical = b"c2VjcmV0";
    for index in 0..canonical.len() {
        for replacement in [b'!', b'=', 0xff] {
            let mut malformed = *canonical;
            malformed[index] = replacement;
            assert_eq!(
                decode_array::<16, _>(&STRICT_STANDARD_PADDED, &[&malformed]),
                Err(SecretDecodeError::InvalidInput)
            );
        }
    }
    for malformed in [b"A===".as_slice(), b"AB=C", b"AB==AAAA", b"AB=!"] {
        assert_eq!(
            decode_array::<16, _>(&STRICT_STANDARD_PADDED, &[malformed]),
            Err(SecretDecodeError::InvalidInput)
        );
    }
    for malformed in [b"A".as_slice(), b"AB=", b"ABCD="] {
        assert_eq!(
            decode_array::<16, _>(&STRICT_STANDARD_UNPADDED, &[malformed]),
            Err(SecretDecodeError::InvalidInput)
        );
    }
}

#[test]
fn successful_array_tail_stays_zero_and_declassifies_only_prefix() {
    let mut frame = SecretArrayFrame::<16>::new(&STRICT_STANDARD_PADDED).unwrap();
    assert_eq!(frame.state().maximum_decoded_len(), 16);
    assert_eq!(frame.state().maximum_encoded_len(), 24);
    frame.update(&SecretInput::new(b"a2V5")).unwrap();
    let secret = frame.finish().unwrap();
    assert_eq!(secret.expose_secret().as_bytes(), b"key");
    assert!(secret.backing_for_test()[3..].iter().all(|byte| *byte == 0));
    let ordinary = secret.declassify();
    assert_eq!(ordinary.as_bytes(), b"key");
}

#[test]
fn runtime_validated_alphabet_uses_the_same_fixed_scan_boundary() {
    let alphabet = ValidatedAlphabet::new(
        *b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    )
    .unwrap();
    let codec = CodecBuilder::new(alphabet)
        .encode_padding(EncodePadding::Unpadded)
        .decode_padding(DecodePadding::Forbid)
        .trailing_bits(TrailingBits::RequireCanonical)
        .build()
        .unwrap();
    let mut encoded = [0u8; 32];
    let written = codec.encode_into(b"custom", &mut encoded).unwrap();
    assert_eq!(
        decode_array::<16, _>(&codec, &[&encoded[..written]]).unwrap(),
        b"custom"
    );
}

#[test]
fn range_arithmetic_rejects_overlap_and_overflow() {
    assert_eq!(
        require_disjoint_ranges_for_test(100, 8, 107, 4),
        Err(SecretDecodeError::OverlappingBuffers)
    );
    assert_eq!(require_disjoint_ranges_for_test(100, 8, 108, 4), Ok(()));
    assert_eq!(
        require_disjoint_ranges_for_test(usize::MAX, 2, 0, 0),
        Err(SecretDecodeError::AddressRangeOverflow)
    );
}

#[cfg(feature = "std")]
#[test]
fn borrowed_frame_wipes_on_unwind() {
    let mut staging = [0xa5; 16];
    let mut output = [0xa5; 16];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut frame =
            SecretFrame::new(&STRICT_STANDARD_PADDED, 16, &mut staging, &mut output).unwrap();
        frame.update(&SecretInput::new(b"c2VjcmV0")).unwrap();
        panic!("reviewed secret frame cleanup test");
    }));
    assert!(result.is_err());
    assert_eq!(staging, [0; 16]);
    assert_eq!(output, [0; 16]);
}

#[cfg(feature = "alloc")]
#[test]
fn vector_frame_preallocates_and_preserves_zero_spare_capacity() {
    use super::secret::SecretVecFrame;

    let mut frame = SecretVecFrame::new(&STRICT_STANDARD_PADDED, 32).unwrap();
    frame.update(&SecretInput::new(b"c2VjcmV0")).unwrap();
    let secret = frame.finish().unwrap();
    assert_eq!(secret.expose_secret().as_bytes(), b"secret");
    assert!(secret.capacity() >= 32);
    let ordinary = secret.declassify_into_unprotected_vec();
    assert_eq!(ordinary, std::vec![b's', b'e', b'c', b'r', b'e', b't']);
}
