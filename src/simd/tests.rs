use super::*;
#[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
use crate::{Alphabet, decode_alphabet_byte};
use crate::{Engine, Standard, UrlSafe};

#[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
struct AnchorMatchingCustom;

#[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
impl Alphabet for AnchorMatchingCustom {
    const ENCODE: [u8; 64] = *b"ACBDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

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

#[cfg(any(target_arch = "aarch64", target_arch = "x86", target_arch = "x86_64"))]
fn fill_indices_pattern(output: &mut [u8; 12], seed: u8) {
    let mut write = 0;
    for group in 0..4 {
        let i0 = seed.wrapping_add(group * 4) & 0x3f;
        let i1 = seed.wrapping_add(group * 4 + 1) & 0x3f;
        let i2 = seed.wrapping_add(group * 4 + 2) & 0x3f;
        let i3 = seed.wrapping_add(group * 4 + 3) & 0x3f;

        output[write] = (i0 << 2) | (i1 >> 4);
        output[write + 1] = (i1 << 4) | (i2 >> 2);
        output[write + 2] = (i2 << 6) | i3;
        write += 3;
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn fill_indices_pattern_wide(output: &mut [u8; 24], seed: u8) {
    let mut write = 0;
    for group in 0..8 {
        let i0 = seed.wrapping_add(group * 4) & 0x3f;
        let i1 = seed.wrapping_add(group * 4 + 1) & 0x3f;
        let i2 = seed.wrapping_add(group * 4 + 2) & 0x3f;
        let i3 = seed.wrapping_add(group * 4 + 3) & 0x3f;

        output[write] = (i0 << 2) | (i1 >> 4);
        output[write + 1] = (i1 << 4) | (i2 >> 2);
        output[write + 2] = (i2 << 6) | i3;
        write += 3;
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn fill_indices_pattern_zmm(output: &mut [u8; 48], seed: u8) {
    let mut write = 0;
    for group in 0..16 {
        let i0 = seed.wrapping_add(group * 4) & 0x3f;
        let i1 = seed.wrapping_add(group * 4 + 1) & 0x3f;
        let i2 = seed.wrapping_add(group * 4 + 2) & 0x3f;
        let i3 = seed.wrapping_add(group * 4 + 3) & 0x3f;

        output[write] = (i0 << 2) | (i1 >> 4);
        output[write + 1] = (i1 << 4) | (i2 >> 2);
        output[write + 2] = (i2 << 6) | i3;
        write += 3;
    }
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
#[test]
fn avx512_encode_block_matches_scalar_when_available() {
    if !avx512_vbmi_base64_available() {
        println!(
            "skipped: AVX-512 VBMI encode block test requires avx512f,avx512bw,avx512vl,avx512vbmi"
        );
        return;
    }

    let mut input = [0; 48];
    for seed in 0..64 {
        fill_pattern(&mut input, seed);

        let mut avx512_standard = [0x55; 64];
        let mut scalar_standard = [0xaa; 64];
        // SAFETY: The candidate check above uses runtime AVX-512 feature
        // bundle detection on std builds and compile-time target-feature
        // detection otherwise.
        unsafe {
            encode_48_bytes_avx512::<Standard>(&input, &mut avx512_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, avx512_standard.len());
        assert_eq!(avx512_standard, scalar_standard);

        let mut avx512_url_safe = [0x55; 64];
        let mut scalar_url_safe = [0xaa; 64];
        // SAFETY: The candidate check above proves the AVX-512 feature
        // bundle is available for this test invocation.
        unsafe {
            encode_48_bytes_avx512::<UrlSafe>(&input, &mut avx512_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, avx512_url_safe.len());
        assert_eq!(avx512_url_safe, scalar_url_safe);
    }

    for seed in 0..64 {
        fill_indices_pattern_zmm(&mut input, seed);

        let mut avx512_standard = [0x55; 64];
        let mut scalar_standard = [0xaa; 64];
        // SAFETY: The candidate check above proves the AVX-512 feature
        // bundle is available for this test invocation.
        unsafe {
            encode_48_bytes_avx512::<Standard>(&input, &mut avx512_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, avx512_standard.len());
        assert_eq!(avx512_standard, scalar_standard);

        let mut avx512_url_safe = [0x55; 64];
        let mut scalar_url_safe = [0xaa; 64];
        // SAFETY: The candidate check above proves the AVX-512 feature
        // bundle is available for this test invocation.
        unsafe {
            encode_48_bytes_avx512::<UrlSafe>(&input, &mut avx512_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, avx512_url_safe.len());
        assert_eq!(avx512_url_safe, scalar_url_safe);
    }

    fill_indices_pattern_zmm(&mut input, 0);
    let mut avx512_custom = [0x55; 64];
    let mut scalar_custom = [0xaa; 64];
    // SAFETY: The candidate check above proves the AVX-512 feature bundle is
    // available for this test invocation.
    unsafe {
        encode_48_bytes_avx512::<AnchorMatchingCustom>(&input, &mut avx512_custom);
    }
    let scalar_len = Engine::<AnchorMatchingCustom, true>::new()
        .encode_slice(&input, &mut scalar_custom)
        .unwrap();
    assert_eq!(scalar_len, avx512_custom.len());
    assert_eq!(avx512_custom, scalar_custom);
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
#[test]
fn avx2_encode_block_matches_scalar_when_available() {
    if !avx2_available() {
        println!("skipped: AVX2 encode block test requires avx2");
        return;
    }

    let mut input = [0; 24];
    for seed in 0..64 {
        fill_pattern(&mut input, seed);

        let mut avx2_standard = [0x55; 32];
        let mut scalar_standard = [0xaa; 32];
        // SAFETY: The feature check above uses runtime AVX2 detection on
        // std builds and compile-time target-feature detection otherwise.
        unsafe {
            encode_24_bytes_avx2::<Standard>(&input, &mut avx2_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, avx2_standard.len());
        assert_eq!(avx2_standard, scalar_standard);

        let mut avx2_url_safe = [0x55; 32];
        let mut scalar_url_safe = [0xaa; 32];
        // SAFETY: The feature check above proves AVX2 availability for
        // this test invocation.
        unsafe {
            encode_24_bytes_avx2::<UrlSafe>(&input, &mut avx2_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, avx2_url_safe.len());
        assert_eq!(avx2_url_safe, scalar_url_safe);
    }

    for seed in 0..64 {
        fill_indices_pattern_wide(&mut input, seed);

        let mut avx2_standard = [0x55; 32];
        let mut scalar_standard = [0xaa; 32];
        // SAFETY: The feature check above proves AVX2 availability for this
        // test invocation.
        unsafe {
            encode_24_bytes_avx2::<Standard>(&input, &mut avx2_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, avx2_standard.len());
        assert_eq!(avx2_standard, scalar_standard);

        let mut avx2_url_safe = [0x55; 32];
        let mut scalar_url_safe = [0xaa; 32];
        // SAFETY: The feature check above proves AVX2 availability for this
        // test invocation.
        unsafe {
            encode_24_bytes_avx2::<UrlSafe>(&input, &mut avx2_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, avx2_url_safe.len());
        assert_eq!(avx2_url_safe, scalar_url_safe);
    }

    fill_indices_pattern_wide(&mut input, 0);
    let mut avx2_custom = [0x55; 32];
    let mut scalar_custom = [0xaa; 32];
    // SAFETY: The feature check above proves AVX2 availability for this test
    // invocation.
    unsafe {
        encode_24_bytes_avx2::<AnchorMatchingCustom>(&input, &mut avx2_custom);
    }
    let scalar_len = Engine::<AnchorMatchingCustom, true>::new()
        .encode_slice(&input, &mut scalar_custom)
        .unwrap();
    assert_eq!(scalar_len, avx2_custom.len());
    assert_eq!(avx2_custom, scalar_custom);
}

#[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
#[test]
fn ssse3_sse41_encode_block_matches_scalar_when_available() {
    if !ssse3_sse41_available() {
        println!("skipped: SSSE3/SSE4.1 encode block test requires ssse3 and sse4.1");
        return;
    }

    let mut input = [0; 12];
    for seed in 0..64 {
        fill_pattern(&mut input, seed);

        let mut ssse3_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The feature check above uses runtime SSSE3/SSE4.1
        // detection on std builds and compile-time target-feature
        // detection otherwise.
        unsafe {
            encode_12_bytes_ssse3_sse41::<Standard>(&input, &mut ssse3_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, ssse3_standard.len());
        assert_eq!(ssse3_standard, scalar_standard);

        let mut ssse3_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The feature check above proves SSSE3/SSE4.1
        // availability for this test invocation.
        unsafe {
            encode_12_bytes_ssse3_sse41::<UrlSafe>(&input, &mut ssse3_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, ssse3_url_safe.len());
        assert_eq!(ssse3_url_safe, scalar_url_safe);
    }

    for seed in 0..64 {
        fill_indices_pattern(&mut input, seed);

        let mut ssse3_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The feature check above proves SSSE3/SSE4.1 availability
        // for this test invocation.
        unsafe {
            encode_12_bytes_ssse3_sse41::<Standard>(&input, &mut ssse3_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, ssse3_standard.len());
        assert_eq!(ssse3_standard, scalar_standard);

        let mut ssse3_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The feature check above proves SSSE3/SSE4.1 availability
        // for this test invocation.
        unsafe {
            encode_12_bytes_ssse3_sse41::<UrlSafe>(&input, &mut ssse3_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, ssse3_url_safe.len());
        assert_eq!(ssse3_url_safe, scalar_url_safe);
    }

    fill_indices_pattern(&mut input, 0);
    let mut ssse3_custom = [0x55; 16];
    let mut scalar_custom = [0xaa; 16];
    // SAFETY: The feature check above proves SSSE3/SSE4.1 availability for
    // this test invocation.
    unsafe {
        encode_12_bytes_ssse3_sse41::<AnchorMatchingCustom>(&input, &mut ssse3_custom);
    }
    let scalar_len = Engine::<AnchorMatchingCustom, true>::new()
        .encode_slice(&input, &mut scalar_custom)
        .unwrap();
    assert_eq!(scalar_len, ssse3_custom.len());
    assert_eq!(ssse3_custom, scalar_custom);
}

#[cfg(any(
    target_arch = "aarch64",
    all(target_arch = "arm", target_feature = "neon")
))]
#[test]
fn neon_encode_block_matches_scalar_when_available() {
    if !neon_available() {
        println!("skipped: NEON encode block test requires aarch64 or arm+neon");
        return;
    }

    let mut input = [0; 12];
    for seed in 0..64 {
        fill_pattern(&mut input, seed);

        let mut neon_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The candidate check above proves NEON availability for
        // this test invocation.
        unsafe {
            encode_12_bytes_neon::<Standard>(&input, &mut neon_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, neon_standard.len());
        assert_eq!(neon_standard, scalar_standard);

        let mut neon_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The candidate check above proves NEON availability for
        // this test invocation.
        unsafe {
            encode_12_bytes_neon::<UrlSafe>(&input, &mut neon_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, neon_url_safe.len());
        assert_eq!(neon_url_safe, scalar_url_safe);
    }

    for seed in 0..64 {
        fill_indices_pattern(&mut input, seed);

        let mut neon_standard = [0x55; 16];
        let mut scalar_standard = [0xaa; 16];
        // SAFETY: The candidate check above proves NEON availability for
        // this test invocation.
        unsafe {
            encode_12_bytes_neon::<Standard>(&input, &mut neon_standard);
        }
        let scalar_len = Engine::<Standard, true>::new()
            .encode_slice(&input, &mut scalar_standard)
            .unwrap();
        assert_eq!(scalar_len, neon_standard.len());
        assert_eq!(neon_standard, scalar_standard);

        let mut neon_url_safe = [0x55; 16];
        let mut scalar_url_safe = [0xaa; 16];
        // SAFETY: The candidate check above proves NEON availability for
        // this test invocation.
        unsafe {
            encode_12_bytes_neon::<UrlSafe>(&input, &mut neon_url_safe);
        }
        let scalar_len = Engine::<UrlSafe, true>::new()
            .encode_slice(&input, &mut scalar_url_safe)
            .unwrap();
        assert_eq!(scalar_len, neon_url_safe.len());
        assert_eq!(neon_url_safe, scalar_url_safe);
    }

    fill_indices_pattern(&mut input, 0);
    let mut neon_custom = [0x55; 16];
    let mut scalar_custom = [0xaa; 16];
    // SAFETY: The candidate check above proves NEON availability for this
    // test invocation.
    unsafe {
        encode_12_bytes_neon::<AnchorMatchingCustom>(&input, &mut neon_custom);
    }
    let scalar_len = Engine::<AnchorMatchingCustom, true>::new()
        .encode_slice(&input, &mut scalar_custom)
        .unwrap();
    assert_eq!(scalar_len, neon_custom.len());
    assert_eq!(neon_custom, scalar_custom);
}
