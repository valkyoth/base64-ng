#!/usr/bin/env python3
from rvv_validation_codegen import check, expected


fixture = "\n".join(
    f"base64_ng_rvv_validate_{name}:\n" + "\n".join(expected(a, b))
    + f"\n.size base64_ng_rvv_validate_{name},.-base64_ng_rvv_validate_{name}\n"
    for name, a, b in [("standard", 43, 47), ("url_safe", 45, 95)]
)
check(fixture)
for old, new in [
    ("vcpop.m a4,v3", "vcpop.m a4,v4"),
    ("bne a4,a3,", "beq a4,a3,"),
    ("bne a4,a3,", "beqz a4,"),
    ("vsetvli a3,a1,", "vsetvli a3,zero,"),
    ("vmsleu.vi v4,v2,9", "vmsleu.vi v4,v2,10"),
    ("li t4,43", "li t4,45"),
    ("li t5,95", "li t5,47"),
    ("li a0,0", "li a0,1"),
    ("vmv.v.i v15,0", ""),
    ("sub a1,a1,a3", "sub a1,a1,a4"),
    ("vle8.v v1,(a0)", "vle8.v v1,(a0)\nvse8.v v1,(a0)"),
    ("ret", "call unexpected\nret"),
    ("j .Lbase64_ng_rvv_validate_return", "j .Lbase64_ng_rvv_valid"),
]:
    assert old in fixture
    try:
        check(fixture.replace(old, new, 1))
    except ValueError:
        pass
    else:
        raise AssertionError(f"accepted mutation: {old} -> {new}")
print("RVV validation codegen: mutation tests ok")
