# dudect-Style Timing Evidence

`base64-ng` includes an isolated dudect-style timing harness under `dudect/`.
The harness is not included in the published crate and does not add
dependencies to the root package. It depends on the local core and subtle
companion plus the exact reviewed `subtle` version for Commit 41 equality
evidence.

The harness measures equal-public-length inputs through the 2.0 bounded
`SecretArrayFrame` and `SecretArrayEncoder`. Decode reports separate Welch
t-statistics for distinct valid contents, first-versus-last malformed
positions, malformed byte classes, and the pre-gate core without successful
declassification. Encode compares fixed and randomized classified input for
both the built-in arithmetic mapper and one custom-alphabet fixed scan. It is
also compares fixed and randomized equal-length contents through the sealed
`SubtleSecretEq` path. It is empirical evidence for review, not a formal proof
or standalone cryptographic constant-time claim.

## Compile the Harness

```sh
scripts/check_dudect.sh
```

By default this compiles the harness and checks its isolated dependency policy.
Normal CI runs this compile/dependency check. It deliberately does not run the
timing test in normal CI because timing measurements are noisy on shared
runners.

## Run Local Timing Evidence

Run the timing measurement on an idle local machine:

```sh
BASE64_NG_RUN_DUDECT=1 scripts/check_dudect.sh
```

Tune sample counts for longer local evidence:

```sh
BASE64_NG_RUN_DUDECT=1 \
BASE64_NG_DUDECT_SAMPLES=100000 \
BASE64_NG_DUDECT_ITERS=128 \
scripts/check_dudect.sh
```

The default threshold is `10`, matching the usual dudect convention that large
absolute t-statistics require investigation. A passing run means this specific
binary, on this machine, did not show a strong timing signal for the measured
class pairs. It does not prove all targets or compiler modes. Whole-call valid
versus invalid timing is deliberately not compared because successful release
performs a documented post-gate copy.

Opt-in timing runs write release evidence under:

```text
target/release-evidence/dudect/
```

Expected files:

- `dudect-output.txt`: raw harness output.
- `MANIFEST.txt`: rustc/cargo/system metadata, command line, parameters,
  status, checksum, and interpretation notes.

## Review Rules

- Keep the harness outside the published crate.
- Keep dependencies limited to the local `base64-ng` family and the exact
  reviewed `subtle` dependency admitted by `base64-ng-subtle`.
- Do not use this as a replacement for Kani, Miri, generated-code review,
  fuzzing, or scalar differential tests.
- Archive `target/release-evidence/dudect/` when using dudect evidence for a
  release note or security review.
