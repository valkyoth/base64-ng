//! Linux native guard-page checks for the non-admitted x86 validation candidates.
#![cfg(not(miri))]

use core::ffi::c_void;

unsafe extern "C" {
    fn getpagesize() -> i32;
    fn mmap(
        address: *mut c_void,
        length: usize,
        protection: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn mprotect(address: *mut c_void, length: usize, protection: i32) -> i32;
    fn munmap(address: *mut c_void, length: usize) -> i32;
}

struct Pages {
    base: *mut u8,
    page: usize,
}

impl Pages {
    fn new() -> Self {
        // SAFETY: Native Linux x86_64 libc ABI; no pointers are supplied to getpagesize.
        let page = usize::try_from(unsafe { getpagesize() }).unwrap();
        // SAFETY: Anonymous private mapping, fd/offset ignored; length is three pages.
        let base = unsafe { mmap(core::ptr::null_mut(), page * 3, 3, 0x22, -1, 0) }.cast::<u8>();
        assert_ne!(base as usize, usize::MAX);
        let pages = Self { base, page };
        // SAFETY: The owned mapping comprises three aligned pages; seal both outside pages.
        unsafe {
            assert_eq!(mprotect(base.cast(), page, 0), 0);
            assert_eq!(mprotect(base.add(page * 2).cast(), page, 0), 0);
        }
        pages
    }

    fn region(&mut self, length: usize, end: bool) -> &mut [u8] {
        assert!(length <= self.page);
        let offset = if end {
            self.page * 2 - length
        } else {
            self.page
        };
        // SAFETY: The slice lies wholly in the writable middle page. The exclusive
        // borrow prevents aliased slices or unmapping while it is live.
        unsafe { core::slice::from_raw_parts_mut(self.base.add(offset), length) }
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        // SAFETY: This object owns the complete mapping; all borrowed slices expired.
        unsafe {
            munmap(self.base.cast(), self.page * 3);
        }
    }
}

#[test]
fn validation_guard_pages_bound_loads_and_decode_stores() {
    if !super::ssse3_validation_candidate_available() {
        return;
    }
    for at_end in [false, true] {
        let mut source = Pages::new();
        let mut destination = Pages::new();
        let input: &mut [u8; 16] = source.region(16, at_end).try_into().unwrap();
        input.fill(b'A');
        let output: &mut [u8; 12] = destination.region(12, at_end).try_into().unwrap();
        for url_safe in [false, true] {
            assert!(super::candidate_validate_16(input, url_safe));
            assert!(super::candidate_decode_16(input, output, url_safe));
            assert_eq!(*output, [0; 12]);
            for lane in 0..16 {
                input[lane] = 0xff;
                output.fill(0xa5);
                assert!(!super::candidate_validate_16(input, url_safe));
                assert!(!super::candidate_decode_16(input, output, url_safe));
                assert_eq!(*output, [0xa5; 12]);
                input[lane] = b'A';
            }
        }
    }
}

#[test]
fn avx2_validation_guard_pages_cover_blocks_and_complete_decode() {
    if !super::avx2_validation_candidate_available() {
        return;
    }
    for at_end in [false, true] {
        let mut source = Pages::new();
        let mut destination = Pages::new();
        for url_safe in [false, true] {
            for blocks in 1..=3 {
                let input = source.region(blocks * 32, at_end);
                input.fill(b'A');
                let output = destination.region(blocks * 24, at_end);
                assert!(super::candidate_validate_avx2(input, url_safe));
                assert!(super::candidate_decode_avx2(input, output, url_safe));
                assert!(output.iter().all(|&byte| byte == 0));
                input[0] = b'!';
                output.fill(0xa5);
                assert!(!super::candidate_decode_avx2(input, output, url_safe));
                assert!(output.iter().all(|&byte| byte == 0xa5));
                input[0] = b'A';
                for lane in 0..input.len() {
                    input[lane] = 0xff;
                    assert!(!super::candidate_validate_avx2(input, url_safe));
                    input[lane] = b'A';
                }
            }
        }
        for settings in [
            crate::STRICT_STANDARD_PADDED.settings(),
            crate::STRICT_STANDARD_UNPADDED.settings(),
            crate::STRICT_URL_SAFE_PADDED.settings(),
            crate::STRICT_URL_SAFE_UNPADDED.settings(),
        ] {
            let codec = crate::CodecBuilder::new(*settings.alphabet())
                .encode_padding(settings.encode_padding())
                .decode_padding(settings.decode_padding())
                .build()
                .unwrap();
            for length in 0..=100 {
                let plain = [0x9b; 100];
                let mut encoded = [0; 136];
                let n = codec.encode_into(&plain[..length], &mut encoded).unwrap();
                let input = source.region(n, at_end);
                input.copy_from_slice(&encoded[..n]);
                let output = destination.region(length, at_end);
                assert_eq!(
                    crate::v2::decode_avx2_candidate_for_test(codec.settings(), input, output),
                    Ok(length)
                );
                assert_eq!(output, &plain[..length]);
                if n > 0 {
                    input[n - 1] = b'!';
                    output.fill(0xa5);
                    assert!(
                        crate::v2::decode_avx2_candidate_for_test(codec.settings(), input, output)
                            .is_err()
                    );
                    assert!(output.iter().all(|&byte| byte == 0xa5));
                }
            }
        }
    }
}

#[test]
fn avx512_validation_guard_pages_cover_exact_masked_stores() {
    if !super::avx512_validation_candidate_available() {
        return;
    }
    for at_end in [false, true] {
        let mut source = Pages::new();
        let mut destination = Pages::new();
        for url_safe in [false, true] {
            for blocks in 1..=3 {
                let input = source.region(blocks * 64, at_end);
                input.fill(b'A');
                let output = destination.region(blocks * 48, at_end);
                assert!(super::candidate_validate_avx512(input, url_safe));
                assert!(super::candidate_decode_avx512(input, output, url_safe));
                assert!(output.iter().all(|&byte| byte == 0));
                output.fill(0xa5);
                for requested in [input.len() + 1, usize::MAX] {
                    assert_eq!(
                        super::test_avx512_loop_geometry(input, output, requested, url_safe),
                        Some((0, 0, false))
                    );
                    assert!(output.iter().all(|&byte| byte == 0xa5));
                }
                let short_len = output.len() - 1;
                assert_eq!(
                    super::test_avx512_loop_geometry(
                        input,
                        &mut output[..short_len],
                        input.len(),
                        url_safe
                    ),
                    Some((0, 0, false))
                );
                assert!(output.iter().all(|&byte| byte == 0xa5));
                for lane in 0..input.len() {
                    input[lane] = 0xff;
                    output.fill(0xa5);
                    assert!(!super::candidate_validate_avx512(input, url_safe));
                    assert!(!super::candidate_decode_avx512(input, output, url_safe));
                    // The low-level loop may store earlier valid blocks; the
                    // rejected block and everything after it remain untouched.
                    let written = lane / 64 * 48;
                    assert!(output[..written].iter().all(|&byte| byte == 0));
                    assert!(output[written..].iter().all(|&byte| byte == 0xa5));
                    input[lane] = b'A';
                }
            }
        }
        for settings in [
            crate::STRICT_STANDARD_PADDED.settings(),
            crate::STRICT_STANDARD_UNPADDED.settings(),
            crate::STRICT_URL_SAFE_PADDED.settings(),
            crate::STRICT_URL_SAFE_UNPADDED.settings(),
        ] {
            let codec = crate::CodecBuilder::new(*settings.alphabet())
                .encode_padding(settings.encode_padding())
                .decode_padding(settings.decode_padding())
                .build()
                .unwrap();
            for length in 0..=200 {
                let plain = [0x9b; 200];
                let mut encoded = [0; 268];
                let n = codec.encode_into(&plain[..length], &mut encoded).unwrap();
                let input = source.region(n, at_end);
                input.copy_from_slice(&encoded[..n]);
                let output = destination.region(length, at_end);
                assert_eq!(
                    crate::v2::decode_avx512_candidate_for_test(codec.settings(), input, output),
                    Ok(length)
                );
                assert_eq!(output, &plain[..length]);
                if n > 0 {
                    input[n - 1] = b'!';
                    output.fill(0xa5);
                    assert!(
                        crate::v2::decode_avx512_candidate_for_test(
                            codec.settings(),
                            input,
                            output
                        )
                        .is_err()
                    );
                    assert!(output.iter().all(|&byte| byte == 0xa5));
                }
            }
        }
    }
}
