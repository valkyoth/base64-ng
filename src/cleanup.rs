//! Best-effort dependency-free memory cleanup helpers.

#[inline(never)]
#[allow(unsafe_code)]
pub(crate) fn wipe_bytes(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        // SAFETY: `byte` comes from a unique mutable slice iterator, so the
        // pointer is non-null, aligned, valid for one `u8` write, and does not
        // alias another live mutable reference during this iteration.
        unsafe {
            core::ptr::write_volatile(byte, 0);
        }
    }
    wipe_barrier(bytes.as_mut_ptr(), bytes.len());
}

#[inline(never)]
#[allow(unsafe_code)]
fn wipe_barrier(ptr: *mut u8, len: usize) {
    let _ = (ptr, len);

    #[cfg(all(not(miri), any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // `mfence` orders prior stores before later memory operations on
        // x86/x86_64, while the pointer and length are opaque optimizer inputs.
        // SAFETY: the assembly block does not read or write through the pointer.
        unsafe {
            core::arch::asm!(
                "mfence",
                "/* {0} {1} */",
                in(reg) ptr,
                in(reg) len,
                options(nostack, preserves_flags)
            );
        }
    }

    #[cfg(all(not(miri), target_arch = "aarch64"))]
    {
        // `dsb sy` completes prior explicit memory accesses before later
        // instructions, and `isb sy` flushes subsequent instruction context.
        // SAFETY: the assembly block does not read or write through the pointer.
        unsafe {
            core::arch::asm!(
                "dsb sy",
                "isb sy",
                "hint #20",
                "/* {0} {1} */",
                in(reg) ptr,
                in(reg) len,
                options(nostack, preserves_flags)
            );
        }
    }

    #[cfg(all(not(miri), target_arch = "arm"))]
    {
        // `dsb sy` completes prior explicit memory accesses before later
        // instructions, and `isb sy` flushes subsequent instruction context.
        // SAFETY: the assembly block does not read or write through the pointer.
        unsafe {
            core::arch::asm!(
                "dsb sy",
                "isb sy",
                "/* {0} {1} */",
                in(reg) ptr,
                in(reg) len,
                options(nostack, preserves_flags)
            );
        }
    }

    #[cfg(all(not(miri), any(target_arch = "riscv32", target_arch = "riscv64")))]
    {
        // `fence rw, rw` orders prior reads/writes before later reads/writes.
        // SAFETY: the assembly block does not read or write through the pointer.
        unsafe {
            core::arch::asm!(
                "fence rw, rw",
                "/* {0} {1} */",
                in(reg) ptr,
                in(reg) len,
                options(nostack, preserves_flags)
            );
        }
    }

    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

pub(crate) fn wipe_tail(bytes: &mut [u8], start: usize) {
    wipe_bytes(&mut bytes[start..]);
}

#[cfg(feature = "alloc")]
#[allow(unsafe_code)]
pub(crate) fn wipe_vec_spare_capacity(bytes: &mut alloc::vec::Vec<u8>) {
    let ptr = bytes.as_mut_ptr();
    let len = bytes.len();
    let capacity = bytes.capacity();
    let spare = capacity - len;
    if spare == 0 {
        return;
    }

    let mut offset = len;
    while offset < capacity {
        // SAFETY: `offset` is within the vector allocation's spare capacity, so
        // the pointer is valid, aligned, and writable for one `u8`. This writes
        // a zero byte without reading the prior uninitialized value.
        unsafe {
            core::ptr::write_volatile(ptr.add(offset), 0);
        }
        offset += 1;
    }
    // SAFETY: `spare > 0`, so `len < capacity` and `ptr.add(len)` points
    // inside the vector allocation at the first spare-capacity byte.
    let spare_ptr = unsafe { ptr.add(len) };
    wipe_barrier(spare_ptr, spare);
}

#[cfg(feature = "alloc")]
pub(crate) fn wipe_vec_all(bytes: &mut alloc::vec::Vec<u8>) {
    wipe_bytes(bytes);
    wipe_vec_spare_capacity(bytes);
}
