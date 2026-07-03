# base64-ng 1.3.1 Release Notes

`1.3.1` publishes the full `base64-ng` crate family. The main crate and
`base64-ng-tokio` carry the async writer patch; the remaining companion crates
receive synchronized package metadata so the workspace keeps one coherent
crate-family version.

## Added

- Added `base64-ng-tokio::EncoderWriter`, a manual `AsyncWrite` adapter that
  accepts raw bytes and writes Base64 to the wrapped async writer.
- Added `base64-ng-tokio::DecoderWriter`, a manual `AsyncWrite` adapter that
  accepts strict Base64 bytes and writes decoded bytes to the wrapped async
  writer.
- Added fixed internal output queues with wipe-on-discard/drop behavior for the
  writer adapters.
- Added deterministic Tokio tests for split writes, short writes, pending
  shutdown drains, malformed decoder input, unpadded decoder shutdown tails,
  inner writer errors, large-input capacity clamps, and one-byte backpressure.

## Security And Policy

- Writer adapters are explicit state machines rather than `async fn` internals,
  keeping cancellation-visible state in fixed pending/output buffers.
- Shutdown is the finalization boundary. Call `AsyncWriteExt::shutdown` before
  extracting the wrapped writer if the encoded or decoded stream must be
  complete.
- Wrapped-writer I/O errors during drain are retryable and do not latch
  `is_failed`; internal protocol or capacity violations still latch permanent
  failure.
- Panic-policy validation now scans companion crate production source under
  `crates/*/src`, not only the core crate `src/` tree.
- Added permanent pentest evidence for the `1.3.1` async writer checkpoint in
  `security/pentest/v1.3.1.md`.

## Published Crates

- `base64-ng` `1.3.1`
- `base64-ng-sanitization` `1.3.1`
- `base64-ng-derive` `1.3.1`
- `base64-ng-serde` `1.3.1`
- `base64-ng-bytes` `1.3.1`
- `base64-ng-subtle` `1.3.1`
- `base64-ng-tokio` `1.3.1`

## Verification

- `cargo test -p base64-ng-tokio --all-features`
- `cargo clippy -p base64-ng-tokio --all-targets --all-features -- -D warnings`
- `scripts/validate-panic-policy.sh`
- `scripts/checks.sh`

## Tag

- Release tag: `v1.3.1`
