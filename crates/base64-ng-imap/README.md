# base64-ng-imap

Bounded, `no_std`-first RFC 3501 Section 5.1.3 modified-Base64 payload
transforms for the `base64-ng` crate family.

This is deliberately **not a complete IMAP modified UTF-7 mailbox codec**.
Encoding accepts bytes that the caller has already converted to UTF-16BE;
decoding returns UTF-16BE bytes. Unicode conversion, printable-ASCII routing,
the special `&-` spelling, and surrounding `&...-` shift grammar remain the
caller's responsibility.

RFC 3501 is obsolete. This companion exists for interoperating with legacy
IMAP4rev1 deployments and does not claim current IMAP4rev2 conformance.

```rust
use base64_ng_imap::{
    ImapPayloadLimits, decode_modified_utf7_payload_into,
    encode_modified_utf7_payload_into,
};

let limits = ImapPayloadLimits::new(64, 64, 64);
let utf16be = [0x53, 0xf0, 0x53, 0x17]; // "台北" after caller conversion
let mut encoded = [0u8; 16];
let written = encode_modified_utf7_payload_into(&utf16be, &mut encoded, limits)?;
assert_eq!(&encoded[..written], b"U,BTFw");

let mut decoded = [0u8; 8];
let written = decode_modified_utf7_payload_into(b"U,BTFw", &mut decoded, limits)?;
assert_eq!(&decoded[..written], utf16be);
# Ok::<(), base64_ng_imap::ImapPayloadError>(())
```

All public transforms require explicit finite input, output, and work limits.
One-shot destination APIs validate and size before writing, so errors leave
caller-owned output unchanged. Incremental APIs are heapless and bounded, but
previously emitted ordinary bytes cannot be retracted if a later tail fails.

The payload transform is an ordinary public-data API. It is not a
constant-time secret decoder and provides no cleanup or Unicode validation.
