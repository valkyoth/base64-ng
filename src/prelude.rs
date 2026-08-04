//! Focused imports for ordinary 2.0 Base64 operations.
//!
//! This prelude intentionally excludes compatibility policies, secret owners,
//! constant-time-oriented codecs, protocol companions, and the historical 1.x
//! API. Import those boundaries explicitly so their contracts remain visible.
//!
//! # Example
//!
//! ```
//! # #[cfg(feature = "alloc")]
//! # {
//! use base64_ng::prelude::*;
//!
//! let encoded = STRICT_STANDARD_PADDED.encode_to_string(b"hello").unwrap();
//! assert_eq!(encoded, "aGVsbG8=");
//! # }
//! ```

#[cfg(feature = "alloc")]
pub use crate::Base64String;
pub use crate::{
    Base64, Codec, OneShotError, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED,
};
