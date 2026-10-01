//! Linux guard-page checks for production NEON validation.
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
        // SAFETY: Native Linux AArch64 libc ABI; no pointers are supplied to getpagesize.
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
fn neon_validation_guard_pages_bound_loads_and_stores() {
    for at_end in [false, true] {
        for url in [false, true] {
            let mut source = Pages::new();
            let mut destination = Pages::new();
            for length in [0, 16, 32, 48, 512, 1024] {
                let input = source.region(length, at_end);
                input.fill(b'A');
                let output = destination.region(length / 4 * 3, at_end);
                output.fill(0xa5);
                assert!(super::validate(crate::runtime::Backend::Neon, input, url));
                assert!(super::decode(
                    crate::runtime::Backend::Neon,
                    input,
                    output,
                    url
                ));
                assert!(output.iter().all(|&byte| byte == 0));
                for position in 0..length {
                    input[position] = 0xff;
                    assert!(!super::validate(crate::runtime::Backend::Neon, input, url));
                    input[position] = b'A';
                }
                if length != 0 {
                    output.fill(0xa5);
                    assert!(!super::decode(
                        crate::runtime::Backend::Neon,
                        input,
                        &mut output[..0],
                        url
                    ));
                    assert!(output.iter().all(|&byte| byte == 0xa5));
                    input[0] = b'=';
                    assert!(!super::decode(
                        crate::runtime::Backend::Neon,
                        input,
                        output,
                        url
                    ));
                    assert!(output.iter().all(|&byte| byte == 0xa5));
                }
            }
        }
    }
}

#[test]
fn neon_public_guard_pages_preserve_transactional_output() {
    for at_end in [false, true] {
        for length in [508, 512, 516, 1024, 1028, 4088, 4092, 4096] {
            let mut source = Pages::new();
            let mut destination = Pages::new();
            let input = source.region(length, at_end);
            input.fill(b'A');
            let output = destination.region(length / 4 * 3, at_end);
            for settings in [
                crate::STRICT_STANDARD_PADDED.settings(),
                crate::STRICT_URL_SAFE_PADDED.settings(),
            ] {
                let codec = crate::CodecBuilder::new(*settings.alphabet())
                    .encode_padding(settings.encode_padding())
                    .decode_padding(settings.decode_padding())
                    .build()
                    .unwrap();
                output.fill(0xa5);
                assert_eq!(codec.decode_into(input, output), Ok(output.len()));
                assert!(output.iter().all(|&b| b == 0));
                for position in [0, 15, length - 4, length - 1] {
                    input[position] = b'!';
                    output.fill(0xa5);
                    assert!(codec.decode_into(input, output).is_err());
                    assert!(output.iter().all(|&b| b == 0xa5));
                    input[position] = b'A';
                }
                output.fill(0xa5);
                assert!(codec.decode_into(input, &mut output[..0]).is_err());
                assert!(output.iter().all(|&b| b == 0xa5));
            }
        }
    }
}
