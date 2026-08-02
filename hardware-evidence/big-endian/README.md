# Big-Endian Hardware Evidence

This directory defines the submission contract for real big-endian hardware.
QEMU output does not belong here and cannot satisfy this contract.

1. Check out the exact 40-character `source_commit` on the real target.
2. Run `scripts/check_big_endian_hardware.sh` and retain its complete output.
3. Record the output SHA-256 and all fields required by `schema-v1.json`.
4. Validate the report with
   `scripts/validate-big-endian-hardware-evidence.py REPORT.json`.
5. Submit the report, raw output, and external pentest reference for review.

Passing schema validation proves only that the report is structurally
complete. Maintainers must verify provenance and results before linking it as
hardware evidence. An accelerated result additionally requires the separate
backend admission, assembly, cleanup, timing, and performance review named in
`docs/BIG_ENDIAN_QEMU_REVIEW.md`.
