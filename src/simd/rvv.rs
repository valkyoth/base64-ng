//! Non-admitted RVV 1.0 encode/decode candidate.
//!
//! Stable Rust 1.97.1 recognizes the `v` target feature but does not stabilize
//! per-function RVV intrinsics or `#[target_feature(enable = "v")]`. This
//! module therefore isolates the candidate in leaf `global_asm!` functions.
//! It is compiled only by project-owned QEMU and native admission evidence
//! through the internal `base64_ng_rvv_candidate` cfg. Normal crate builds do
//! not compile or dispatch this code.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the RVV candidate is compiled for codegen evidence before production dispatch admission"
    )
)]

use crate::{Alphabet, DecodeError, EncodeError, Standard, checked_encoded_len, scalar};

const ENCODE_INPUT_BLOCK: usize = 12;
const DECODE_INPUT_BLOCK: usize = 16;

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
        li t1, 26
        li t2, 52
        li t3, 75
        li t4, 62
        li t6, 63
    .Lbase64_ng_rvv_encode_loop_\@:
        vsetvli a3, a2, e8, m1, ta, ma
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
        base64_ng_rvv_encode_map v4, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v5, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v6, v8, \delta62, \delta63
        base64_ng_rvv_encode_map v7, v8, \delta62, \delta63
        vsseg4e8.v v4, (a1)

        slli a4, a3, 1
        add a4, a4, a3
        add a0, a0, a4
        slli a4, a3, 2
        add a1, a1, a4
        sub a2, a2, a3
        bnez a2, .Lbase64_ng_rvv_encode_loop_\@

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
        li t0, 65
        li t1, 97
        li t2, 71
        li t3, 58
        li t4, \ascii62
        li t5, \ascii63
    .Lbase64_ng_rvv_decode_loop_\@:
        vsetvli a3, a2, e8, m1, ta, ma
        vlseg4e8.v v1, (a0)

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

        slli a4, a3, 2
        add a0, a0, a4
        slli a4, a3, 1
        add a4, a4, a3
        add a1, a1, a4
        sub a2, a2, a3
        bnez a2, .Lbase64_ng_rvv_decode_loop_\@

        base64_ng_rvv_clear
        ret
        .cfi_endproc
        .size \name, .-\name
    .endm

    base64_ng_rvv_encode base64_ng_rvv_encode_standard_quanta, -15, -12
    base64_ng_rvv_encode base64_ng_rvv_encode_url_safe_quanta, -13, 36
    base64_ng_rvv_decode base64_ng_rvv_decode_standard_quanta, 43, 47
    base64_ng_rvv_decode base64_ng_rvv_decode_url_safe_quanta, 45, 95

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

    .p2align 2
    .global base64_ng_rvv_signal_context_round_trip
    .hidden base64_ng_rvv_signal_context_round_trip
    .type base64_ng_rvv_signal_context_round_trip, @function
base64_ng_rvv_signal_context_round_trip:
    .cfi_startproc
    mv t1, a0
    mv t2, a1
    mv t3, a2
    vsetivli zero, 16, e8, m1, ta, ma
    li t0, 90
    vmv.v.x v8, t0
    li t0, 1
    amoswap.w.rl zero, t0, (t2)
    li t0, 250000000
.Lbase64_ng_rvv_signal_wait:
    amoadd.w.aq t4, zero, (t3)
    bnez t4, .Lbase64_ng_rvv_signal_done
    addi t0, t0, -1
    bnez t0, .Lbase64_ng_rvv_signal_wait
.Lbase64_ng_rvv_signal_done:
    amoswap.w.rl zero, zero, (t2)
    vse8.v v8, (t1)
    base64_ng_rvv_clear
    ret
    .cfi_endproc
    .size base64_ng_rvv_signal_context_round_trip, .-base64_ng_rvv_signal_context_round_trip

    .p2align 2
    .global base64_ng_rvv_signal_clobber
    .hidden base64_ng_rvv_signal_clobber
    .type base64_ng_rvv_signal_clobber, @function
base64_ng_rvv_signal_clobber:
    .cfi_startproc
    vsetivli zero, 16, e8, m1, ta, ma
    li t0, 165
    vmv.v.x v8, t0
    ret
    .cfi_endproc
    .size base64_ng_rvv_signal_clobber, .-base64_ng_rvv_signal_clobber

    .option pop
    "#,
    options(raw)
);

unsafe extern "C" {
    fn base64_ng_rvv_encode_standard_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_encode_url_safe_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_decode_standard_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_decode_url_safe_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_vlenb() -> usize;
    fn base64_ng_rvv_signal_context_round_trip(
        output: *mut u8,
        armed: *mut u32,
        delivered: *mut u32,
    );
    fn base64_ng_rvv_signal_clobber();
}

#[cfg(test)]
pub(super) fn vector_length_bytes() -> usize {
    // SAFETY: Candidate tests call this only after `available()` proves RVV
    // and enabled vector state on the current thread.
    unsafe { base64_ng_rvv_vlenb() }
}

#[cfg(test)]
pub(super) unsafe fn signal_context_round_trip(
    output: *mut u8,
    armed: *mut u32,
    delivered: *mut u32,
) {
    // SAFETY: The caller provides 16 writable output bytes, aligned atomic
    // words, and installs the reviewed timer handler before this helper runs.
    unsafe { base64_ng_rvv_signal_context_round_trip(output, armed, delivered) };
}

#[cfg(test)]
pub(super) unsafe extern "C" fn signal_clobber(_signal: i32) {
    if super::rvv_tests::SIGNAL_ARMED.load(core::sync::atomic::Ordering::Acquire) == 0 {
        return;
    }
    // SAFETY: The native evidence gate proves vector state is enabled before
    // installing this signal handler.
    unsafe { base64_ng_rvv_signal_clobber() };
    super::rvv_tests::SIGNAL_DELIVERED.store(1, core::sync::atomic::Ordering::Release);
}

pub(crate) fn available() -> bool {
    #[cfg(all(feature = "std", target_os = "linux"))]
    {
        // Linux does not permit an enabled thread to turn Vector off again.
        // Caching a positive result per thread is therefore stable until
        // `execve`, while a stale negative remains a safe scalar fallback.
        std::thread_local! {
            static AVAILABLE: bool = detect_linux_rvv();
        }
        AVAILABLE.with(|available| *available)
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
    encode_slice_with_availability::<A, PAD>(input, output, available())
}

fn encode_slice_with_availability<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    rvv_available: bool,
) -> Result<usize, EncodeError> {
    if input.len() < ENCODE_INPUT_BLOCK || !supports_alphabet::<A>() || !rvv_available {
        return scalar::encode_slice::<A, PAD>(input, output);
    }
    let required = checked_encoded_len(input.len(), PAD).ok_or(EncodeError::LengthOverflow)?;
    if output.len() < required {
        return Err(EncodeError::OutputTooSmall {
            required,
            available: output.len(),
        });
    }

    let quanta = input.len() / 3;
    let read = quanta * 3;
    let write = quanta * 4;
    // SAFETY: The quotient proves exact complete-quantum input/output bounds.
    // The per-thread availability gate proves RVV and enabled vector state.
    unsafe { encode_quanta::<A>(input.as_ptr(), output.as_mut_ptr(), quanta) };
    let tail = scalar::encode_slice::<A, PAD>(&input[read..], &mut output[write..])?;
    Ok(write + tail)
}

pub(crate) fn decode_slice<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    decode_slice_with_availability::<A, PAD>(input, output, available())
}

fn decode_slice_with_availability<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
    rvv_available: bool,
) -> Result<usize, DecodeError> {
    if input.len() < DECODE_INPUT_BLOCK || !supports_alphabet::<A>() || !rvv_available {
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
    let quanta = simd_input_len / 4;
    let read = quanta * 4;
    let write = quanta * 3;
    // SAFETY: Whole-input scalar validation proves classification and
    // canonicality. The quotient proves complete-quantum bounds, and the
    // per-thread availability gate proves RVV and enabled vector state.
    unsafe { decode_quanta::<A>(input.as_ptr(), output.as_mut_ptr(), quanta) };
    let tail = scalar::decode_slice::<A, PAD>(&input[read..], &mut output[write..])
        .map_err(|error| error.with_index_offset(read))?;
    Ok(write + tail)
}

#[cfg(test)]
pub(super) fn encode_slice_unavailable_for_test<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    encode_slice_with_availability::<A, PAD>(input, output, false)
}

#[cfg(test)]
pub(super) fn decode_slice_unavailable_for_test<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, DecodeError> {
    decode_slice_with_availability::<A, PAD>(input, output, false)
}

unsafe fn encode_quanta<A: Alphabet>(input: *const u8, output: *mut u8, quanta: usize) {
    if A::ENCODE[62] == b'-' {
        // SAFETY: The caller owns the complete-quantum bounds and RVV contract.
        unsafe { base64_ng_rvv_encode_url_safe_quanta(input, output, quanta) };
    } else {
        // SAFETY: The caller owns the complete-quantum bounds and RVV contract.
        unsafe { base64_ng_rvv_encode_standard_quanta(input, output, quanta) };
    }
}

unsafe fn decode_quanta<A: Alphabet>(input: *const u8, output: *mut u8, quanta: usize) {
    if A::ENCODE[62] == b'-' {
        // SAFETY: The caller owns the complete-quantum bounds and RVV contract.
        unsafe { base64_ng_rvv_decode_url_safe_quanta(input, output, quanta) };
    } else {
        // SAFETY: The caller owns the complete-quantum bounds and RVV contract.
        unsafe { base64_ng_rvv_decode_standard_quanta(input, output, quanta) };
    }
}
