//! Non-admitted RVV 1.0 encode/decode candidate.
//!
//! Stable Rust 1.97.1 recognizes the `v` target feature but does not stabilize
//! per-function RVV intrinsics or `#[target_feature(enable = "v")]`. This
//! module therefore isolates the candidate in leaf `global_asm!` functions.
//! It is compiled only by project-owned QEMU evidence through the internal
//! `base64_ng_rvv_candidate` cfg. Normal crate builds do not compile or
//! dispatch this code.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the RVV candidate is compiled for codegen evidence before production dispatch admission"
    )
)]

use crate::{Alphabet, DecodeError, EncodeError, Standard, checked_encoded_len, scalar};

const ENCODE_INPUT_BLOCK: usize = 12;
const ENCODE_OUTPUT_BLOCK: usize = 16;
const DECODE_INPUT_BLOCK: usize = 16;
const DECODE_OUTPUT_BLOCK: usize = 12;

core::arch::global_asm!(
    r#"
    .attribute arch, "rv64gcv"
    .option push
    .option arch, +v

    .macro base64_ng_rvv_clear
        li t0, -1
        vsetvli zero, t0, e8, m1, ta, ma
        vmv.v.i v0, 0
        vmv.v.i v1, 0
        vmv.v.i v2, 0
        vmv.v.i v3, 0
        vmv.v.i v4, 0
        vmv.v.i v5, 0
        vmv.v.i v6, 0
        vmv.v.i v7, 0
        vmv.v.i v8, 0
        vmv.v.i v9, 0
        vmv.v.i v10, 0
        vmv.v.i v11, 0
        vmv.v.i v12, 0
        vmv.v.i v13, 0
        vmv.v.i v14, 0
        vmv.v.i v15, 0
    .endm

    .macro base64_ng_rvv_encode_map value, scratch, delta62, delta63
        vmv.v.v \scratch, \value
        vadd.vx \value, \value, t0
        vmsgeu.vx v0, \scratch, t1
        vadd.vi \value, \value, 6, v0.t
        vmsgeu.vx v0, \scratch, t2
        vsub.vx \value, \value, t3, v0.t
        vmseq.vx v0, \scratch, t4
        li t5, \delta62
        vadd.vx \value, \value, t5, v0.t
        vmseq.vx v0, \scratch, t6
        li t5, \delta63
        vadd.vx \value, \value, t5, v0.t
    .endm

    .macro base64_ng_rvv_encode name, delta62, delta63
        .p2align 2
        .global \name
        .hidden \name
        .type \name, @function
    \name:
        .cfi_startproc
        vsetivli zero, 4, e8, m1, ta, ma
        vlseg3e8.v v1, (a0)

        vsrl.vi v4, v1, 2
        vand.vi v5, v1, 3
        vsll.vi v5, v5, 4
        vsrl.vi v8, v2, 4
        vor.vv v5, v5, v8
        vand.vi v6, v2, 15
        vsll.vi v6, v6, 2
        vsrl.vi v8, v3, 6
        vor.vv v6, v6, v8
        li t0, 63
        vand.vx v7, v3, t0

        li t0, 65
        li t1, 26
        li t2, 52
        li t3, 75
        li t4, 62
        li t6, 63
        base64_ng_rvv_encode_map v4, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v5, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v6, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v7, v8, \delta62, \delta63
        vsseg4e8.v v4, (a1)

        base64_ng_rvv_clear
        ret
        .cfi_endproc
        .size \name, .-\name
    .endm

    .macro base64_ng_rvv_decode_map ascii, value, scratch, special62, special63
        vsub.vx \value, \ascii, t0
        vmsgeu.vx v0, \ascii, t1
        vsub.vx \scratch, \ascii, t2
        vmerge.vvm \value, \value, \scratch, v0
        vmsltu.vx v0, \ascii, t3
        vadd.vi \scratch, \ascii, 4
        vmerge.vvm \value, \value, \scratch, v0
        vmseq.vx v0, \ascii, \special62
        li t6, 62
        vmv.v.x \scratch, t6
        vmerge.vvm \value, \value, \scratch, v0
        vmseq.vx v0, \ascii, \special63
        li t6, 63
        vmv.v.x \scratch, t6
        vmerge.vvm \value, \value, \scratch, v0
    .endm

    .macro base64_ng_rvv_decode name, ascii62, ascii63
        .p2align 2
        .global \name
        .hidden \name
        .type \name, @function
    \name:
        .cfi_startproc
        vsetivli zero, 4, e8, m1, ta, ma
        vlseg4e8.v v1, (a0)

        li t0, 65
        li t1, 97
        li t2, 71
        li t3, 58
        li t4, \ascii62
        li t5, \ascii63
        base64_ng_rvv_decode_map v1, v5, v9, t4, t5
        base64_ng_rvv_decode_map v2, v6, v9, t4, t5
        base64_ng_rvv_decode_map v3, v7, v9, t4, t5
        base64_ng_rvv_decode_map v4, v8, v9, t4, t5

        vsll.vi v10, v5, 2
        vsrl.vi v12, v6, 4
        vor.vv v10, v10, v12
        vsll.vi v11, v6, 4
        vsrl.vi v12, v7, 2
        vor.vv v11, v11, v12
        vsll.vi v12, v7, 6
        vor.vv v12, v12, v8
        vsseg3e8.v v10, (a1)

        base64_ng_rvv_clear
        ret
        .cfi_endproc
        .size \name, .-\name
    .endm

    base64_ng_rvv_encode base64_ng_rvv_encode_standard_12, -15, -12
    base64_ng_rvv_encode base64_ng_rvv_encode_url_safe_12, -13, 36
    base64_ng_rvv_decode base64_ng_rvv_decode_standard_16, 43, 47
    base64_ng_rvv_decode base64_ng_rvv_decode_url_safe_16, 45, 95

    .p2align 2
    .global base64_ng_rvv_vlenb
    .hidden base64_ng_rvv_vlenb
    .type base64_ng_rvv_vlenb, @function
base64_ng_rvv_vlenb:
    .cfi_startproc
    vsetvli zero, zero, e8, m1, ta, ma
    csrr a0, vlenb
    ret
    .cfi_endproc
    .size base64_ng_rvv_vlenb, .-base64_ng_rvv_vlenb

    .option pop
    "#,
    options(raw)
);

unsafe extern "C" {
    fn base64_ng_rvv_encode_standard_12(input: *const u8, output: *mut u8);
    fn base64_ng_rvv_encode_url_safe_12(input: *const u8, output: *mut u8);
    fn base64_ng_rvv_decode_standard_16(input: *const u8, output: *mut u8);
    fn base64_ng_rvv_decode_url_safe_16(input: *const u8, output: *mut u8);
    fn base64_ng_rvv_vlenb() -> usize;
}

#[cfg(test)]
pub(super) fn vector_length_bytes() -> usize {
    // SAFETY: Candidate tests call this only after `available()` proves RVV
    // and enabled vector state on the current thread.
    unsafe { base64_ng_rvv_vlenb() }
}

pub(crate) fn available() -> bool {
    #[cfg(all(feature = "std", target_os = "linux"))]
    {
        static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *AVAILABLE.get_or_init(detect_linux_rvv)
    }
    #[cfg(all(feature = "std", not(target_os = "linux")))]
    {
        false
    }
    #[cfg(not(feature = "std"))]
    {
        cfg!(target_feature = "v")
    }
}

#[cfg(all(feature = "std", target_os = "linux"))]
fn detect_linux_rvv() -> bool {
    const AT_HWCAP: usize = 16;
    const HWCAP_V: usize = 1 << (b'v' - b'a');
    const RISCV_HWPROBE_SYSCALL: isize = 258;
    const RISCV_HWPROBE_KEY_IMA_EXT_0: i64 = 4;
    const RISCV_HWPROBE_IMA_V: u64 = 1 << 2;
    const PR_RISCV_V_GET_CONTROL: i32 = 70;

    #[repr(C)]
    struct HwProbe {
        key: i64,
        value: u64,
    }

    unsafe extern "C" {
        fn getauxval(kind: usize) -> usize;
        fn prctl(option: i32, ...) -> i32;
        fn syscall(number: isize, ...) -> isize;
    }

    // SAFETY: These are Linux RISC-V UAPI calls with the exact declared ABI.
    // The probe pointer is valid for one writable pair, CPU-set size zero
    // permits a null CPU-set pointer, and all failures are treated as absent.
    unsafe {
        let auxv_has_v = getauxval(AT_HWCAP) & HWCAP_V != 0;
        let mut probe = HwProbe {
            key: RISCV_HWPROBE_KEY_IMA_EXT_0,
            value: 0,
        };
        let hwprobe_ok = syscall(
            RISCV_HWPROBE_SYSCALL,
            &raw mut probe,
            1usize,
            0usize,
            core::ptr::null_mut::<usize>(),
            0u32,
        ) == 0;
        let hwprobe_has_v = hwprobe_ok
            && probe.key == RISCV_HWPROBE_KEY_IMA_EXT_0
            && probe.value & RISCV_HWPROBE_IMA_V != 0;
        let vector_control = prctl(PR_RISCV_V_GET_CONTROL, 0usize, 0usize, 0usize, 0usize);
        probe_allows_rvv(hwprobe_ok, hwprobe_has_v, auxv_has_v, vector_control)
    }
}

#[cfg(any(test, all(feature = "std", target_os = "linux")))]
pub(super) const fn probe_allows_rvv(
    hwprobe_ok: bool,
    hwprobe_has_v: bool,
    auxv_has_v: bool,
    vector_control: i32,
) -> bool {
    const VSTATE_CURRENT_MASK: i32 = 3;
    const VSTATE_ON: i32 = 2;

    let hardware_has_v = if hwprobe_ok {
        hwprobe_has_v
    } else {
        auxv_has_v
    };
    let vector_state_on = if vector_control >= 0 {
        vector_control & VSTATE_CURRENT_MASK == VSTATE_ON
    } else {
        auxv_has_v
    };
    hardware_has_v && vector_state_on
}

pub(crate) fn supports_alphabet<A: Alphabet>() -> bool {
    let mut index = 0;
    while index < 62 {
        if A::ENCODE[index] != Standard::ENCODE[index] {
            return false;
        }
        index += 1;
    }
    matches!((A::ENCODE[62], A::ENCODE[63]), (b'+', b'/') | (b'-', b'_'))
}

pub(crate) fn encode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    if input.len() < ENCODE_INPUT_BLOCK || !supports_alphabet::<A>() {
        return scalar::encode_slice::<A, PAD>(input, output);
    }
    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let mut read = 0;
    let mut write = 0;
    while read + ENCODE_INPUT_BLOCK <= input.len() {
        // SAFETY: The loop proves exact fixed input/output block bounds. The
        // caller-side candidate gate proves RVV and enabled vector state.
        unsafe {
            encode_block::<A>(input.as_ptr().add(read), output.as_mut_ptr().add(write));
        }
        read += ENCODE_INPUT_BLOCK;
        write += ENCODE_OUTPUT_BLOCK;
    }
    let tail = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail)
}

pub(crate) fn decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    if input.len() < DECODE_INPUT_BLOCK || !supports_alphabet::<A>() {
        return scalar::decode_slice::<A, PAD>(input, output);
    }
    let required = scalar::validate_decode::<A, PAD>(input)?;
    if output.len() < required {
        return Err(DecodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let simd_input_len = if input.last() == Some(&b'=') {
        input.len().saturating_sub(4)
    } else {
        input.len()
    };
    let mut read = 0;
    let mut write = 0;
    while read + DECODE_INPUT_BLOCK <= simd_input_len {
        // SAFETY: Whole-input scalar validation proves classification and
        // canonicality; the loop proves exact fixed block bounds; the
        // caller-side candidate gate proves RVV and enabled vector state.
        unsafe {
            decode_block::<A>(input.as_ptr().add(read), output.as_mut_ptr().add(write));
        }
        read += DECODE_INPUT_BLOCK;
        write += DECODE_OUTPUT_BLOCK;
    }
    let tail = scalar::decode_slice::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail)
}

unsafe fn encode_block<A: Alphabet>(input: *const u8, output: *mut u8) {
    if A::ENCODE[62] == b'-' {
        // SAFETY: The caller owns the fixed block bounds and RVV contract.
        unsafe { base64_ng_rvv_encode_url_safe_12(input, output) };
    } else {
        // SAFETY: The caller owns the fixed block bounds and RVV contract.
        unsafe { base64_ng_rvv_encode_standard_12(input, output) };
    }
}

unsafe fn decode_block<A: Alphabet>(input: *const u8, output: *mut u8) {
    if A::ENCODE[62] == b'-' {
        // SAFETY: The caller owns the fixed block bounds and RVV contract.
        unsafe { base64_ng_rvv_decode_url_safe_16(input, output) };
    } else {
        // SAFETY: The caller owns the fixed block bounds and RVV contract.
        unsafe { base64_ng_rvv_decode_standard_16(input, output) };
    }
}
