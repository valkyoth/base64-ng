use base64::Engine;
use base64_ng::{Base64, Base64Ref, Codec, DecodeValidation, OneShotError};

pub fn exercise(data: &[u8]) {
    let input = &data[..data.len().min(8192)];
    let offset = data.first().copied().unwrap_or(0) as usize % 32;
    check(
        &base64_ng::STRICT_STANDARD_PADDED,
        &base64::engine::general_purpose::STANDARD,
        input,
        offset,
    );
    check(
        &base64_ng::STRICT_STANDARD_UNPADDED,
        &base64::engine::general_purpose::STANDARD_NO_PAD,
        input,
        offset,
    );
    check(
        &base64_ng::STRICT_URL_SAFE_PADDED,
        &base64::engine::general_purpose::URL_SAFE,
        input,
        offset,
    );
    check(
        &base64_ng::STRICT_URL_SAFE_UNPADDED,
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        input,
        offset,
    );
}

fn check<S: Codec + Clone>(codec: &Base64<S>, oracle: &impl Engine, data: &[u8], offset: usize) {
    let encoded = oracle.encode(data);
    let mut mutated = encoded.as_bytes().to_vec();
    if !mutated.is_empty() {
        let index = data.len().wrapping_mul(37) % mutated.len();
        mutated[index] = data.first().copied().unwrap_or(b'=');
    }
    for input in [data, encoded.as_bytes(), &mutated] {
        let expected = oracle.decode(input);
        // The oracle is another implementation, not a second optimized route.
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            let view = Base64Ref::parse_with_validation(codec.clone(), input, policy);
            match (&expected, view) {
                (Ok(decoded), Ok(view)) => {
                    assert_eq!(view.decoded_len(), decoded.len());
                    for capacity in [
                        decoded.len().saturating_sub(1),
                        decoded.len(),
                        decoded.len() + 3,
                    ] {
                        let mut output = vec![0xa5; offset + capacity + 3];
                        for _ in 0..2 {
                            output.fill(0xa5);
                            let result = view.decode_into(&mut output[offset..offset + capacity]);
                            if capacity < decoded.len() {
                                assert_eq!(
                                    result,
                                    Err(OneShotError::OutputTooSmall {
                                        required: decoded.len(),
                                        available: capacity,
                                    })
                                );
                                assert!(output.iter().all(|&b| b == 0xa5));
                            } else {
                                assert_eq!(result, Ok(decoded.len()));
                                assert_eq!(&output[offset..offset + decoded.len()], decoded);
                                assert!(output[..offset].iter().all(|&b| b == 0xa5));
                                assert!(
                                    output[offset + decoded.len()..].iter().all(|&b| b == 0xa5)
                                );
                            }
                        }
                    }
                }
                (Err(_), Err(_)) => {}
                _ => panic!("borrowed-view acceptance differs from independent decoder"),
            }
        }
        let mut storage = vec![0xa5; offset + input.len() + 32];
        storage[offset..offset + input.len()].copy_from_slice(input);
        let before = storage.clone();
        let actual = codec.decode_in_place(&mut storage[offset..], input.len());
        match (expected, actual) {
            (Ok(expected), Ok(n)) => {
                assert_eq!(n, expected.len());
                assert_eq!(&storage[offset..offset + n], expected);
                assert_eq!(&storage[..offset], &before[..offset]);
                assert_eq!(&storage[offset + n..], &before[offset + n..]);
            }
            (Err(_), Err(_)) => assert_eq!(storage, before),
            _ => panic!("in-place acceptance differs from independent decoder"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn in_place_bulk_fuzz_regression_crosses_scratch_and_vector_boundaries() {
        for len in [
            0, 1, 2, 3, 383, 384, 385, 767, 768, 769, 3071, 3072, 3073, 8192,
        ] {
            let input: Vec<_> = (0..len).map(|i| (i as u8).wrapping_mul(37)).collect();
            super::exercise(&input);
        }
        super::exercise(&[b'A'; 8192]);
        let mut late_invalid = [b'A'; 8192];
        late_invalid[8191] = b'!';
        super::exercise(&late_invalid);
        for malformed in [
            b"Zh==".as_slice(),
            b"Zm9=",
            b"Zh",
            b"Zm9",
            b"=AAA",
            b"AA\x80A",
        ] {
            super::exercise(malformed);
        }
    }
}
