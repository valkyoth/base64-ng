"""Check the pinned classifier's lane-local mask and reduction-to-bool dataflow.

This is deliberately not a general AArch64 emulator. Unknown instructions fail
closed. A 256-entry table represents each lane-local byte function; only the
reviewed full-width min/max or zero-tested pair sum may combine lanes. Scalar
pairs then track the all-valid and any-invalid cases through to the returned Boolean.
"""
import re


def check_return(body, url_safe):
    alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    alphabet += b"-_" if url_safe else b"+/"
    valid = tuple(255 if byte in alphabet else 0 for byte in range(256))
    vectors, scalars = {}, {}
    reduction = None
    returned = False
    zero64 = {}
    flags = None

    def binary(op, left, right, mask):
        functions = {
            "add": lambda a, b: a + b,
            "and": lambda a, b: a & b,
            "orr": lambda a, b: a | b,
            "eor": lambda a, b: a ^ b,
            "bic": lambda a, b: a & ~b,
            "cmeq": lambda a, b: 255 if a == b else 0,
            "cmhi": lambda a, b: 255 if a > b else 0,
        }
        return tuple(functions[op](a, b) & mask for a, b in zip(left, right))

    for raw in body.splitlines():
        line = raw.split("//", 1)[0].split(";", 1)[0].strip()
        if not line or line == ".cfi_startproc":
            continue
        if returned:
            raise ValueError("instructions after classifier return")
        # Normalize only the reviewed ELF/Mach-O 16-byte lane spellings.
        line = re.sub(r"^(\w+)\.16b\s+", r"\1 ", line)
        line = re.sub(r"\bv(\d+)\.16b\b", r"v\1", line)
        line = re.sub(r"\s+", " ", line)
        match = re.fullmatch(r"ldr q(\d+), \[x0\]", line)
        constant = re.fullmatch(r"movi v(\d+), #(\d+)", line)
        vector = re.fullmatch(r"(add|and|orr|eor|bic|cmeq|cmhi) v(\d+), v(\d+), v(\d+)", line)
        reduce = re.fullmatch(r"(uminv|umaxv) b(\d+), v(\d+)", line)
        pair_sum = re.fullmatch(r"addp(?:\.2d)? d(\d+), v(\d+)(?:\.2d)?", line)
        move = re.fullmatch(r"mov w(\d+), #(\d+)", line)
        extract = re.fullmatch(r"fmov w(\d+), s(\d+)", line)
        scalar = re.fullmatch(r"(and|orr|eor|bic) w(\d+), w(\d+), w(\d+)", line)
        extract64 = re.fullmatch(r"fmov x(\d+), d(\d+)", line)
        compare64 = re.fullmatch(r"cmp x(\d+), #0", line)
        set_equal = re.fullmatch(r"cset w(\d+), eq", line)
        try:
            if match or constant or vector:
                if reduction is not None:
                    raise ValueError("vector operation after reduction")
                if match:
                    vectors[match[1]] = tuple(range(256))
                elif constant:
                    value = int(constant[2])
                    if value > 255:
                        raise ValueError("invalid byte immediate")
                    vectors[constant[1]] = (value,) * 256
                else:
                    vectors[vector[2]] = binary(vector[1], vectors[vector[3]], vectors[vector[4]], 255)
            elif reduce:
                if reduction is not None:
                    raise ValueError("multiple reductions")
                expected = valid if reduce[1] == "uminv" else tuple(255 - v for v in valid)
                if vectors[reduce[3]] != expected:
                    raise ValueError("reduction does not express all lanes valid")
                # Scalar reduction clears the other bits of its SIMD register.
                values = (255, 0) if reduce[1] == "uminv" else (0, 255)
                reduction = (reduce[2], values, 32)
            elif pair_sum:
                if reduction is not None or vectors[pair_sum[2]] != tuple(255 - v for v in valid):
                    raise ValueError("pair sum requires exact invalid-byte masks")
                # Each 64-bit half has eight bytes in {0, 255}. Exhaustively
                # prove that no two nonzero halves can sum to zero modulo 2^64.
                words = {sum(255 << (8 * lane) for lane in range(8) if mask & (1 << lane))
                         for mask in range(256)}
                if words.intersection({(-word) % (1 << 64) for word in words}) != {0}:
                    raise ValueError("invalid-mask sum can cancel")
                reduction = (pair_sum[1], (True, False), 64)
            elif move:
                value = int(move[2])
                if value > 0xffffffff:
                    raise ValueError("invalid scalar immediate")
                scalars[move[1]] = (value, value)
                zero64.pop(move[1], None)
            elif extract:
                if reduction is None or extract[2] != reduction[0] or reduction[2] != 32:
                    raise ValueError("return does not extract the reduced register")
                scalars[extract[1]] = reduction[1]
                zero64.pop(extract[1], None)
            elif extract64:
                if reduction is None or extract64[2] != reduction[0] or reduction[2] != 64:
                    raise ValueError("return does not extract the complete pair sum")
                zero64[extract64[1]] = reduction[1]
                scalars.pop(extract64[1], None)
            elif compare64:
                flags = zero64[compare64[1]]
            elif set_equal:
                if flags is None:
                    raise ValueError("return lacks a full-width zero comparison")
                scalars[set_equal[1]] = tuple(int(value) for value in flags)
                zero64.pop(set_equal[1], None)
            elif scalar:
                scalars[scalar[2]] = binary(scalar[1], scalars[scalar[3]], scalars[scalar[4]], 0xffffffff)
                zero64.pop(scalar[2], None)
            elif line == "ret":
                if reduction is None or scalars.get("0") != (1, 0):
                    raise ValueError("return must be true for all-valid and false for any-invalid")
                returned = True
            else:
                raise ValueError(f"unreviewed classifier instruction: {line}")
        except KeyError as error:
            raise ValueError("undefined classifier register") from error
    if not returned:
        raise ValueError("missing classifier return")
