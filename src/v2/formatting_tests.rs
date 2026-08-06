use core::fmt::Write as _;

use super::{
    CodecBuilder, CountedSink, CountedWriteError, DecodePadding, EncodePadding, FormatWriteError,
    STRICT_STANDARD_PADDED, ValidatedAlphabet,
};

#[test]
fn display_and_formatter_support_builtin_and_temporary_runtime_codecs() {
    let mut built_in = StackWriter::new();
    write!(
        &mut built_in,
        "{}",
        STRICT_STANDARD_PADDED.display(b"hello").unwrap()
    )
    .unwrap();
    assert_eq!(built_in.as_bytes(), b"aGVsbG8=");

    let runtime_display = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    )
    .encode_padding(EncodePadding::Unpadded)
    .decode_padding(DecodePadding::Forbid)
    .build()
    .unwrap()
    .display(b"custom")
    .unwrap();
    let mut runtime = StackWriter::new();
    write!(&mut runtime, "{runtime_display}").unwrap();
    assert_eq!(runtime.as_bytes(), b"V3SwaD9q");

    let secret_adjacent = b"classified";
    let display = STRICT_STANDARD_PADDED.display(secret_adjacent).unwrap();
    let mut debug = StackWriter::new();
    write!(&mut debug, "{display:?}").unwrap();
    assert!(!contains(debug.as_bytes(), secret_adjacent));

    let chunks = STRICT_STANDARD_PADDED
        .encoded_chunks(secret_adjacent)
        .unwrap();
    let mut chunk_debug = StackWriter::new();
    write!(&mut chunk_debug, "{chunks:?}").unwrap();
    assert!(!contains(chunk_debug.as_bytes(), secret_adjacent));
}

#[test]
fn formatter_failure_reports_only_fully_successful_calls() {
    let mut writer = StackWriter::failing_on_call(1, true);
    let error = STRICT_STANDARD_PADDED
        .encode_to_fmt(b"foobar", &mut writer)
        .unwrap_err();
    assert_eq!(error, FormatWriteError::Formatter { confirmed: 4 });
    assert_eq!(error.confirmed(), 4);
    assert_eq!(writer.as_bytes(), b"Zm9vY");
    let mut error_text = StackWriter::new();
    write!(&mut error_text, "{error}").unwrap();
    assert_eq!(
        error_text.as_bytes(),
        b"formatter failed after 4 confirmed Base64 bytes"
    );

    let mut successful = StackWriter::new();
    assert_eq!(
        STRICT_STANDARD_PADDED
            .encode_to_fmt(b"foobar", &mut successful)
            .unwrap(),
        8
    );
    assert_eq!(successful.as_bytes(), b"Zm9vYmFy");
}

#[test]
#[cfg(feature = "std")]
fn formatter_panics_propagate_after_prior_successful_calls() {
    let mut writer = PanickingWriter::new(1);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = STRICT_STANDARD_PADDED.encode_to_fmt(b"foobar", &mut writer);
    }));
    assert!(panic.is_err());
    assert_eq!(writer.as_bytes(), b"Zm9v");
}

#[test]
fn counted_sink_reports_exact_short_write_progress_and_contract_failures() {
    let mut short = TestCountedSink::new(1);
    assert_eq!(
        STRICT_STANDARD_PADDED
            .encode_to_counted(b"foobar", &mut short)
            .unwrap(),
        8
    );
    assert_eq!(short.as_bytes(), b"Zm9vYmFy");

    let mut failing = TestCountedSink::new(2);
    failing.fail_on_call = Some(3);
    let error = STRICT_STANDARD_PADDED
        .encode_to_counted(b"foobar", &mut failing)
        .unwrap_err();
    assert!(matches!(
        error,
        CountedWriteError::Sink {
            error: SinkError,
            committed: 6
        }
    ));
    assert_eq!(error.committed(), 6);
    assert_eq!(failing.as_bytes(), b"Zm9vYm");

    let mut zero = TestCountedSink::new(0);
    assert!(matches!(
        STRICT_STANDARD_PADDED.encode_to_counted(b"f", &mut zero),
        Err(CountedWriteError::WriteZero { committed: 0 })
    ));

    let mut lying = TestCountedSink::new(8);
    lying.invalid_count = true;
    assert!(matches!(
        STRICT_STANDARD_PADDED.encode_to_counted(b"f", &mut lying),
        Err(CountedWriteError::InvalidCount {
            reported: 5,
            offered: 4,
            committed: 0
        })
    ));
}

struct StackWriter {
    bytes: [u8; 128],
    len: usize,
    calls: usize,
    fail_on_call: Option<usize>,
    partial_failure: bool,
}

impl StackWriter {
    const fn new() -> Self {
        Self {
            bytes: [0; 128],
            len: 0,
            calls: 0,
            fail_on_call: None,
            partial_failure: false,
        }
    }

    const fn failing_on_call(call: usize, partial_failure: bool) -> Self {
        Self {
            fail_on_call: Some(call),
            partial_failure,
            ..Self::new()
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl core::fmt::Write for StackWriter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let call = self.calls;
        self.calls += 1;
        if self.fail_on_call == Some(call) {
            if self.partial_failure && !text.is_empty() {
                self.bytes[self.len] = text.as_bytes()[0];
                self.len += 1;
            }
            return Err(core::fmt::Error);
        }
        let end = self.len + text.len();
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[cfg(feature = "std")]
struct PanickingWriter {
    bytes: [u8; 64],
    len: usize,
    calls: usize,
    panic_on_call: usize,
}

#[cfg(feature = "std")]
impl PanickingWriter {
    const fn new(panic_on_call: usize) -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
            calls: 0,
            panic_on_call,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[cfg(feature = "std")]
impl core::fmt::Write for PanickingWriter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let call = self.calls;
        self.calls += 1;
        assert_ne!(call, self.panic_on_call, "injected formatter panic");
        let end = self.len + text.len();
        self.bytes[self.len..end].copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
struct SinkError;

struct TestCountedSink {
    bytes: [u8; 64],
    len: usize,
    max_write: usize,
    calls: usize,
    fail_on_call: Option<usize>,
    invalid_count: bool,
}

impl TestCountedSink {
    const fn new(max_write: usize) -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
            max_write,
            calls: 0,
            fail_on_call: None,
            invalid_count: false,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl CountedSink for TestCountedSink {
    type Error = SinkError;

    fn write(&mut self, bytes: &[u8]) -> Result<usize, Self::Error> {
        let call = self.calls;
        self.calls += 1;
        if self.fail_on_call == Some(call) {
            return Err(SinkError);
        }
        if self.invalid_count {
            return Ok(bytes.len() + 1);
        }
        let written = bytes.len().min(self.max_write);
        let end = self.len + written;
        self.bytes[self.len..end].copy_from_slice(&bytes[..written]);
        self.len = end;
        Ok(written)
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|candidate| candidate == needle)
}
