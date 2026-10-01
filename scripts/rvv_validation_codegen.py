"""Fail-closed inventory of the two hand-written RVV classifier leaves.

Unlike compiler-lowered intrinsics, these leaves have a fixed reviewed program.
Compare its complete instructions and branch labels, not selected mnemonics.
Execution tests independently check its alphabet and VL semantics.
"""
import re


def expected(ascii62, ascii63):
    body = f"""li t1,65
li t2,97
li t3,48
li t4,{ascii62}
li t5,{ascii63}
li t6,26
beqz a1,.Lbase64_ng_rvv_valid
.Lbase64_ng_rvv_validate_loop:
vsetvli a3,a1,e8,m1,ta,ma
vle8.v v1,(a0)
vsub.vx v2,v1,t1
vmsltu.vx v3,v2,t6
vsub.vx v2,v1,t2
vmsltu.vx v4,v2,t6
vmor.mm v3,v3,v4
vsub.vx v2,v1,t3
vmsleu.vi v4,v2,9
vmor.mm v3,v3,v4
vmseq.vx v4,v1,t4
vmor.mm v3,v3,v4
vmseq.vx v4,v1,t5
vmor.mm v3,v3,v4
vcpop.m a4,v3
bne a4,a3,.Lbase64_ng_rvv_invalid
add a0,a0,a3
sub a1,a1,a3
bnez a1,.Lbase64_ng_rvv_validate_loop
.Lbase64_ng_rvv_valid:
li a0,1
j .Lbase64_ng_rvv_validate_return
.Lbase64_ng_rvv_invalid:
li a0,0
.Lbase64_ng_rvv_validate_return:
li t0,-1
vsetvli zero,t0,e8,m1,ta,ma
"""
    return body.strip().splitlines() + [f"vmv.v.i v{i},0" for i in range(16)] + ["ret"]


def check(text):
    for family, a62, a63 in [("standard", 43, 47), ("url_safe", 45, 95)]:
        name = f"base64_ng_rvv_validate_{family}"
        matches = re.findall(rf"^{name}:\s*\n(.*?)^\s*\.size\s+{name},", text, re.M | re.S)
        if len(matches) != 1:
            raise ValueError(f"expected exactly one production {name}")
        lines = []
        for raw in matches[0].splitlines():
            line = raw.split("#", 1)[0].strip()
            if not line or line.startswith(".cfi_") or re.fullmatch(r"\.Ltmp\d+:", line):
                continue
            line = re.sub(r"\s+", " ", line)
            line = re.sub(r"\s*,\s*", ",", line)
            line = re.sub(r"(\.Lbase64_ng_rvv_[a-z_]+)_\d+", r"\1", line)
            lines.append(line)
        if lines != expected(a62, a63):
            raise ValueError(f"unreviewed classifier instructions/control flow: {name}")
