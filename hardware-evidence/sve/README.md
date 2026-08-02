# AArch64 SVE Hardware Evidence

Commit 33 includes a complete SVE candidate but does not admit it to normal
dispatch. QEMU evidence proves functional behavior only. Admission requires
accepted reports from at least two real SVE systems with different vector
lengths, followed by a separate reviewed dispatch commit.

Run `scripts/check_sve_hardware.sh` from a clean exact commit on each native
system. Record the resulting transcript checksum and the remaining benchmark,
signal/context, ABI, assembly, cleanup, and pentest reviews in a report that
matches `schema-v1.json`. Validate it with:

```text
scripts/validate-sve-hardware-evidence.py REPORT.json
```

Reports from QEMU, emulators, or virtual machines are rejected. An accepted
report remains candidate evidence and cannot set `production_admitted`.
