//! Non-admitted SVE encode/decode candidate.
//!
//! Stable Rust 1.97.1 recognizes the `sve` target feature but does not expose
//! a stable per-function SVE intrinsic API. This module therefore isolates the
//! candidate in leaf `global_asm!` functions. It is compiled only by
//! project-owned evidence through the internal `base64_ng_sve_candidate` cfg.
//! Normal crate builds do not compile or dispatch this code.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "the SVE candidate is compiled for codegen evidence before production dispatch admission"
    )
)]

use crate::{Alphabet, DecodeError, EncodeError, Standard, checked_encoded_len, scalar};

const ENCODE_INPUT_BLOCK: usize = 12;
const ENCODE_OUTPUT_BLOCK: usize = 16;
const DECODE_INPUT_BLOCK: usize = 16;
const DECODE_OUTPUT_BLOCK: usize = 12;

core::arch::global_asm!(
    r#"
    .arch armv8-a+sve
    .text

    .macro base64_ng_sve_clear
        pfalse p0.b
        pfalse p1.b
        dup z0.b, #0
        dup z1.b, #0
        dup z2.b, #0
        dup z3.b, #0
        dup z4.b, #0
        dup z5.b, #0
        dup z6.b, #0
        dup z7.b, #0
    .endm

    .macro base64_ng_sve_encode_map value, ascii62, ascii63
        mov z7.d, \value\().d
        add \value\().b, \value\().b, #65
        cmphs p1.b, p0/z, z7.b, #26
        mov z0.d, z7.d
        add z0.b, z0.b, #71
        sel \value\().b, p1, z0.b, \value\().b
        cmphs p1.b, p0/z, z7.b, #52
        mov z0.d, z7.d
        sub z0.b, z0.b, #4
        sel \value\().b, p1, z0.b, \value\().b
        mov z0.d, z7.d
        sub z0.b, z0.b, #48
        cmpeq p1.b, p0/z, z0.b, #14
        dup z0.b, #\ascii62
        sel \value\().b, p1, z0.b, \value\().b
        mov z0.d, z7.d
        sub z0.b, z0.b, #48
        cmpeq p1.b, p0/z, z0.b, #15
        dup z0.b, #\ascii63
        sel \value\().b, p1, z0.b, \value\().b
    .endm

    .macro base64_ng_sve_encode name, ascii62, ascii63
        .p2align 2
        .global \name
        .hidden \name
        .type \name, %function
    \name:
        .cfi_startproc
        ptrue p0.b, vl4
        ld3b { z0.b - z2.b }, p0/z, [x0]

        lsr z3.b, z0.b, #2
        mov z4.d, z0.d
        and z4.b, z4.b, #0x3
        lsl z4.b, z4.b, #4
        lsr z7.b, z1.b, #4
        orr z4.d, z4.d, z7.d
        mov z5.d, z1.d
        and z5.b, z5.b, #0xf
        lsl z5.b, z5.b, #2
        lsr z7.b, z2.b, #6
        orr z5.d, z5.d, z7.d
        mov z6.d, z2.d
        and z6.b, z6.b, #0x3f

        base64_ng_sve_encode_map z3, \ascii62, \ascii63
        base64_ng_sve_encode_map z4, \ascii62, \ascii63
        base64_ng_sve_encode_map z5, \ascii62, \ascii63
        base64_ng_sve_encode_map z6, \ascii62, \ascii63
        st4b { z3.b - z6.b }, p0, [x1]

        base64_ng_sve_clear
        ret
        .cfi_endproc
        .size \name, .-\name
    .endm

    .macro base64_ng_sve_decode_map ascii, value, scratch, special62, special63off
        mov \value\().d, \ascii\().d
        sub \value\().b, \value\().b, #65
        cmphs p1.b, p0/z, \ascii\().b, #97
        mov \scratch\().d, \ascii\().d
        sub \scratch\().b, \scratch\().b, #71
        sel \value\().b, p1, \scratch\().b, \value\().b
        cmplo p1.b, p0/z, \ascii\().b, #58
        mov \scratch\().d, \ascii\().d
        add \scratch\().b, \scratch\().b, #4
        sel \value\().b, p1, \scratch\().b, \value\().b
        mov \scratch\().d, \ascii\().d
        sub \scratch\().b, \scratch\().b, #48
        cmpeq p1.b, p0/z, \scratch\().b, #\special62
        dup \scratch\().b, #62
        sel \value\().b, p1, \scratch\().b, \value\().b
        mov \scratch\().d, \ascii\().d
        sub \scratch\().b, \scratch\().b, #\special63off
        cmpeq p1.b, p0/z, \scratch\().b, #-1
        dup \scratch\().b, #63
        sel \value\().b, p1, \scratch\().b, \value\().b
    .endm

    .macro base64_ng_sve_decode name, special62, special63off
        .p2align 2
        .global \name
        .hidden \name
        .type \name, %function
    \name:
        .cfi_startproc
        ptrue p0.b, vl4
        ld4b { z0.b - z3.b }, p0/z, [x0]

        base64_ng_sve_decode_map z3, z7, z4, \special62, \special63off
        base64_ng_sve_decode_map z2, z6, z3, \special62, \special63off
        base64_ng_sve_decode_map z1, z5, z2, \special62, \special63off
        base64_ng_sve_decode_map z0, z4, z1, \special62, \special63off

        lsl z0.b, z4.b, #2
        lsr z3.b, z5.b, #4
        orr z0.d, z0.d, z3.d
        lsl z1.b, z5.b, #4
        lsr z3.b, z6.b, #2
        orr z1.d, z1.d, z3.d
        lsl z2.b, z6.b, #6
        orr z2.d, z2.d, z7.d
        st3b { z0.b - z2.b }, p0, [x1]

        base64_ng_sve_clear
        ret
        .cfi_endproc
        .size \name, .-\name
    .endm

    base64_ng_sve_encode base64_ng_sve_encode_standard_12, 43, 47
    base64_ng_sve_encode base64_ng_sve_encode_url_safe_12, 45, 95
    base64_ng_sve_decode base64_ng_sve_decode_standard_16, -5, 48
    base64_ng_sve_decode base64_ng_sve_decode_url_safe_16, -3, 96

    .p2align 2
    .global base64_ng_sve_vector_length
    .hidden base64_ng_sve_vector_length
    .type base64_ng_sve_vector_length, %function
base64_ng_sve_vector_length:
    .cfi_startproc
    cntb x0
    ret
    .cfi_endproc
    .size base64_ng_sve_vector_length, .-base64_ng_sve_vector_length
    "#,
    options(raw)
);

unsafe extern "C" {
    fn base64_ng_sve_encode_standard_12(input: *const u8, output: *mut u8);
    fn base64_ng_sve_encode_url_safe_12(input: *const u8, output: *mut u8);
    fn base64_ng_sve_decode_standard_16(input: *const u8, output: *mut u8);
    fn base64_ng_sve_decode_url_safe_16(input: *const u8, output: *mut u8);
    fn base64_ng_sve_vector_length() -> usize;
}

#[cfg(test)]
pub(super) fn instruction_vector_length_bytes() -> usize {
    // SAFETY: Candidate tests call this only after the Linux SVE capability
    // and per-thread vector-length query succeeds.
    unsafe { base64_ng_sve_vector_length() }
}

pub(crate) fn available() -> bool {
    vector_length_bytes().is_some()
}

pub(super) fn vector_length_bytes() -> Option<usize> {
    #[cfg(all(feature = "std", any(target_os = "linux", target_os = "android")))]
    {
        detect_linux_sve()
    }
    #[cfg(all(feature = "std", not(any(target_os = "linux", target_os = "android"))))]
    {
        None
    }
    #[cfg(not(feature = "std"))]
    {
        if cfg!(target_feature = "sve") {
            // SAFETY: Compile-time target-feature evidence proves SVE.
            Some(unsafe { base64_ng_sve_vector_length() })
        } else {
            None
        }
    }
}

#[cfg(all(feature = "std", any(target_os = "linux", target_os = "android")))]
fn detect_linux_sve() -> Option<usize> {
    const AT_HWCAP: usize = 16;
    const HWCAP_SVE: usize = 1 << 22;
    const PR_SVE_GET_VL: i32 = 51;

    unsafe extern "C" {
        fn getauxval(kind: usize) -> usize;
        fn prctl(option: i32, ...) -> i32;
    }

    // SAFETY: These are Linux AArch64 UAPI calls with the exact declared ABI.
    // They do not dereference caller pointers, and every failure is treated as
    // unavailable rather than as permission to execute SVE.
    unsafe {
        let hwcap_has_sve = getauxval(AT_HWCAP) & HWCAP_SVE != 0;
        let vector_control = prctl(PR_SVE_GET_VL, 0usize, 0usize, 0usize, 0usize);
        probe_vector_length(hwcap_has_sve, vector_control)
    }
}

#[cfg(any(
    test,
    all(feature = "std", any(target_os = "linux", target_os = "android"))
))]
pub(super) fn probe_vector_length(hwcap_has_sve: bool, vector_control: i32) -> Option<usize> {
    const SVE_VL_LEN_MASK: i32 = 0xffff;
    const MINIMUM_SVE_BYTES: usize = 16;
    const MAXIMUM_SVE_BYTES: usize = 256;

    if !hwcap_has_sve || vector_control < 0 {
        return None;
    }
    let Ok(length) = usize::try_from(vector_control & SVE_VL_LEN_MASK) else {
        return None;
    };
    if !(MINIMUM_SVE_BYTES..=MAXIMUM_SVE_BYTES).contains(&length)
        || length & (MINIMUM_SVE_BYTES - 1) != 0
    {
        return None;
    }
    Some(length)
}

#[cfg(all(test, feature = "std", target_os = "linux"))]
pub(super) fn set_vector_length_for_test(length: usize) -> Option<usize> {
    const PR_SVE_SET_VL: i32 = 50;

    unsafe extern "C" {
        fn prctl(option: i32, ...) -> i32;
    }

    let requested = i32::try_from(length).ok()?;
    // SAFETY: `PR_SVE_SET_VL` consumes an integer value and no pointers. A
    // failed or adjusted request is returned as an unavailable/actual length.
    let result = unsafe { prctl(PR_SVE_SET_VL, requested, 0usize, 0usize, 0usize) };
    probe_vector_length(true, result)
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
    if input.len() < ENCODE_INPUT_BLOCK || !supports_alphabet::<A>() || !available() {
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
        // SAFETY: The loop proves exact fixed input/output block bounds and
        // the per-call candidate gate proves SVE on this thread.
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
    if input.len() < DECODE_INPUT_BLOCK || !supports_alphabet::<A>() || !available() {
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
        // per-call candidate gate proves SVE on this thread.
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
        // SAFETY: The caller owns the fixed block bounds and SVE contract.
        unsafe { base64_ng_sve_encode_url_safe_12(input, output) };
    } else {
        // SAFETY: The caller owns the fixed block bounds and SVE contract.
        unsafe { base64_ng_sve_encode_standard_12(input, output) };
    }
}

unsafe fn decode_block<A: Alphabet>(input: *const u8, output: *mut u8) {
    if A::ENCODE[62] == b'-' {
        // SAFETY: The caller owns the fixed block bounds and SVE contract.
        unsafe { base64_ng_sve_decode_url_safe_16(input, output) };
    } else {
        // SAFETY: The caller owns the fixed block bounds and SVE contract.
        unsafe { base64_ng_sve_decode_standard_16(input, output) };
    }
}
