#![allow(missing_docs)]
#![allow(unsafe_code)]

use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
};

use base64_ng::{
    Failure, OperationError, STRICT_STANDARD_PADDED, STRICT_URL_SAFE_UNPADDED, Status,
};
use base64_ng_bytes::{
    Base64BytesExt, BytesErrorKind, BytesLimits, BytesProgress, InputCursorProgress,
};
use bytes::{Buf, BufMut, Bytes, buf::UninitSlice};

#[test]
fn transactional_owned_results_accept_one_byte_fragments() {
    assert!(
        STRICT_STANDARD_PADDED
            .encode_buf(Bytes::new())
            .unwrap()
            .is_empty()
    );
    assert!(
        STRICT_STANDARD_PADDED
            .decode_buf(Bytes::new())
            .unwrap()
            .is_empty()
    );

    let encoded = STRICT_STANDARD_PADDED
        .encode_buf(OneByteBuf::new(b"fragmented bytes input"))
        .unwrap();
    assert_eq!(&encoded[..], b"ZnJhZ21lbnRlZCBieXRlcyBpbnB1dA==");

    let decoded = STRICT_STANDARD_PADDED
        .decode_buf(OneByteBuf::new(&encoded))
        .unwrap();
    assert_eq!(&decoded[..], b"fragmented bytes input");

    let encoded = STRICT_URL_SAFE_UNPADDED
        .encode_buf(OneByteBuf::new(b"\xfb\xff"))
        .unwrap();
    assert_eq!(&encoded[..], b"-_8");
    let decoded = STRICT_URL_SAFE_UNPADDED
        .decode_buf(OneByteBuf::new(&encoded))
        .unwrap();
    assert_eq!(&decoded[..], b"\xfb\xff");
}

#[test]
fn encoder_resumes_after_every_one_byte_destination() {
    let original = b"one byte output fragments";
    let mut input = OneByteBuf::new(original);
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();
    let mut output = Vec::new();
    let mut consumed = 0;

    while input.has_remaining() {
        let mut byte = [0u8; 1];
        let mut destination = &mut byte[..];
        let step = encoder.update(&mut input, &mut destination).unwrap();
        consumed += step.progress().input_consumed();
        output.extend_from_slice(&byte[..step.progress().output_committed()]);
        assert!(matches!(
            step.status(),
            Status::NeedInput | Status::OutputFull(_)
        ));
    }
    loop {
        let mut byte = [0u8; 1];
        let mut destination = &mut byte[..];
        let step = encoder.finish(&mut destination).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_committed()]);
        if matches!(step.status(), Status::Complete) {
            break;
        }
        assert!(matches!(step.status(), Status::OutputFull(_)));
    }

    assert_eq!(consumed, original.len());
    assert_eq!(encoder.source_position(), original.len());
    assert_eq!(encoder.output_committed(), output.len());
    assert_eq!(output, b"b25lIGJ5dGUgb3V0cHV0IGZyYWdtZW50cw==");
}

#[test]
fn decoder_resumes_after_fragmented_input_and_output() {
    let encoded = b"b25lIGJ5dGUgb3V0cHV0IGZyYWdtZW50cw==";
    let mut input = OneByteBuf::new(encoded);
    let mut decoder = STRICT_STANDARD_PADDED.bytes_decoder();
    let mut output = Vec::new();

    while input.has_remaining() {
        let mut byte = [0u8; 1];
        let mut destination = &mut byte[..];
        let step = decoder.update(&mut input, &mut destination).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_committed()]);
    }
    loop {
        let mut byte = [0u8; 1];
        let mut destination = &mut byte[..];
        let step = decoder.finish(&mut destination).unwrap();
        output.extend_from_slice(&byte[..step.progress().output_committed()]);
        if matches!(step.status(), Status::Complete) {
            break;
        }
    }

    assert_eq!(output, b"one byte output fragments");
}

#[test]
fn malformed_suffix_reports_only_previously_committed_prefix() {
    let mut input = Bytes::from_static(b"aGVsbG8=").chain(Bytes::from_static(b"!AAA"));
    let mut output = Vec::new();
    let mut decoder = STRICT_STANDARD_PADDED.bytes_decoder();
    let error = decoder.update(&mut input, &mut output).unwrap_err();

    assert_eq!(error.progress().input_consumed(), 8);
    assert_eq!(error.progress().output_committed(), 5);
    assert_eq!(error.input_cursor_progress(), InputCursorProgress::Exact(8));
    assert_eq!(output, b"hello");
    assert!(matches!(
        error.kind(),
        BytesErrorKind::Operation(OperationError::Failed(Failure::Input(_)))
    ));
    assert!(decoder.is_failed());
    assert!(matches!(
        decoder
            .update(&mut Bytes::new(), &mut Vec::new())
            .unwrap_err()
            .kind(),
        BytesErrorKind::FailedState
    ));
}

#[test]
fn transactional_decode_returns_no_partial_plaintext() {
    let error = STRICT_STANDARD_PADDED
        .decode_buf(Bytes::from_static(b"aGVsbG8=").chain(Bytes::from_static(b"!AAA")))
        .unwrap_err();
    assert_eq!(error.progress().output_committed(), 5);
    assert!(matches!(error.kind(), BytesErrorKind::Operation(_)));
}

#[test]
fn cumulative_input_and_output_limits_fail_closed() {
    let input_error = STRICT_STANDARD_PADDED
        .encode_buf_with_limits(
            Bytes::from_static(b"hello"),
            BytesLimits::new(4, usize::MAX),
        )
        .unwrap_err();
    assert_eq!(
        input_error.kind(),
        BytesErrorKind::InputLimitExceeded {
            required: 5,
            limit: 4,
        }
    );

    let output_error = STRICT_STANDARD_PADDED
        .encode_buf_with_limits(Bytes::from_static(b"hello"), BytesLimits::new(5, 7))
        .unwrap_err();
    assert_eq!(
        output_error.kind(),
        BytesErrorKind::OutputLimitExceeded { limit: 7 }
    );

    let mut decoder = STRICT_STANDARD_PADDED.bytes_decoder_with_limits(BytesLimits::new(8, 4));
    let mut input = Bytes::from_static(b"aGVsbG8=");
    let error = decoder.update(&mut input, &mut Vec::new()).unwrap_err();
    assert_eq!(
        error.kind(),
        BytesErrorKind::OutputLimitExceeded { limit: 4 }
    );
    assert_eq!(error.progress().output_committed(), 4);
    assert!(decoder.is_failed());
}

#[test]
fn finite_buf_mut_reports_retryable_output_full() {
    let mut input = Bytes::from_static(b"hello");
    let mut first = [0u8; 3];
    let mut first_destination = &mut first[..];
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();

    let first_step = encoder.update(&mut input, &mut first_destination).unwrap();
    assert!(matches!(first_step.status(), Status::OutputFull(_)));
    assert_eq!(first_step.progress().output_committed(), 3);

    let mut remainder = Vec::new();
    let second_step = encoder.update(&mut input, &mut remainder).unwrap();
    assert!(matches!(second_step.status(), Status::NeedInput));
    let final_step = encoder.finish(&mut remainder).unwrap();
    assert!(matches!(final_step.status(), Status::Complete));

    let mut combined = first.to_vec();
    combined.extend_from_slice(&remainder);
    assert_eq!(combined, b"aGVsbG8=");
}

#[test]
fn invalid_safe_buf_contract_is_rejected_and_latched() {
    let mut input = EmptyChunkBuf { remaining: 4 };
    let mut output = Vec::new();
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();
    let error = encoder.update(&mut input, &mut output).unwrap_err();
    assert_eq!(
        error.kind(),
        BytesErrorKind::InvalidInputBuffer { remaining: 4 }
    );
    assert_eq!(error.input_cursor_progress(), InputCursorProgress::Exact(0));
    assert!(encoder.is_failed());
}

#[test]
fn changing_remaining_cannot_bypass_the_cumulative_input_limit() {
    let mut input = ExpandingRemainingBuf::new(b"hello", 1);
    let mut output = Vec::new();
    let mut encoder =
        STRICT_STANDARD_PADDED.bytes_encoder_with_limits(BytesLimits::new(4, usize::MAX));

    let error = encoder.update(&mut input, &mut output).unwrap_err();

    assert_eq!(
        error.kind(),
        BytesErrorKind::InputLimitExceeded {
            required: 5,
            limit: 4,
        }
    );
    assert_eq!(error.progress(), BytesProgress::ZERO);
    assert_eq!(input.advanced, 0);
    assert!(output.is_empty());
    assert!(encoder.is_failed());
}

#[test]
fn inconsistent_post_advance_remaining_reports_committed_progress() {
    let mut input = InconsistentAfterAdvanceBuf::new(b"abc");
    let mut output = Vec::new();
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();

    let error = encoder.update(&mut input, &mut output).unwrap_err();

    assert_eq!(input.advanced, 3);
    assert_eq!(error.progress().input_consumed(), 3);
    assert_eq!(error.progress().output_committed(), 4);
    assert_eq!(
        error.input_cursor_progress(),
        InputCursorProgress::Indeterminate
    );
    assert_eq!(output, b"YWJj");
    assert_eq!(
        error.kind(),
        BytesErrorKind::InvalidInputBuffer { remaining: 1 }
    );
    assert!(encoder.is_failed());
}

#[test]
fn no_op_advance_reports_indeterminate_cursor_progress() {
    let mut input = NoOpAdvanceBuf;
    let mut output = Vec::new();
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();

    let error = encoder.update(&mut input, &mut output).unwrap_err();

    assert_eq!(input.remaining(), 3);
    assert_eq!(error.progress().input_consumed(), 3);
    assert_eq!(error.progress().output_committed(), 4);
    assert_eq!(
        error.input_cursor_progress(),
        InputCursorProgress::Indeterminate
    );
    assert_eq!(output, b"YWJj");
    assert_eq!(
        error.kind(),
        BytesErrorKind::InvalidInputBuffer { remaining: 3 }
    );
    assert!(encoder.is_failed());
}

#[test]
fn downstream_panic_latches_state_until_reset() {
    let mut input = Bytes::from_static(b"abc");
    let mut output = PanicAfterWrite::default();
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();

    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = encoder.update(&mut input, &mut output);
    }));
    assert!(result.is_err());
    assert!(encoder.is_failed());
    assert_eq!(output.bytes, b"YWJj");

    let error = encoder
        .update(&mut Bytes::new(), &mut Vec::new())
        .unwrap_err();
    assert_eq!(error.kind(), BytesErrorKind::FailedState);
    encoder.reset();
    assert!(!encoder.is_failed());
}

#[test]
fn input_panic_latches_state_until_reset() {
    let mut encoder = STRICT_STANDARD_PADDED.bytes_encoder();
    let mut input = PanicOnChunk;
    let mut output = Vec::new();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = encoder.update(&mut input, &mut output);
    }));
    assert!(panic.is_err());
    assert!(encoder.is_failed());
    assert_eq!(
        encoder
            .update(&mut Bytes::from_static(b"ok"), &mut output)
            .unwrap_err()
            .kind(),
        BytesErrorKind::FailedState
    );

    encoder.reset();
    assert!(!encoder.is_failed());
}

struct OneByteBuf {
    bytes: Bytes,
}

impl OneByteBuf {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: Bytes::copy_from_slice(bytes),
        }
    }
}

impl Buf for OneByteBuf {
    fn remaining(&self) -> usize {
        self.bytes.remaining()
    }

    fn chunk(&self) -> &[u8] {
        let len = self.bytes.remaining().min(1);
        &self.bytes.chunk()[..len]
    }

    fn advance(&mut self, count: usize) {
        self.bytes.advance(count);
    }
}

struct EmptyChunkBuf {
    remaining: usize,
}

struct InconsistentAfterAdvanceBuf {
    bytes: Bytes,
    advanced: usize,
}

struct NoOpAdvanceBuf;

impl Buf for NoOpAdvanceBuf {
    fn remaining(&self) -> usize {
        3
    }

    fn chunk(&self) -> &[u8] {
        b"abc"
    }

    fn advance(&mut self, _count: usize) {}
}

impl InconsistentAfterAdvanceBuf {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: Bytes::copy_from_slice(bytes),
            advanced: 0,
        }
    }
}

impl Buf for InconsistentAfterAdvanceBuf {
    fn remaining(&self) -> usize {
        self.bytes.remaining() + usize::from(self.advanced != 0)
    }

    fn chunk(&self) -> &[u8] {
        self.bytes.chunk()
    }

    fn advance(&mut self, count: usize) {
        self.bytes.advance(count);
        self.advanced += count;
    }
}

struct ExpandingRemainingBuf {
    bytes: Bytes,
    first_report: usize,
    calls: Cell<usize>,
    advanced: usize,
}

impl ExpandingRemainingBuf {
    fn new(bytes: &[u8], first_report: usize) -> Self {
        Self {
            bytes: Bytes::copy_from_slice(bytes),
            first_report,
            calls: Cell::new(0),
            advanced: 0,
        }
    }
}

impl Buf for ExpandingRemainingBuf {
    fn remaining(&self) -> usize {
        let calls = self.calls.get();
        self.calls.set(calls + 1);
        if calls == 0 {
            self.first_report
        } else {
            self.bytes.remaining()
        }
    }

    fn chunk(&self) -> &[u8] {
        self.bytes.chunk()
    }

    fn advance(&mut self, count: usize) {
        self.bytes.advance(count);
        self.advanced += count;
    }
}

impl Buf for EmptyChunkBuf {
    fn remaining(&self) -> usize {
        self.remaining
    }

    fn chunk(&self) -> &[u8] {
        &[]
    }

    fn advance(&mut self, count: usize) {
        self.remaining -= count;
    }
}

#[derive(Default)]
struct PanicAfterWrite {
    bytes: Vec<u8>,
}

struct PanicOnChunk;

impl Buf for PanicOnChunk {
    fn remaining(&self) -> usize {
        1
    }

    fn chunk(&self) -> &[u8] {
        panic!("injected input panic");
    }

    fn advance(&mut self, _count: usize) {}
}

// SAFETY: All required storage operations delegate to Vec's BufMut
// implementation. The deliberate panic occurs only after a fully initialized
// slice has been appended and models an adversarial downstream implementation.
unsafe impl BufMut for PanicAfterWrite {
    fn remaining_mut(&self) -> usize {
        self.bytes.remaining_mut()
    }

    unsafe fn advance_mut(&mut self, count: usize) {
        // SAFETY: The caller upholds BufMut's initialization contract, which is
        // forwarded unchanged to Vec.
        unsafe { self.bytes.advance_mut(count) };
    }

    fn chunk_mut(&mut self) -> &mut UninitSlice {
        self.bytes.chunk_mut()
    }

    fn put_slice(&mut self, source: &[u8]) {
        self.bytes.put_slice(source);
        panic!("injected downstream panic after observable write");
    }
}
