use crate::DecodeError;

#[inline]
pub(crate) const fn ct_mask_bit(bit: u8) -> u8 {
    0u8.wrapping_sub(bit & 1)
}

#[inline]
pub(crate) const fn ct_mask_nonzero_u8(value: u8) -> u8 {
    let wide = value as u16;
    let negative = 0u16.wrapping_sub(wide);
    let nonzero = ((wide | negative) >> 8) as u8;
    ct_mask_bit(nonzero)
}

#[inline]
pub(crate) const fn ct_mask_eq_u8(left: u8, right: u8) -> u8 {
    !ct_mask_nonzero_u8(left ^ right)
}

#[inline]
pub(crate) const fn ct_mask_lt_u8(left: u8, right: u8) -> u8 {
    let diff = (left as u16).wrapping_sub(right as u16);
    ct_mask_bit((diff >> 8) as u8)
}

#[inline(never)]
pub(crate) fn constant_time_eq_public_len(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    constant_time_eq_same_len(left, right)
}

#[inline(never)]
pub(crate) fn constant_time_eq_fixed_width_array<const N: usize>(
    left: &[u8; N],
    right: &[u8; N],
) -> bool {
    constant_time_eq_same_len(left, right)
}

#[inline(never)]
#[allow(unsafe_code)]
fn constant_time_eq_same_len(left: &[u8], right: &[u8]) -> bool {
    let mut diff = 0u8;
    for (left, right) in left.iter().zip(right) {
        diff = ct_accumulate_u8(diff, *left ^ *right);
    }
    ct_error_gate_barrier(diff, 0);
    // SAFETY: `diff` is an initialized local `u8`; this final volatile read
    // keeps the public equality comparison dependent on a post-barrier load of
    // the accumulated value.
    let result = unsafe { core::ptr::read_volatile(&raw const diff) };
    result == 0
}

#[inline(never)]
#[allow(unsafe_code)]
pub(super) fn ct_accumulate_u8(accumulator: u8, value: u8) -> u8 {
    let result = core::hint::black_box(accumulator) | core::hint::black_box(value);
    // SAFETY: `result` is an initialized local `u8`; the volatile read is a
    // dependency-free optimizer barrier for the accumulation value and does not
    // access caller memory.
    unsafe { core::ptr::read_volatile(&raw const result) }
}

#[inline(never)]
#[allow(unsafe_code)]
pub(super) fn ct_error_gate_barrier(invalid_byte: u8, invalid_padding: u8) {
    core::hint::black_box(invalid_byte | invalid_padding);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);

    #[cfg(all(not(miri), not(kani), any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // SAFETY: `lfence` does not access memory and is used as a speculation
        // barrier before the public success/failure branch is observed.
        unsafe {
            core::arch::asm!("lfence", options(nostack, preserves_flags, nomem));
        }
    }

    #[cfg(all(not(miri), not(kani), target_arch = "aarch64"))]
    {
        // Older cores may treat CSDB as a no-op; runtime reporting marks this
        // as unattested until the deployment provides platform evidence.
        // SAFETY: these barriers do not access memory.
        unsafe {
            core::arch::asm!("isb sy", "hint #20", options(nostack, preserves_flags));
        }
    }

    #[cfg(all(not(miri), not(kani), target_arch = "arm"))]
    {
        // SAFETY: `isb sy` does not access memory and is used as the best
        // available stable ARM speculation boundary for this crate.
        unsafe {
            core::arch::asm!("isb sy", options(nostack, preserves_flags));
        }
    }

    #[cfg(all(
        not(miri),
        not(kani),
        any(target_arch = "riscv32", target_arch = "riscv64")
    ))]
    {
        // RISC-V base ISA does not provide a canonical speculation barrier.
        // `fence rw, rw` is the available ordering primitive for the CT public
        // result gate and is reported separately as `ordering-fence`; callers
        // on speculative RISC-V cores must use platform mitigations because
        // this does not satisfy `BackendPolicy::HighAssuranceScalarOnly`.
        // SAFETY: the assembly block does not access memory.
        unsafe {
            core::arch::asm!("fence rw, rw", options(nostack, preserves_flags));
        }
    }
}

pub(crate) fn report_ct_error(invalid_byte: u8, invalid_padding: u8) -> Result<(), DecodeError> {
    ct_error_gate_barrier(invalid_byte, invalid_padding);

    if (invalid_byte | invalid_padding) != 0 {
        Err(DecodeError::InvalidInput)
    } else {
        Ok(())
    }
}
