# RISC-V Hardware Evidence

Commit 32 includes a complete RVV 1.0 candidate, but does not admit it to
normal dispatch. QEMU evidence proves functional behavior only. Production
admission requires a report from real RISC-V vector hardware that validates
against `schema-v1.json`.

Generate the native transcript with:

```text
scripts/check_riscv_hardware.sh
```

Submit the report, transcript SHA-256, generated assembly, benchmark source
data, and pentest report together. Reports from QEMU, virtual machines, or
emulators are rejected. Accepted Commit 32 reports still describe a candidate,
not an admitted production backend; a later reviewed commit must consume the
evidence and explicitly change dispatch.
