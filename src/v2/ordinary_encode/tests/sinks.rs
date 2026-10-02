use super::*;
#[cfg(feature = "alloc")]
use crate::OneShotError;
use crate::{CountedSink, CountedWriteError, FormatWriteError};

#[cfg(test)]
#[test]
fn buffered_sink_failures_preserve_four_byte_calls_and_exact_progress() {
    struct Sink {
        bytes: std::vec::Vec<u8>,
        limit: usize,
    }
    impl core::fmt::Write for Sink {
        fn write_str(&mut self, text: &str) -> core::fmt::Result {
            assert!(text.len() <= 4);
            if self.bytes.len() + text.len() > self.limit {
                return Err(core::fmt::Error);
            }
            self.bytes.extend_from_slice(text.as_bytes());
            Ok(())
        }
    }
    impl CountedSink for Sink {
        type Error = ();
        fn write(&mut self, bytes: &[u8]) -> Result<usize, ()> {
            assert!(bytes.len() <= 4);
            if self.bytes.len() == self.limit {
                return Err(());
            }
            let len = bytes.len().min(3).min(self.limit - self.bytes.len());
            self.bytes.extend_from_slice(&bytes[..len]);
            Ok(len)
        }
    }
    let _ = ready();
    let input = [0xfbu8; 1537];
    let codec = crate::STRICT_URL_SAFE_UNPADDED;
    let expected = oracle::encode(Profile::UrlSafeUnpadded, &input);
    for limit in [0, 4, 1020, 1024, 1027, 1028, 2048, expected.len()] {
        let mut sink = Sink {
            bytes: std::vec::Vec::new(),
            limit,
        };
        let result = codec.encode_to_counted(&input, &mut sink);
        if limit == expected.len() {
            assert_eq!(result.unwrap(), limit);
        } else {
            assert!(
                matches!(result, Err(CountedWriteError::Sink { committed, .. }) if committed == limit)
            );
        }
        assert_eq!(sink.bytes, expected[..limit]);
        sink.bytes.clear();
        let result = codec.encode_to_fmt(&input, &mut sink);
        let confirmed = if limit == expected.len() {
            limit
        } else {
            limit / 4 * 4
        };
        if limit == expected.len() {
            assert_eq!(result.unwrap(), limit);
        } else {
            assert_eq!(result, Err(FormatWriteError::Formatter { confirmed }));
        }
        assert_eq!(sink.bytes, expected[..confirmed]);
    }
}

#[cfg(feature = "alloc")]
#[test]
fn append_rolls_back_after_buffer_boundary_on_error_and_unwind() {
    let input = [0xfbu8; 1537];
    let codec = crate::STRICT_URL_SAFE_UNPADDED;
    for panic in [false, true] {
        // WASI's test runtime aborts on panic rather than supporting unwinding.
        if panic && cfg!(target_arch = "wasm32") {
            continue;
        }
        let mut output = std::string::String::from("prefix:");
        let mut calls = 0;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            codec.encode_append_with_hooks(
                &input,
                &mut output,
                |out, required| {
                    out.reserve_exact(required);
                    Ok(())
                },
                |_, len| {
                    assert_eq!(len, 1024);
                    calls += 1;
                    if calls == 2 {
                        assert!(!panic, "injected append unwind after a full batch");
                        return Err(OneShotError::Backend(BackendFault::ImpossibleState));
                    }
                    Ok(())
                },
            )
        }));
        if panic {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert_eq!(calls, 2);
        assert_eq!(output, "prefix:");
    }
}
