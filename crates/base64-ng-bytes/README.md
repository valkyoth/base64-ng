<p align="center">
  <b>bounded bytes and Buf helpers for base64-ng.</b><br>
  Strict decoding, caller-owned buffers, optional integrations, and release-gated evidence.
</p>

<div align="center">
  <a href="https://crates.io/crates/base64-ng">base64-ng crate</a>
  |
  <a href="https://docs.rs/base64-ng-bytes">Docs.rs</a>
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

# base64-ng-bytes

Candidate source: `2.1.0`, not published. Publication metadata is prepared;
final campaigns, review, CI and the signed tag remain required by the
[2.1 commit plan](../../docs/2.1.0-release-plan.md). Dependency examples target
2.1.0 after publication; use a reviewed Git revision or local path before then.

### Optional acceleration (2.1 development)

For a consumer next to a checkout of this repository:

```toml
base64-ng-bytes = { path = "../base64-ng/crates/base64-ng-bytes", features = ["simd"] }
```

Use `checked-backend` instead of `simd` for redundant scalar checking; it
implies SIMD support but never promises that a particular call will accelerate.
Fragmented ordinary encode/decode can use the shared bulk kernels.
Defaults, grammar and cleanup contracts are unchanged. Neither feature enables
`std`, `alloc` or `secrets` by itself. Without core `std`, acceleration needs
complete static target features; otherwise the scalar fallback remains active.
See the [companion feature matrix](../../docs/COMPANION_FEATURES_2.1.md).

Fragment-preserving `bytes` integration for the `base64-ng` 2.0 codec.

This companion crate provides explicit helpers for services that already use
`bytes::Bytes`, `bytes::Buf`, and `bytes::BufMut`. Fragmented input is driven
directly through the shared incremental core and is never fully coalesced.

```rust
use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_bytes::Base64BytesExt;
use bytes::Bytes;

let encoded = STRICT_STANDARD_PADDED
    .encode_buf(Bytes::from_static(b"hello"))
    .unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");

let decoded = STRICT_STANDARD_PADDED.decode_buf(encoded).unwrap();
assert_eq!(&decoded[..], b"hello");
```

The owned helpers are transactional for caller-visible output. For bounded
peer-controlled frames, set both cumulative limits:

```rust
use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_bytes::{Base64BytesExt, BytesLimits};
use bytes::Bytes;

let encoded = STRICT_STANDARD_PADDED
    .encode_buf_with_limits(
        Bytes::from_static(b"hello"),
        BytesLimits::new(5, 8),
    )
    .unwrap();
assert_eq!(&encoded[..], b"aGVsbG8=");
```

For arbitrary `BufMut`, use `bytes_encoder()` or `bytes_decoder()`. These
states are explicitly prefix-committing and return retryable
`Status::OutputFull` while retaining pending quantum state. Ordinary decoded
plaintext committed before a malformed later fragment cannot be withdrawn;
use bounded secret APIs from the core `secrets` capability for secret-bearing
frames.

In development 2.1, `BytesDecoder::update_with_validation` accepts
`base64_ng::DecodeValidation::ScalarReference` for independent scalar
validation of newly accepted input. The default `update` uses `Auto`; both
retain the same fragment, cumulative-limit and prefix-commitment contracts.
Scalar validation can still use an admitted writer and is not constant-time.
See the [adapter checkpoint](../../docs/ADAPTER_BULK_2.1.md).

`BytesProgress::input_consumed()` records bytes accepted by the transform. A
normal error also reports exact external `Buf` cursor movement through
`BytesError::input_cursor_progress()`. If a custom `Buf` violates its safe trait
contract during `advance`, that cursor result is `Indeterminate`; discard the
cursor, failed adapter, and committed prefix instead of attempting recovery by
offset.

The complete contracts and evidence are documented in
[`docs/2.0_BYTES_INTEGRATION.md`](https://github.com/valkyoth/base64-ng/blob/main/docs/2.0_BYTES_INTEGRATION.md).
