//! Isolated RVV 1.0 assembly leaves.

core::arch::global_asm!(
    r#"
    .option push
    .option arch, +a, +v

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
