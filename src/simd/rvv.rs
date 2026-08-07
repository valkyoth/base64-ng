//! Exact-profile RVV 1.0 encode/decode backend.
//!
//! Stable Rust 1.97.1 recognizes the `v` target feature but does not stabilize
//! per-function RVV intrinsics or `#[target_feature(enable = "v")]`. This
//! module therefore isolates RVV instructions in leaf `global_asm!` functions.
//! Production dispatch is fail-closed to Linux on the measured `SpacemiT` X60
//! profile. The internal `base64_ng_rvv_candidate` cfg retains broader QEMU
//! execution for candidate evidence without widening production admission.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "RVV evidence-only helpers are not reachable from every production feature combination"
    )
)]

use crate::{Alphabet, DecodeError, EncodeError, Standard, checked_encoded_len, scalar};

mod asm;

const ENCODE_INPUT_BLOCK: usize = 12;
const DECODE_INPUT_BLOCK: usize = 16;

unsafe extern "C" {
    fn base64_ng_rvv_encode_standard_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_encode_url_safe_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_decode_standard_quanta(input: *const u8, output: *mut u8, quanta: usize);
    fn base64_ng_rvv_decode_url_safe_quanta(input: *const u8, output: *mut u8, quanta: usize);
    #[cfg(base64_ng_rvv_candidate)]
    fn base64_ng_rvv_vlenb() -> usize;
    #[cfg(base64_ng_rvv_candidate)]
    fn base64_ng_rvv_signal_context_round_trip(
        output: *mut u8,
        armed: *mut u32,
        delivered: *mut u32,
    );
    #[cfg(base64_ng_rvv_candidate)]
    fn base64_ng_rvv_signal_clobber();
}

#[cfg(all(test, base64_ng_rvv_candidate))]
pub(super) fn vector_length_bytes() -> usize {
    // SAFETY: Candidate tests call this only after `available()` proves RVV
    // and enabled vector state on the current thread.
    unsafe { base64_ng_rvv_vlenb() }
}

#[cfg(all(test, base64_ng_rvv_candidate))]
pub(super) unsafe fn signal_context_round_trip(
    output: *mut u8,
    armed: *mut u32,
    delivered: *mut u32,
) {
    // SAFETY: The caller provides 16 writable output bytes, aligned atomic
    // words, and installs the reviewed timer handler before this helper runs.
    unsafe { base64_ng_rvv_signal_context_round_trip(output, armed, delivered) };
}

#[cfg(all(test, base64_ng_rvv_candidate))]
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
            static AVAILABLE: bool = detect_linux_x60_rvv();
        }
        AVAILABLE.with(|available| *available)
    }
    #[cfg(all(feature = "std", not(target_os = "linux")))]
    {
        false
    }
    #[cfg(not(feature = "std"))]
    {
        false
    }
}

#[cfg(all(
    feature = "std",
    target_os = "linux",
    any(base64_ng_rvv_candidate, base64_ng_perf_evidence)
))]
pub(crate) fn candidate_available() -> bool {
    detect_linux_rvv_candidate()
}

fn execution_available() -> bool {
    #[cfg(base64_ng_rvv_candidate)]
    {
        candidate_available()
    }
    #[cfg(not(base64_ng_rvv_candidate))]
    {
        available()
    }
}

#[cfg(all(
    any(base64_ng_rvv_candidate, base64_ng_perf_evidence),
    not(all(feature = "std", target_os = "linux"))
))]
pub(crate) const fn candidate_available() -> bool {
    false
}

#[cfg(all(feature = "std", target_os = "linux"))]
fn detect_linux_x60_rvv() -> bool {
    const RISCV_HWPROBE_SYSCALL: isize = 258;
    const RISCV_HWPROBE_KEY_MVENDORID: i64 = 0;
    const RISCV_HWPROBE_KEY_MARCHID: i64 = 1;
    const RISCV_HWPROBE_KEY_MIMPID: i64 = 2;
    const RISCV_HWPROBE_KEY_IMA_EXT_0: i64 = 4;
    const PR_RISCV_V_GET_CONTROL: i32 = 70;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct HwProbe {
        key: i64,
        value: u64,
    }

    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
        fn syscall(number: isize, ...) -> isize;
    }

    let mut probes = [
        HwProbe {
            key: RISCV_HWPROBE_KEY_MVENDORID,
            value: 0,
        },
        HwProbe {
            key: RISCV_HWPROBE_KEY_MARCHID,
            value: 0,
        },
        HwProbe {
            key: RISCV_HWPROBE_KEY_MIMPID,
            value: 0,
        },
        HwProbe {
            key: RISCV_HWPROBE_KEY_IMA_EXT_0,
            value: 0,
        },
    ];
    // SAFETY: These are Linux RISC-V UAPI calls with the exact declared ABI.
    // The probe pointer covers the complete writable array, CPU-set size zero
    // permits a null CPU-set pointer, and every failure becomes scalar fallback.
    unsafe {
        let hwprobe_ok = syscall(
            RISCV_HWPROBE_SYSCALL,
            probes.as_mut_ptr(),
            probes.len(),
            0usize,
            core::ptr::null_mut::<usize>(),
            0u32,
        ) == 0;
        let vector_control = prctl(PR_RISCV_V_GET_CONTROL, 0usize, 0usize, 0usize, 0usize);
        exact_x60_profile_allows_rvv(
            hwprobe_ok,
            probes[0].key,
            probes[0].value,
            probes[1].key,
            probes[1].value,
            probes[2].key,
            probes[2].value,
            probes[3].key,
            probes[3].value,
            vector_control,
        )
    }
}

#[cfg(all(
    feature = "std",
    target_os = "linux",
    any(base64_ng_rvv_candidate, base64_ng_perf_evidence)
))]
fn detect_linux_rvv_candidate() -> bool {
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
#[allow(clippy::too_many_arguments)]
pub(super) const fn exact_x60_profile_allows_rvv(
    hwprobe_ok: bool,
    vendor_key: i64,
    vendor: u64,
    arch_key: i64,
    arch: u64,
    implementation_key: i64,
    implementation: u64,
    extensions_key: i64,
    extensions: u64,
    vector_control: i32,
) -> bool {
    const X60_MVENDORID: u64 = 0x710;
    const X60_MARCHID: u64 = 0x8000_0000_5800_0001;
    const X60_MIMPID: u64 = 0x1000_0000_4977_2200;
    const RISCV_HWPROBE_IMA_V: u64 = 1 << 2;
    const VSTATE_CURRENT_MASK: i32 = 3;
    const VSTATE_ON: i32 = 2;

    hwprobe_ok
        && vendor_key == 0
        && vendor == X60_MVENDORID
        && arch_key == 1
        && arch == X60_MARCHID
        && implementation_key == 2
        && implementation == X60_MIMPID
        && extensions_key == 4
        && extensions & RISCV_HWPROBE_IMA_V != 0
        && vector_control >= 0
        && vector_control & VSTATE_CURRENT_MASK == VSTATE_ON
}

#[cfg(any(
    base64_ng_rvv_candidate,
    all(feature = "std", target_os = "linux", base64_ng_perf_evidence)
))]
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
    encode_slice_with_availability::<A, PAD>(input, output, execution_available())
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
    decode_slice_with_availability::<A, PAD>(input, output, execution_available())
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

#[cfg(all(test, base64_ng_rvv_candidate))]
pub(super) fn encode_slice_unavailable_for_test<A: Alphabet, const PAD: bool>(
    input: &[u8],
    output: &mut [u8],
) -> Result<usize, EncodeError> {
    encode_slice_with_availability::<A, PAD>(input, output, false)
}

#[cfg(all(test, base64_ng_rvv_candidate))]
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
