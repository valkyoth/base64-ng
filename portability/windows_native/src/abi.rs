use base64_ng::{DecodeValidation, STRICT_STANDARD_PADDED, StaticBackendToken};
use std::arch::{asm, x86_64::__m128i};

struct Context<'a> {
    input: &'a [u8],
    encoded: &'a [u8],
    output: &'a mut [u8],
    operation: u8,
    token: Option<&'a StaticBackendToken>,
}

type Callback = unsafe extern "win64" fn(*mut Context<'_>) -> usize;

/// # Safety
/// `raw` must be non-null, aligned, and point to an initialized, live Context
/// that is exclusively accessible for the complete call.
#[inline(never)]
unsafe extern "win64" fn invoke(raw: *mut Context<'_>) -> usize {
    // SAFETY: The caller guarantees validity and exclusivity for this call.
    let context = unsafe { &mut *raw };
    let result = match (context.operation, context.token) {
        (0, None) => STRICT_STANDARD_PADDED
            .encode_into(context.input, context.output)
            .ok(),
        (0, Some(token)) => token
            .encode_standard::<true>(context.input, context.output)
            .ok(),
        (1, None) => STRICT_STANDARD_PADDED
            .decode_into(context.encoded, context.output)
            .ok(),
        (1, Some(token)) => token
            .decode_standard::<true>(context.encoded, context.output)
            .ok(),
        (2, _) => STRICT_STANDARD_PADDED
            .decode_into_with_validation(
                context.encoded,
                context.output,
                DecodeValidation::ScalarReference,
            )
            .ok(),
        _ => None,
    };
    result.unwrap_or(usize::MAX)
}

/// # Safety
/// `callback` must accept an exclusively borrowed live Context, must not retain
/// its pointer or unwind, and must follow Win64 except that XMM6-XMM15 may be
/// clobbered (this probe explicitly declares and checks those outputs).
#[inline(never)]
unsafe fn call(context: &mut Context<'_>, callback: Callback) -> usize {
    // Distinct, nonzero low/high halves detect partial restores and swapped slots.
    let expected: [[u64; 2]; 10] = std::array::from_fn(|i| {
        [
            0x1234_5678_90ab_cdef + i as u64,
            0xfedc_ba09_8765_4321 - i as u64,
        ]
    });
    // SAFETY: Every bit pattern is valid in the same-sized SIMD integer type.
    let mut registers: [__m128i; 10] = unsafe { std::mem::transmute(expected) };
    let result;
    let raw = core::ptr::from_mut(context);
    // SAFETY: SSE2 is baseline on x86_64. Default asm stack alignment permits a
    // call; reserve Win64's 32-byte shadow space and restore rsp exactly. RCX
    // carries the exclusive live context; the caller guarantees the callback
    // accepts this pointer without additional preconditions. All volatile
    // state and all tested XMM registers are declared outputs. No unwind crosses
    // this block; callback operations return errors instead of asserting.
    unsafe {
        asm!(
            "sub rsp, 32", "call r11", "add rsp, 32",
            in("rcx") raw, in("r11") callback,
            lateout("rax") result,
            inout("xmm6") registers[0], inout("xmm7") registers[1],
            inout("xmm8") registers[2], inout("xmm9") registers[3],
            inout("xmm10") registers[4], inout("xmm11") registers[5],
            inout("xmm12") registers[6], inout("xmm13") registers[7],
            inout("xmm14") registers[8], inout("xmm15") registers[9],
            clobber_abi("win64"),
        );
    }
    // SAFETY: This reads initialized SIMD outputs as same-sized integers.
    let actual: [[u64; 2]; 10] = unsafe { std::mem::transmute(registers) };
    assert_eq!(actual, expected, "Win64 nonvolatile XMM preservation");
    result
}

pub fn exercise() {
    let token = StaticBackendToken::for_compiled_target();
    if cfg!(all(target_feature = "ssse3", target_feature = "sse4.1")) {
        assert!(token.is_some(), "compiled static backend failed admission");
    }
    for token in std::iter::once(None).chain(token.as_ref().map(Some)) {
        for size in [0, 1, 2, 3, 15, 16, 31, 32, 63, 64, 767, 768, 4097] {
            let input: Vec<u8> = (0..size).map(|i| (i * 37) as u8).collect();
            let encoded = STRICT_STANDARD_PADDED.encode_to_string(&input).unwrap();
            for operation in 0..3 {
                let expected = if operation == 0 {
                    encoded.as_bytes()
                } else {
                    &input
                };
                let mut storage = vec![0xa5; expected.len() + 18];
                let mut context = Context {
                    input: &input,
                    encoded: encoded.as_bytes(),
                    output: &mut storage[1..expected.len() + 1],
                    operation,
                    token,
                };
                // SAFETY: invoke requires only the exclusive live context
                // supplied by call(), obeys Win64 and retains no pointer.
                assert_eq!(unsafe { call(&mut context, invoke) }, expected.len());
                assert_eq!(&storage[1..expected.len() + 1], expected);
                assert_eq!(storage[0], 0xa5);
                assert!(storage[expected.len() + 1..].iter().all(|b| *b == 0xa5));
            }
        }
        for position in [0, 511, 1023] {
            let mut malformed = vec![b'A'; 1024];
            malformed[position] = b'!';
            for operation in [1, 2] {
                let mut output = [0xa5; 768];
                assert_eq!(
                    // SAFETY: invoke accepts this exclusive live context,
                    // obeys Win64 and neither retains its pointer nor unwinds.
                    unsafe {
                        call(
                            &mut Context {
                                input: &[],
                                encoded: &malformed,
                                output: &mut output,
                                operation,
                                token,
                            },
                            invoke,
                        )
                    },
                    usize::MAX
                );
                assert_eq!(output, [0xa5; 768]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn win64_callback_preserves_nonvolatile_registers() {
        exercise();
    }

    // Deliberately broken assembly is called ONLY via call(), which explicitly
    // declares every tested XMM register clobbered. No Rust caller trusts it.
    macro_rules! corrupt {
        ($name:ident, $register:literal) => {
            #[unsafe(naked)]
            unsafe extern "win64" fn $name(_: *mut Context<'_>) -> usize {
                std::arch::naked_asm!(
                    concat!("pxor ", $register, ", ", $register),
                    "xor eax, eax",
                    "ret"
                );
            }
        };
    }
    corrupt!(corrupt6, "xmm6");
    corrupt!(corrupt7, "xmm7");
    corrupt!(corrupt8, "xmm8");
    corrupt!(corrupt9, "xmm9");
    corrupt!(corrupt10, "xmm10");
    corrupt!(corrupt11, "xmm11");
    corrupt!(corrupt12, "xmm12");
    corrupt!(corrupt13, "xmm13");
    corrupt!(corrupt14, "xmm14");
    corrupt!(corrupt15, "xmm15");

    #[test]
    fn broken_callee_is_detected() {
        for callback in [
            corrupt6, corrupt7, corrupt8, corrupt9, corrupt10, corrupt11, corrupt12, corrupt13,
            corrupt14, corrupt15,
        ] {
            assert!(
                // SAFETY: These known naked callbacks ignore the context,
                // return normally, and alter only declared probe outputs.
                std::panic::catch_unwind(|| unsafe {
                    call(
                        &mut Context {
                            input: &[],
                            encoded: &[],
                            output: &mut [],
                            operation: 0,
                            token: None,
                        },
                        callback,
                    )
                })
                .is_err()
            );
        }
    }
}
