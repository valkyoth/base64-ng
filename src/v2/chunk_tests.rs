use super::{
    Base64, Codec, CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED,
    STRICT_STANDARD_UNPADDED, STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, ValidatedAlphabet,
};

const SENTINEL: u8 = 0xa5;

#[test]
fn encoded_chunks_match_one_shot_for_all_bounded_lengths_and_profiles() {
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
fn encoded_chunks_make_final_tail_and_borrowing_explicit() {
    assert!(
        STRICT_STANDARD_PADDED
            .encoded_chunks(b"")
            .unwrap()
            .next()
            .is_none()
    );

    let mut padded = STRICT_STANDARD_PADDED.encoded_chunks(b"hello").unwrap();
    assert_eq!(padded.len(), 2);
    assert_eq!(padded.next().unwrap().as_bytes(), b"aGVs");
    let padded_tail = padded.next().unwrap();
    assert_eq!(padded_tail.as_bytes(), b"bG8=");
    assert_eq!(padded_tail.as_str(), Ok("bG8="));
    assert!(padded.next().is_none());

    let mut unpadded = STRICT_STANDARD_UNPADDED.encoded_chunks(b"hello").unwrap();
    assert_eq!(unpadded.len(), 2);
    assert_eq!(unpadded.next().unwrap().as_bytes(), b"aGVs");
    assert_eq!(unpadded.next().unwrap().as_bytes(), b"bG8");
    assert!(unpadded.next().is_none());

    let input = *b"foo";
    let mut chunks = STRICT_STANDARD_PADDED.encoded_chunks(&input).unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks.next().unwrap().as_bytes(), b"Zm9v");
    assert_eq!(chunks.len(), 0);
    assert!(chunks.next().is_none());
    assert!(chunks.next().is_none());
}

fn exercise_profile<S: Codec>(codec: &Base64<S>) {
    let mut input = [0u8; 193];
    for len in 0..=input.len() {
        fill_pattern(&mut input[..len], len);
        let mut expected = [SENTINEL; 260];
        let expected_len = codec.encode_into(&input[..len], &mut expected).unwrap();

        let mut actual = [SENTINEL; 260];
        let mut actual_len = 0;
        let chunks = codec.encoded_chunks(&input[..len]).unwrap();
        assert_eq!(chunks.len(), len.div_ceil(3));
        for chunk in chunks {
            let end = actual_len + chunk.as_bytes().len();
            actual[actual_len..end].copy_from_slice(chunk.as_bytes());
            actual_len = end;
        }

        assert_eq!(actual_len, expected_len, "len={len}");
        assert_eq!(
            &actual[..actual_len],
            &expected[..expected_len],
            "len={len}"
        );
        assert!(actual[actual_len..].iter().all(|byte| *byte == SENTINEL));
    }
}

fn fill_pattern(bytes: &mut [u8], seed: usize) {
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .unwrap_or(0)
            .wrapping_mul(67)
            .wrapping_add(u8::try_from(seed).unwrap_or(0));
    }
}
