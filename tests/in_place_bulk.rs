#[path = "../src/v2/rfc4648_oracle.rs"]
mod oracle;

use base64_ng::{CodecBuilder, DecodeValidation, InPlaceError, OneShotError};
use oracle::Profile;

fn profiles() -> [(Profile, base64_ng::CodecSettings); 4] {
    [
        (
            Profile::StandardPadded,
            base64_ng::STRICT_STANDARD_PADDED.settings(),
        ),
        (
            Profile::StandardUnpadded,
            base64_ng::STRICT_STANDARD_UNPADDED.settings(),
        ),
        (
            Profile::UrlSafePadded,
            base64_ng::STRICT_URL_SAFE_PADDED.settings(),
        ),
        (
            Profile::UrlSafeUnpadded,
            base64_ng::STRICT_URL_SAFE_UNPADDED.settings(),
        ),
    ]
}

fn historical(profile: Profile, buffer: &mut [u8]) -> Result<usize, base64_ng::DecodeError> {
    match profile {
        Profile::StandardPadded => base64_ng::STANDARD.decode_in_place(buffer),
        Profile::StandardUnpadded => base64_ng::STANDARD_NO_PAD.decode_in_place(buffer),
        Profile::UrlSafePadded => base64_ng::URL_SAFE.decode_in_place(buffer),
        Profile::UrlSafeUnpadded => base64_ng::URL_SAFE_NO_PAD.decode_in_place(buffer),
    }
    .map(|output| output.len())
}

#[test]
fn in_place_bulk_profiles_lengths_alignment_and_residuals_match_independent_oracle() {
    let _ = base64_ng::initialize_backends();
    for (profile, settings) in profiles() {
        let codec = CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        for len in (0..=256)
            .chain(375..=393)
            .chain(759..=777)
            .chain(1527..=1545)
            .chain(3063..=3081)
            .chain([6145, 12289])
        {
            let plain: Vec<_> = (0..len).map(|i| (i as u8).wrapping_mul(73)).collect();
            let encoded = oracle::encode(profile, &plain);
            assert_eq!(oracle::decode(profile, &encoded).unwrap(), plain);
            for offset in 0..32 {
                let mut storage = vec![0xa5; offset + encoded.len() + 32];
                storage[offset..offset + encoded.len()].copy_from_slice(&encoded);
                let mut expected = storage.clone();
                expected[offset..offset + len].copy_from_slice(&plain);
                let mut legacy = storage.clone();
                assert_eq!(
                    codec.decode_in_place(&mut storage[offset..], encoded.len()),
                    Ok(len)
                );
                assert_eq!(storage, expected);
                assert_eq!(
                    historical(profile, &mut legacy[offset..offset + encoded.len()]),
                    Ok(len)
                );
                assert_eq!(legacy, expected);
            }
        }
    }
}

#[test]
fn in_place_bulk_all_byte_mutations_preserve_errors_and_entire_buffer() {
    let _ = base64_ng::initialize_backends();
    for (profile, settings) in profiles() {
        let codec = CodecBuilder::new(*settings.alphabet())
            .encode_padding(settings.encode_padding())
            .decode_padding(settings.decode_padding())
            .build()
            .unwrap();
        let encoded = oracle::encode(profile, &[0x9b; 3077]);
        for position in [
            0,
            15,
            31,
            511,
            1023,
            1024,
            2047,
            encoded.len() - 3,
            encoded.len() - 2,
            encoded.len() - 1,
        ] {
            for byte in 0..=255 {
                let mut source = encoded.clone();
                source[position] = byte;
                let mut buffer = source.clone();
                buffer.extend_from_slice(&[0xa5; 19]);
                let before = buffer.clone();
                let mut reference = vec![0; source.len()];
                let result = codec.decode_into_with_validation(
                    &source,
                    &mut reference,
                    DecodeValidation::ScalarReference,
                );
                assert_eq!(result.is_ok(), oracle::decode(profile, &source).is_ok());
                let expected = result.map_err(|error| match error {
                    OneShotError::Input(error) => InPlaceError::Input(error),
                    other => panic!("unexpected reference error: {other:?}"),
                });
                assert_eq!(codec.decode_in_place(&mut buffer, source.len()), expected);
                if let Ok(n) = result {
                    assert_eq!(&buffer[..n], &reference[..n]);
                    assert_eq!(&buffer[n..], &before[n..]);
                } else {
                    assert_eq!(buffer, before);
                }
            }
        }
    }
}

#[test]
fn in_place_bulk_bad_prefix_and_custom_relaxed_fallback_are_transactional() {
    let mut input = [b'A'; 8];
    assert_eq!(
        base64_ng::STRICT_STANDARD_PADDED.decode_in_place(&mut input, 9),
        Err(InPlaceError::InputLengthExceedsBuffer {
            input_len: 9,
            buffer_len: 8
        })
    );
    assert_eq!(input, [b'A'; 8]);
    for alphabet in [
        *base64_ng::STRICT_STANDARD_PADDED.settings().alphabet(),
        base64_ng::ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    ] {
        let codec = CodecBuilder::new(alphabet)
            .decode_padding(base64_ng::DecodePadding::Indifferent)
            .trailing_bits(base64_ng::TrailingBits::AllowNonCanonical)
            .build()
            .unwrap();
        let mut buffer = [b'A'; 4100];
        let mut reference = [0; 3075];
        let n = codec.decode_into(&buffer, &mut reference).unwrap();
        assert_eq!(codec.decode_in_place(&mut buffer, 4100), Ok(n));
        assert_eq!(&buffer[..n], &reference[..n]);
    }
}
