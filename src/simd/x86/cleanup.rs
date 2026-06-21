#![allow(unsafe_code)]

pub(super) unsafe fn clear_ymm_registers_after_encode_block() {
    // SAFETY: The helper runs after the AVX2 block encoder stores its output.
    // The XMM cleanup zeroes the lower halves declared to the compiler, and
    // `vzeroupper` clears upper YMM state before returning to scalar code.
    unsafe {
        clear_xmm_registers_after_encode_block();
        core::arch::asm!("vzeroupper", options(nostack, preserves_flags, nomem));
    }
}

#[cfg(target_arch = "x86")]
pub(super) unsafe fn clear_zmm_registers_after_encode_block() {
    // SAFETY: This cleanup runs after the AVX-512 block encoder stores its
    // output. The explicit outputs tell the compiler these ZMM registers are
    // clobbered while the assembly clears them; `vzeroupper` clears upper
    // vector state before returning to scalar code.
    unsafe {
        core::arch::asm!(
            "vpxord zmm0, zmm0, zmm0",
            "vpxord zmm1, zmm1, zmm1",
            "vpxord zmm2, zmm2, zmm2",
            "vpxord zmm3, zmm3, zmm3",
            "vpxord zmm4, zmm4, zmm4",
            "vpxord zmm5, zmm5, zmm5",
            "vpxord zmm6, zmm6, zmm6",
            "vpxord zmm7, zmm7, zmm7",
            "vzeroupper",
            out("zmm0") _,
            out("zmm1") _,
            out("zmm2") _,
            out("zmm3") _,
            out("zmm4") _,
            out("zmm5") _,
            out("zmm6") _,
            out("zmm7") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
pub(super) unsafe fn clear_zmm_registers_after_encode_block() {
    // SAFETY: This cleanup runs after the AVX-512 block encoder stores its
    // output. The explicit outputs tell the compiler these ZMM registers are
    // clobbered while the assembly clears them; `vzeroupper` clears upper
    // vector state before returning to scalar code.
    unsafe {
        core::arch::asm!(
            "vpxord zmm0, zmm0, zmm0",
            "vpxord zmm1, zmm1, zmm1",
            "vpxord zmm2, zmm2, zmm2",
            "vpxord zmm3, zmm3, zmm3",
            "vpxord zmm4, zmm4, zmm4",
            "vpxord zmm5, zmm5, zmm5",
            "vpxord zmm6, zmm6, zmm6",
            "vpxord zmm7, zmm7, zmm7",
            "vpxord zmm8, zmm8, zmm8",
            "vpxord zmm9, zmm9, zmm9",
            "vpxord zmm10, zmm10, zmm10",
            "vpxord zmm11, zmm11, zmm11",
            "vpxord zmm12, zmm12, zmm12",
            "vpxord zmm13, zmm13, zmm13",
            "vpxord zmm14, zmm14, zmm14",
            "vpxord zmm15, zmm15, zmm15",
            "vpxord zmm16, zmm16, zmm16",
            "vpxord zmm17, zmm17, zmm17",
            "vpxord zmm18, zmm18, zmm18",
            "vpxord zmm19, zmm19, zmm19",
            "vpxord zmm20, zmm20, zmm20",
            "vpxord zmm21, zmm21, zmm21",
            "vpxord zmm22, zmm22, zmm22",
            "vpxord zmm23, zmm23, zmm23",
            "vpxord zmm24, zmm24, zmm24",
            "vpxord zmm25, zmm25, zmm25",
            "vpxord zmm26, zmm26, zmm26",
            "vpxord zmm27, zmm27, zmm27",
            "vpxord zmm28, zmm28, zmm28",
            "vpxord zmm29, zmm29, zmm29",
            "vpxord zmm30, zmm30, zmm30",
            "vpxord zmm31, zmm31, zmm31",
            "vzeroupper",
            out("zmm0") _,
            out("zmm1") _,
            out("zmm2") _,
            out("zmm3") _,
            out("zmm4") _,
            out("zmm5") _,
            out("zmm6") _,
            out("zmm7") _,
            out("zmm8") _,
            out("zmm9") _,
            out("zmm10") _,
            out("zmm11") _,
            out("zmm12") _,
            out("zmm13") _,
            out("zmm14") _,
            out("zmm15") _,
            out("zmm16") _,
            out("zmm17") _,
            out("zmm18") _,
            out("zmm19") _,
            out("zmm20") _,
            out("zmm21") _,
            out("zmm22") _,
            out("zmm23") _,
            out("zmm24") _,
            out("zmm25") _,
            out("zmm26") _,
            out("zmm27") _,
            out("zmm28") _,
            out("zmm29") _,
            out("zmm30") _,
            out("zmm31") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86")]
pub(super) unsafe fn clear_xmm_registers_after_encode_block() {
    // SAFETY: This cleanup runs after the block encoder stores its output and
    // before returning to scalar code. The explicit outputs tell the compiler
    // these XMM registers are clobbered while assembly clears them.
    unsafe {
        core::arch::asm!(
            "pxor xmm0, xmm0",
            "pxor xmm1, xmm1",
            "pxor xmm2, xmm2",
            "pxor xmm3, xmm3",
            "pxor xmm4, xmm4",
            "pxor xmm5, xmm5",
            "pxor xmm6, xmm6",
            "pxor xmm7, xmm7",
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm6") _,
            out("xmm7") _,
            options(nostack, preserves_flags)
        );
    }
}

#[cfg(target_arch = "x86_64")]
pub(super) unsafe fn clear_xmm_registers_after_encode_block() {
    // SAFETY: This cleanup runs after the block encoder stores its output and
    // before returning to scalar code. The explicit outputs tell the compiler
    // these XMM registers are clobbered while assembly clears them.
    unsafe {
        core::arch::asm!(
            "pxor xmm0, xmm0",
            "pxor xmm1, xmm1",
            "pxor xmm2, xmm2",
            "pxor xmm3, xmm3",
            "pxor xmm4, xmm4",
            "pxor xmm5, xmm5",
            "pxor xmm6, xmm6",
            "pxor xmm7, xmm7",
            "pxor xmm8, xmm8",
            "pxor xmm9, xmm9",
            "pxor xmm10, xmm10",
            "pxor xmm11, xmm11",
            "pxor xmm12, xmm12",
            "pxor xmm13, xmm13",
            "pxor xmm14, xmm14",
            "pxor xmm15, xmm15",
            out("xmm0") _,
            out("xmm1") _,
            out("xmm2") _,
            out("xmm3") _,
            out("xmm4") _,
            out("xmm5") _,
            out("xmm6") _,
            out("xmm7") _,
            out("xmm8") _,
            out("xmm9") _,
            out("xmm10") _,
            out("xmm11") _,
            out("xmm12") _,
            out("xmm13") _,
            out("xmm14") _,
            out("xmm15") _,
            options(nostack, preserves_flags)
        );
    }
}
