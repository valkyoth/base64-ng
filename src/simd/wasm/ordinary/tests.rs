use super::*;

#[test]
fn closed_boundary_checks_availability_and_geometry() {
    let backend = Backend::WasmSimd128;
    assert_eq!(width(Backend::Scalar), None);
    assert!(!validate(Backend::Scalar, &[b'A'; 16], false));
    assert!(!decode(Backend::Scalar, &[b'A'; 16], &mut [0; 12], false));
    if !super::super::wasm_simd128_decode_available() {
        assert_eq!(width(backend), None);
        assert!(!validate(backend, &[b'A'; 16], false));
        assert!(!decode(backend, &[b'A'; 16], &mut [0; 12], false));
        return;
    }
    assert_eq!(width(backend), Some(16));
    for len in [1, 15, 17, 31] {
        assert!(!validate(backend, &[b'A'; 32][..len], false));
    }
    for len in [0, 11, 13] {
        let mut output = [0xa5; 13];
        assert!(!decode(backend, &[b'A'; 16], &mut output[..len], false));
        assert_eq!(output, [0xa5; 13]);
    }
}

#[test]
fn every_byte_lane_and_unaligned_block_matches_literal_alphabet() {
    let backend = Backend::WasmSimd128;
    if width(backend).is_none() {
        return;
    }
    for url in [false, true] {
        let alphabet = if url {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        } else {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        };
        for offset in 0..16 {
            for lane in 0..16 {
                for byte in 0..=255 {
                    let mut input = [b'A'; 32];
                    input[offset + lane] = byte;
                    let input = &input[offset..offset + 16];
                    let expected = alphabet.contains(&byte);
                    assert_eq!(validate(backend, input, url), expected);
                    let mut output = [0xa5; 28];
                    assert_eq!(
                        decode(backend, input, &mut output[offset..offset + 12], url),
                        expected
                    );
                    if expected {
                        let mut reference = [0; 12];
                        for (quantum, decoded) in input
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(reference.as_chunks_mut::<3>().0)
                        {
                            let v: [u8; 4] = core::array::from_fn(|i| {
                                u8::try_from(
                                    alphabet.iter().position(|b| *b == quantum[i]).unwrap(),
                                )
                                .unwrap()
                            });
                            decoded.copy_from_slice(&[
                                (v[0] << 2) | (v[1] >> 4),
                                (v[1] << 4) | (v[2] >> 2),
                                (v[2] << 6) | v[3],
                            ]);
                        }
                        assert_eq!(output[offset..offset + 12], reference);
                        assert!(output[..offset].iter().all(|b| *b == 0xa5));
                        assert!(output[offset + 12..].iter().all(|b| *b == 0xa5));
                    } else {
                        assert_eq!(output, [0xa5; 28]);
                    }
                }
            }
        }
    }
}
