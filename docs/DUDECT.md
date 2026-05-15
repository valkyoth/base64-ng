# dudect-Style Timing Evidence

`base64-ng` includes an isolated, dependency-free dudect-style timing harness
under `dudect/`. The harness is not included in the published crate and does
not add dependencies to the root package.

The harness measures fixed-vs-random valid Base64 inputs of the same public
length through `ct::STANDARD_NO_PAD.decode_slice()` and reports a Welch
t-statistic. It is empirical evidence for review, not a formal proof and not a
standalone cryptographic constant-time claim.

## Compile the Harness

```sh
scripts/check_dudect.sh
```

By default this compiles the harness and checks its isolated dependency policy.
It deliberately does not run the timing test in normal CI because timing
measurements are noisy on shared runners.

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
binary, on this machine, did not show a strong fixed-vs-random timing signal
for the measured path. It does not prove all targets or compiler modes.

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
- Keep the harness dependency-free except for the local `base64-ng` path
  dependency.
- Do not use this as a replacement for Kani, Miri, generated-code review,
  fuzzing, or scalar differential tests.
- Archive `target/release-evidence/dudect/` when using dudect evidence for a
  release note or security review.
