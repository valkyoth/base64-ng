use super::*;

struct DispatchFallbackAlphabet;

impl Alphabet for DispatchFallbackAlphabet {
    const ENCODE: [u8; 64] = *b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

fn fill_pattern(output: &mut [u8], seed: usize) {
    for (index, byte) in output.iter_mut().enumerate() {
        let value = (index * 73 + seed * 19) % 256;
        *byte = u8::try_from(value).unwrap();
    }
}

fn assert_encode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut dispatched = [0x55; 256];
    let mut scalar = [0xaa; 256];

    let dispatched_result = engine.encode_slice(input, &mut dispatched);
    let scalar_result = scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar);

    assert_eq!(dispatched_result, scalar_result);
    if let Ok(written) = dispatched_result {
        assert_eq!(&dispatched[..written], &scalar[..written]);
    }

    let required = checked_encoded_len(input.len(), PAD).unwrap();
    if required > 0 {
        let mut dispatched_short = [0x55; 256];
        let mut scalar_short = [0xaa; 256];
        let available = required - 1;

        assert_eq!(
            engine.encode_slice(input, &mut dispatched_short[..available]),
            scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut scalar_short[..available],)
        );
    }
}

fn assert_encode_in_place_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut dispatched = [0x55; 256];
    let mut scalar = [0xaa; 256];

    dispatched[..input.len()].copy_from_slice(input);
    scalar[..input.len()].copy_from_slice(input);

    let dispatched_result = engine
        .encode_in_place(&mut dispatched, input.len())
        .map(|encoded| encoded.len());
    let scalar_result = scalar_encode_in_place::encode_in_place::<A, PAD>(&mut scalar, input.len());

    assert_eq!(dispatched_result, scalar_result);
    if let Ok(written) = dispatched_result {
        assert_eq!(&dispatched[..written], &scalar[..written]);
    }
}

fn assert_decode_backend_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut dispatched = [0x55; 128];
    let mut scalar = [0xaa; 128];

    let dispatched_result = engine.decode_slice(input, &mut dispatched);
    let scalar_result = scalar::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar);

    assert_eq!(dispatched_result, scalar_result);
    if let Ok(written) = dispatched_result {
        assert_eq!(&dispatched[..written], &scalar[..written]);

        if written > 0 {
            let mut dispatched_short = [0x55; 128];
            let mut scalar_short = [0xaa; 128];
            let available = written - 1;

            assert_eq!(
                engine.decode_slice(input, &mut dispatched_short[..available]),
                scalar::scalar_reference_decode_slice::<A, PAD>(
                    input,
                    &mut scalar_short[..available],
                )
            );
        }
    }
}

fn assert_strict_decode_public_surfaces_match_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    let engine = Engine::<A, PAD>::new();
    let mut slice_output = [0x55; 128];
    let mut clear_tail_output = [0xaa; 128];
    let mut scalar_output = [0xcc; 128];

    let scalar_result = scalar::scalar_reference_decode_slice::<A, PAD>(input, &mut scalar_output);
    let slice_result = engine.decode_slice(input, &mut slice_output);
    let clear_tail_result = engine.decode_slice_clear_tail(input, &mut clear_tail_output);
    let buffer_result: Result<DecodedBuffer<128>, DecodeError> = engine.decode_buffer(input);

    assert_eq!(slice_result, scalar_result);
    assert_eq!(clear_tail_result, scalar_result);
    match slice_result {
        Ok(written) => {
            assert_eq!(&slice_output[..written], &scalar_output[..written]);
            assert_eq!(&clear_tail_output[..written], &scalar_output[..written]);
            assert!(clear_tail_output[written..].iter().all(|byte| *byte == 0));
            let buffer = buffer_result.unwrap();
            assert_eq!(buffer.as_bytes(), &scalar_output[..written]);
        }
        Err(err) => {
            assert!(clear_tail_output.iter().all(|byte| *byte == 0));
            assert_eq!(buffer_result.unwrap_err(), err);
        }
    }
}

fn assert_backend_round_trip_matches_scalar<A, const PAD: bool>(input: &[u8])
where
    A: Alphabet,
{
    assert_encode_backend_matches_scalar::<A, PAD>(input);

    let mut encoded = [0; 256];
    let encoded_len = scalar::scalar_reference_encode_slice::<A, PAD>(input, &mut encoded).unwrap();
    assert_decode_backend_matches_scalar::<A, PAD>(&encoded[..encoded_len]);
}

fn assert_standard_decode_chunk_matches_input(input: &[u8]) {
    let mut encoded = [0u8; 4];
    let encoded_len = STANDARD.encode_slice(input, &mut encoded).unwrap();
    assert_eq!(encoded_len, 4);

    let chunk = [encoded[0], encoded[1], encoded[2], encoded[3]];
    let mut decoded = [0u8; 3];
    let decoded_len = decode_chunk::<Standard, true>(chunk, &mut decoded).unwrap();

    assert_eq!(decoded_len, input.len());
    assert_eq!(&decoded[..decoded_len], input);
}

#[test]
fn backend_dispatch_matches_scalar_reference_for_canonical_inputs() {
    let mut input = [0; 128];

    for input_len in 0..=input.len() {
        fill_pattern(&mut input[..input_len], input_len);
        let input = &input[..input_len];

        assert_backend_round_trip_matches_scalar::<Standard, true>(input);
        assert_backend_round_trip_matches_scalar::<Standard, false>(input);
        assert_backend_round_trip_matches_scalar::<UrlSafe, true>(input);
        assert_backend_round_trip_matches_scalar::<UrlSafe, false>(input);
        assert_backend_round_trip_matches_scalar::<DispatchFallbackAlphabet, true>(input);
        assert_backend_round_trip_matches_scalar::<DispatchFallbackAlphabet, false>(input);
    }
}

#[test]
fn encode_in_place_backend_matches_scalar_reference() {
    let mut input = [0; 128];

    for input_len in 0..=input.len() {
        fill_pattern(&mut input[..input_len], input_len);
        let input = &input[..input_len];

        assert_encode_in_place_backend_matches_scalar::<Standard, true>(input);
        assert_encode_in_place_backend_matches_scalar::<Standard, false>(input);
        assert_encode_in_place_backend_matches_scalar::<UrlSafe, true>(input);
        assert_encode_in_place_backend_matches_scalar::<UrlSafe, false>(input);
    }
}

#[test]
fn backend_dispatch_matches_scalar_reference_for_malformed_inputs() {
    for input in [
        &b"Z"[..],
        b"====",
        b"AA=A",
        b"Zh==",
        b"Zm9=",
        b"Zm9v$g==",
        b"Zm9vZh==",
    ] {
        assert_decode_backend_matches_scalar::<Standard, true>(input);
    }

    for input in [&b"Z"[..], b"AA=A", b"Zh", b"Zm9", b"Zm9vYg$"] {
        assert_decode_backend_matches_scalar::<Standard, false>(input);
    }

    assert_decode_backend_matches_scalar::<UrlSafe, true>(b"AA+A");
    assert_decode_backend_matches_scalar::<UrlSafe, false>(b"AA/A");
    assert_decode_backend_matches_scalar::<Standard, true>(b"AA-A");
    assert_decode_backend_matches_scalar::<Standard, false>(b"AA_A");
}

#[test]
fn strict_decode_reports_invalid_byte_positions_exhaustively() {
    for index in 0..8 {
        let mut input = *b"Zm9vYmFy";
        input[index] = b'!';
        let mut output = [0; 6];
        assert_eq!(
            STANDARD.decode_slice(&input, &mut output),
            Err(DecodeError::InvalidByte { index, byte: b'!' })
        );
        assert_decode_backend_matches_scalar::<Standard, true>(&input);
    }

    for index in 0..8 {
        let mut input = *b"Zm9vYmFy";
        input[index] = b'/';
        let mut output = [0; 6];
        assert_eq!(
            URL_SAFE.decode_slice(&input, &mut output),
            Err(DecodeError::InvalidByte { index, byte: b'/' })
        );
        assert_decode_backend_matches_scalar::<UrlSafe, true>(&input);
    }
}

#[test]
fn strict_decode_rejects_padding_and_canonicality_matrix() {
    for input in [
        &b"=m9v"[..],
        b"Z=9v",
        b"Zm=v",
        b"Zm9v=AAA",
        b"Zh==",
        b"Zi==",
        b"Zm9=",
        b"Zm+=",
    ] {
        assert_decode_backend_matches_scalar::<Standard, true>(input);
        assert!(STANDARD.decode_buffer::<8>(input).is_err());
    }

    for input in [&b"Z"[..], b"Zh", b"Zi", b"Zm9", b"Zm+"] {
        assert_decode_backend_matches_scalar::<Standard, false>(input);
        assert!(STANDARD_NO_PAD.decode_buffer::<8>(input).is_err());
    }
}

#[test]
fn strict_decode_public_surfaces_match_scalar_reference() {
    for input in [
        &b""[..],
        b"Zg==",
        b"Zm8=",
        b"Zm9v",
        b"Zm9vYg==",
        b"Zm9vYmE=",
        b"Zm9vYmFy",
        b"////",
        b"AAEC",
    ] {
        assert_strict_decode_public_surfaces_match_scalar::<Standard, true>(input);
    }

    for input in [
        &b""[..],
        b"Zg",
        b"Zm8",
        b"Zm9v",
        b"Zm9vYg",
        b"Zm9vYmE",
        b"Zm9vYmFy",
        b"____",
        b"AAEC",
    ] {
        assert_strict_decode_public_surfaces_match_scalar::<UrlSafe, false>(input);
    }

    for input in [
        &b"Z"[..],
        b"====",
        b"AA=A",
        b"Zh==",
        b"Zm9=",
        b"Zm9v$g==",
        b"Zm9vZh==",
        b"AA-A",
    ] {
        assert_strict_decode_public_surfaces_match_scalar::<Standard, true>(input);
    }

    for input in [&b"Z"[..], b"AA=A", b"Zh", b"Zm9", b"Zm9vYg$", b"AA/A"] {
        assert_strict_decode_public_surfaces_match_scalar::<UrlSafe, false>(input);
    }
}

#[test]
fn decode_chunk_bit_packing_matches_exhaustive_small_inputs() {
    for byte in u8::MIN..=u8::MAX {
        assert_standard_decode_chunk_matches_input(&[byte]);
    }

    for first in u8::MIN..=u8::MAX {
        for second in u8::MIN..=u8::MAX {
            assert_standard_decode_chunk_matches_input(&[first, second]);
        }
    }
}

#[test]
fn decode_chunk_bit_packing_matches_representative_full_quanta() {
    const SAMPLES: [u8; 16] = [
        0, 1, 2, 15, 16, 31, 32, 63, 64, 95, 127, 128, 191, 192, 254, 255,
    ];

    for first in SAMPLES {
        for second in SAMPLES {
            for third in SAMPLES {
                assert_standard_decode_chunk_matches_input(&[first, second, third]);
            }
        }
    }
}

#[test]
fn ct_padded_final_quantum_fails_closed_for_invalid_padding_count() {
    let (_, invalid_byte, invalid_padding, written) =
        ct_padded_final_quantum::<Standard>(*b"ABCD", 3);

    assert_ne!(invalid_byte, 0);
    assert_ne!(invalid_padding, 0);
    assert_eq!(written, 0);
    assert_eq!(
        report_ct_error(invalid_byte, invalid_padding),
        Err(DecodeError::InvalidInput)
    );
}

#[cfg(feature = "simd")]
#[test]
fn simd_dispatch_uses_only_admitted_backends() {
    match simd::active_backend() {
        simd::ActiveBackend::Scalar => {}
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        simd::ActiveBackend::Avx512Vbmi => {
            assert!(matches!(
                simd::detected_candidate(),
                simd::Candidate::Avx512Vbmi
            ));
        }
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        simd::ActiveBackend::Avx2 => {
            assert!(matches!(
                simd::detected_candidate(),
                simd::Candidate::Avx2 | simd::Candidate::Avx512Vbmi
            ));
        }
        #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
        simd::ActiveBackend::Ssse3Sse41 => {
            assert!(matches!(
                simd::detected_candidate(),
                simd::Candidate::Ssse3Sse41 | simd::Candidate::Avx2 | simd::Candidate::Avx512Vbmi
            ));
        }
        #[cfg(all(feature = "std", target_arch = "aarch64"))]
        simd::ActiveBackend::Neon => {
            assert!(matches!(simd::detected_candidate(), simd::Candidate::Neon));
        }
    }
}

#[test]
fn encode_backend_boundary_uses_only_admitted_backends() {
    match encode_backend::active_encode_backend() {
        encode_backend::EncodeBackend::Scalar => {}
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        encode_backend::EncodeBackend::Avx512Vbmi => {
            assert!(simd::avx512_supports_alphabet::<Standard>());
            assert!(simd::avx512_supports_alphabet::<UrlSafe>());
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        encode_backend::EncodeBackend::Avx2 => {
            assert!(simd::avx2_supports_alphabet::<Standard>());
            assert!(simd::avx2_supports_alphabet::<UrlSafe>());
        }
        #[cfg(all(
            feature = "simd",
            feature = "std",
            any(target_arch = "x86", target_arch = "x86_64")
        ))]
        encode_backend::EncodeBackend::Ssse3Sse41 => {
            assert!(simd::ssse3_sse41_supports_alphabet::<Standard>());
            assert!(simd::ssse3_sse41_supports_alphabet::<UrlSafe>());
        }
        #[cfg(all(feature = "simd", feature = "std", target_arch = "aarch64"))]
        encode_backend::EncodeBackend::Neon => {
            assert!(simd::neon_supports_alphabet::<Standard>());
            assert!(simd::neon_supports_alphabet::<UrlSafe>());
        }
    }
}

#[test]
fn decode_backend_boundary_keeps_scalar_active() {
    assert_eq!(
        decode_backend::active_decode_backend(),
        decode_backend::DecodeBackend::Scalar
    );
}

#[test]
fn encodes_standard_vectors() {
    let vectors = [
        (&b""[..], &b""[..]),
        (&b"f"[..], &b"Zg=="[..]),
        (&b"fo"[..], &b"Zm8="[..]),
        (&b"foo"[..], &b"Zm9v"[..]),
        (&b"foob"[..], &b"Zm9vYg=="[..]),
        (&b"fooba"[..], &b"Zm9vYmE="[..]),
        (&b"foobar"[..], &b"Zm9vYmFy"[..]),
    ];
    for (input, expected) in vectors {
        let mut output = [0u8; 16];
        let written = STANDARD.encode_slice(input, &mut output).unwrap();
        assert_eq!(&output[..written], expected);
    }
}

#[test]
fn decodes_standard_vectors() {
    let vectors = [
        (&b""[..], &b""[..]),
        (&b"Zg=="[..], &b"f"[..]),
        (&b"Zm8="[..], &b"fo"[..]),
        (&b"Zm9v"[..], &b"foo"[..]),
        (&b"Zm9vYg=="[..], &b"foob"[..]),
        (&b"Zm9vYmE="[..], &b"fooba"[..]),
        (&b"Zm9vYmFy"[..], &b"foobar"[..]),
    ];
    for (input, expected) in vectors {
        let mut output = [0u8; 16];
        let written = STANDARD.decode_slice(input, &mut output).unwrap();
        assert_eq!(&output[..written], expected);
    }
}

#[test]
fn supports_unpadded_url_safe() {
    let mut encoded = [0u8; 16];
    let written = URL_SAFE_NO_PAD
        .encode_slice(b"\xfb\xff", &mut encoded)
        .unwrap();
    assert_eq!(&encoded[..written], b"-_8");

    let mut decoded = [0u8; 2];
    let written = URL_SAFE_NO_PAD
        .decode_slice(&encoded[..written], &mut decoded)
        .unwrap();
    assert_eq!(&decoded[..written], b"\xfb\xff");
}

#[test]
fn decodes_in_place() {
    let mut buffer = *b"Zm9vYmFy";
    let decoded = STANDARD_NO_PAD.decode_in_place(&mut buffer).unwrap();
    assert_eq!(decoded, b"foobar");
}

#[test]
fn rejects_non_canonical_padding_bits() {
    let mut output = [0u8; 4];
    assert_eq!(
        STANDARD.decode_slice(b"Zh==", &mut output),
        Err(DecodeError::InvalidPadding { index: 1 })
    );
    assert_eq!(
        STANDARD.decode_slice(b"Zm9=", &mut output),
        Err(DecodeError::InvalidPadding { index: 2 })
    );
}
