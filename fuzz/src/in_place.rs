use base64::Engine;
use base64_ng::{Base64, Codec};

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

fn check<S: Codec>(codec: &Base64<S>, oracle: &impl Engine, data: &[u8], offset: usize) {
    let encoded = oracle.encode(data);
    for input in [data, encoded.as_bytes()] {
        let expected = oracle.decode(input);
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
    }
}
