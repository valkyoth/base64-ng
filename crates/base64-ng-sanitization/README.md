<p align="center">
  <b>locked and clear-on-drop secret decode bridge for base64-ng.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-sanitization">Docs.rs</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/PLAN.md">Roadmap</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/docs/TRUST.md">Trust Dashboard</a>
  |
  <a href="https://github.com/valkyoth/base64-ng/blob/main/SECURITY.md">Security</a>
</div>

<br>

<p align="center">
  <a href="https://github.com/valkyoth/base64-ng">
    <img src="https://raw.githubusercontent.com/valkyoth/base64-ng/main/.github/images/base64-ng.webp" alt="base64-ng Rust crate overview">
  </a>
</p>

# base64-ng-sanitization

Optional `sanitization` integration helpers for `base64-ng`.

This companion crate keeps `base64-ng` itself dependency-free while giving
applications that already use `sanitization` a direct path from
constant-time-oriented Base64 decode into clear-on-drop secret containers.

```toml
[dependencies]
base64-ng = { version = "2.0.2", default-features = false }
base64-ng-sanitization = { version = "2.0.2", default-features = false }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

assert!(secret.sanitization_verify(
    b"hello",
    "example compares public expected bytes"
));
```

Enable `alloc` for heap-backed `sanitization::SecretVec` helpers:

```toml
base64-ng-sanitization = { version = "2.0.2", features = ["alloc"] }
```

The convenience `decode_secret_vec` and `decode_secret_vec_staged` methods
derive an encoded-input ceiling from their 1 MiB decoded-output ceiling before
constant-time-oriented validation, and report allocation failure. The staged
method uses the tighter staging ceiling for this preflight. Use an explicit
protocol limit at untrusted boundaries:

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationBoundedExt;

let secret = ct::STANDARD
    .decode_secret_vec_bounded::<4096>(b"aGVsbG8=")?;
secret.with_secret(|bytes| assert_eq!(bytes, b"hello"));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Fixed-size and staged helpers reject encoded input larger than their public
destination capacity before constant-time-oriented validation. They allocate
temporary arrays on the stack and reject capacities above 1,024 bytes at
compile time. Larger values must use a bounded heap helper, caller-provided
protected storage, or the protected mapping APIs below.

For high-assurance x86_64 or AArch64 native deployments, enable locked storage
helpers. This
uses `sanitization` 2.0.3's hardened native controls, including memory locking,
strict random canaries, and strict assembly comparison, and decodes directly
into locked memory:

```toml
base64-ng-sanitization = { version = "2.0.2", features = ["high-assurance"] }
```

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, LockedSanitizationCtEqExt};

let key = ct::STANDARD
    .decode_locked_secret_bytes_checked::<5>(b"aGVsbG8=")
    .unwrap();

key.try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))?;
assert!(key.try_sanitization_verify(
    b"hello",
    "example authentication decision is public"
)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

For dynamic output on supported native targets:

```rust
use base64_ng::ct;
use base64_ng_sanitization::CtDecodeSanitizationProtectedExt;

let key = ct::STANDARD
    .decode_locked_secret_vec_checked_bounded::<64>(b"aGVsbG8=")
    .unwrap();

key.try_with_secret(|bytes| assert_eq!(bytes, b"hello"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

The built-in fixed-size and dynamic `_checked` methods establish required
memory-lock, dump, and fork controls before decoding plaintext into the
mapping. Dynamic decode uses sanitization 2.0.3's protected-capacity fill
constructor, whose closure is not invoked when a required control fails.
External implementations of `CtDecodeSanitizationExt` must implement every
locked checked/fill method explicitly; 2.0 has no post-construction
compatibility default. Non-checked methods remain available when callers apply
a deployment-specific policy to the complete report. The
`decode_locked_secret_bytes_fill` method exposes sanitization 2.0's integrity
aware fill error while the original method retains its generation-error return
type for source compatibility.

`CtDecodeSanitizationProtectedExt` preserves the upstream
`ProtectedSecretFillError` categories so operators can distinguish unavailable
protection controls from canary-integrity failures. Its bounded dynamic helper
rejects encoded input beyond the derived public ceiling before full validation,
then rejects decoded capacities above the const-generic application limit
before mapping allocation or decoder invocation.

The legacy dynamic locked-vector convenience methods use a 1 MiB decoded
ceiling and reject input beyond the corresponding encoded ceiling before full
validation. Their compatibility errors map oversized encoded input to
`Fill(DecodeError::InvalidLength)` and oversized valid decoded output to
`Length`; the detailed protected method reports either case as
`ProtectedSecretFillError::CapacityLimit`. Prefer
`decode_locked_secret_vec_checked_bounded::<MAX>` for protocol-specific limits
and explicit policy-aware error handling.

For the 2.0 codec, `SanitizationProtectedDecodeExt` accepts a classified
`SecretInput` and protects both private staging and final destination before
running the fixed-work `SecretFrame` decoder:

```rust
use base64_ng::{STRICT_STANDARD_PADDED, secret::SecretInput};
use base64_ng_sanitization::SanitizationProtectedDecodeExt;

let input = SecretInput::new(b"aGVsbG8=");
let secret = STRICT_STANDARD_PADDED
    .decode_sanitization_protected_bytes::<5>(&input)?;
secret.try_expose_secret(|bytes| assert_eq!(bytes, b"hello"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

This companion path uses two `sanitization` mappings. The core
`Base64::decode_assured` path remains the single-allocation no-copy route and
is the only route covered by the core provider's generation, quarantine, and
fallible-teardown claims.

For locked comparisons, prefer `LockedSanitizationCtEqExt`: it returns
`CanaryCorruptedError` so the application controls telemetry and termination.
The source-compatible `SanitizationCtEqExt` implementation explicitly panics
on checked-exposure integrity failure rather than hiding corruption as
`Choice::FALSE`.

The integration intentionally targets `base64_ng::ct::CtEngine`. Strict
non-CT decoders remain available in `base64-ng`, but this crate keeps the
secret-container API pointed at the constant-time-oriented decode path.

`base64-ng-sanitization` also re-exports `sanitization::ct` and adds
`SanitizationCtEqExt` for comparing decoded `SecretBytes` and `SecretVec`
values through `sanitization` 2.0.3's native `Choice` API. This gives projects
that already admit `sanitization` a dependency-free alternative to external
`subtle` integration:

```rust
use base64_ng::ct;
use base64_ng_sanitization::{CtDecodeSanitizationExt, SanitizationCtEqExt};

let secret = ct::STANDARD
    .decode_secret_bytes::<5>(b"aGVsbG8=")
    .unwrap();

let equal = secret.sanitization_ct_eq(b"hello");
assert!(equal.declassify("example authentication decision is public"));
```

For deployments that want `sanitization`'s assembly-backed comparison checks,
enable the passthrough features:

```toml
base64-ng-sanitization = { version = "2.0.2", features = ["strict-compare"] }
```

The previous companion feature name `strict-ct` remains as an alias for
`strict-compare` during migration.
