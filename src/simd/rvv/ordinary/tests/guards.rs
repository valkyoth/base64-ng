//! Linux guard pages for partial VL loads and exact quantum stores.
#![cfg(not(miri))]
#[cfg(test)]
mod guard_pages {
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
            // SAFETY: Linux riscv64 libc ABI; no pointers supplied to getpagesize.
            let page = usize::try_from(unsafe { getpagesize() }).unwrap();
            // SAFETY: Private anonymous mapping of three pages, no file backing.
            let base =
                unsafe { mmap(core::ptr::null_mut(), page * 3, 3, 0x22, -1, 0) }.cast::<u8>();
            assert_ne!(base as usize, usize::MAX);
            let pages = Self { base, page };
            // SAFETY: The outer pages are aligned, owned parts of this mapping.
            unsafe {
                assert_eq!(mprotect(base.cast(), page, 0), 0);
                assert_eq!(mprotect(base.add(page * 2).cast(), page, 0), 0);
            }
            pages
        }
        fn region(&mut self, len: usize, end: bool) -> &mut [u8] {
            assert!(len <= self.page);
            let offset = if end { self.page * 2 - len } else { self.page };
            // SAFETY: Exclusive borrow entirely within the writable middle page.
            unsafe { core::slice::from_raw_parts_mut(self.base.add(offset), len) }
        }
    }
    impl Drop for Pages {
        fn drop(&mut self) {
            // SAFETY: Owned mapping, no borrowed slices remain live.
            unsafe {
                munmap(self.base.cast(), self.page * 3);
            }
        }
    }

    #[test]
    fn rvv_guard_pages_bound_partial_vl_and_stores() {
        assert!(super::super::executable());
        for end in [false, true] {
            for url in [false, true] {
                let mut source = Pages::new();
                let mut destination = Pages::new();
                for len in 0..=129 {
                    let input = source.region(len, end);
                    input.fill(b'A');
                    assert!(super::super::classify(input, url));
                    if len % 16 == 0 {
                        let output = destination.region(len / 4 * 3, end);
                        output.fill(0xa5);
                        assert!(super::super::decode_available(input, output, url));
                        assert!(output.iter().all(|&b| b == 0));
                    }
                    for pos in 0..len {
                        input[pos] = 0xff;
                        assert!(!super::super::classify(input, url));
                        input[pos] = b'A';
                    }
                }
            }
        }
    }
}
