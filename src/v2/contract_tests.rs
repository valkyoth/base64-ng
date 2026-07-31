use core::num::NonZeroUsize;

use super::contracts::{
    AssuranceClass, Atomicity, BackendClass, BackendFault, Failure, InputError, InputErrorKind,
    Lifecycle, OperationError, Progress, ProtocolScope, Status, TerminalError,
};

struct StackText {
    bytes: [u8; 128],
    len: usize,
}

impl StackText {
    const fn new() -> Self {
        Self {
            bytes: [0; 128],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}

impl core::fmt::Write for StackText {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let Some(end) = self.len.checked_add(text.len()) else {
            return Err(core::fmt::Error);
        };
        let Some(destination) = self.bytes.get_mut(self.len..end) else {
            return Err(core::fmt::Error);
        };
        destination.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[test]
fn output_full_is_retryable_and_finish_is_idempotent() {
    let mut state = Lifecycle::new();
    let span = state.reserve_input(3).unwrap();
    assert_eq!(span.index(0), Some(0));
    assert_eq!(span.index(2), Some(2));
    assert_eq!(span.index(3), None);
    state.commit_input(span, 2).unwrap();

    let full = state
        .output_full(Progress::new(2, 0), NonZeroUsize::new(4).unwrap())
        .unwrap();
    assert_eq!(full.progress().input_consumed(), 2);
    assert_eq!(full.progress().output_produced(), 0);
    let Status::OutputFull(requirement) = full.status() else {
        panic!("expected output-full status");
    };
    assert_eq!(requirement.minimum_output().get(), 4);

    let retry = state.reserve_input(1).unwrap();
    assert_eq!(retry.index(0), Some(2));
    state.commit_input(retry, 1).unwrap();
    assert_eq!(
        state.need_input(Progress::new(1, 4)).unwrap().status(),
        Status::NeedInput
    );
    assert_eq!(
        state.finish(Progress::new(0, 2)).unwrap().status(),
        Status::Complete
    );
    assert_eq!(
        state.finish(Progress::new(9, 9)).unwrap().progress(),
        Progress::ZERO
    );
    assert_eq!(
        state.reserve_input(1),
        Err(OperationError::Terminal(TerminalError::InputAfterComplete))
    );

    state.reset();
    assert_eq!(state.source_position(), 0);
    assert_eq!(
        state.need_input(Progress::ZERO).unwrap().status(),
        Status::NeedInput
    );
}

#[test]
fn malformed_and_backend_failures_are_absorbing_until_reset() {
    let malformed = Failure::Input(InputError::InvalidByte {
        index: 7,
        byte: b'!',
    });
    let mut state = Lifecycle::new();
    assert_eq!(state.fail(malformed), OperationError::Failed(malformed));
    assert_eq!(
        state.need_input(Progress::ZERO),
        Err(OperationError::Failed(malformed))
    );
    assert_eq!(
        state.finish(Progress::ZERO),
        Err(OperationError::Failed(malformed))
    );
    assert_eq!(
        state.fail(Failure::Backend(BackendFault::ImpossibleState)),
        OperationError::Failed(malformed)
    );

    state.reset();
    let backend = Failure::Backend(BackendFault::OutputMismatch);
    assert_eq!(state.fail(backend), OperationError::Failed(backend));
    assert_eq!(backend.as_str(), "backend-output-mismatch");
}

#[test]
fn truncation_is_a_distinct_absorbing_input_failure() {
    let mut state = Lifecycle::new();
    let span = state.reserve_input(3).unwrap();
    state.commit_input(span, 3).unwrap();
    let truncated = Failure::Input(InputError::TruncatedInput {
        index: state.source_position(),
    });
    assert_eq!(truncated.as_str(), "truncated-input");
    assert_eq!(state.fail(truncated), OperationError::Failed(truncated));
    assert_eq!(
        state.output_full(Progress::ZERO, NonZeroUsize::MIN),
        Err(OperationError::Failed(truncated))
    );
}

#[test]
fn compacted_and_whitespace_only_chunks_keep_original_indexes() {
    let chunks: &[&[u8]] = &[b" \t", b"Z \n", b"g", b"=="];
    let mut state = Lifecycle::new();
    let mut compacted = [0_u8; 4];
    let mut indexes = [0_usize; 4];
    let mut compacted_len = 0;

    for chunk in chunks {
        let span = state.reserve_input(chunk.len()).unwrap();
        for (local, byte) in chunk.iter().copied().enumerate() {
            if !matches!(byte, b' ' | b'\t' | b'\r' | b'\n') {
                compacted[compacted_len] = byte;
                indexes[compacted_len] = span.index(local).unwrap();
                compacted_len += 1;
            }
        }
        state.commit_input(span, chunk.len()).unwrap();
    }

    assert_eq!(compacted_len, compacted.len());
    assert_eq!(compacted, *b"Zg==");
    assert_eq!(indexes, [2, 5, 6, 7]);
    assert_eq!(state.source_position(), 8);
}

#[test]
fn source_position_overflow_is_checked_before_processing_and_absorbing() {
    let mut state = Lifecycle::at_source_position(usize::MAX - 3);
    let span = state.reserve_input(3).unwrap();
    assert_eq!(span.index(0), Some(usize::MAX - 3));
    assert_eq!(span.index(2), Some(usize::MAX - 1));
    assert_eq!(state.source_position(), usize::MAX - 3);
    state.commit_input(span, 3).unwrap();
    assert_eq!(state.source_position(), usize::MAX);

    assert_eq!(
        state.reserve_input(1),
        Err(OperationError::Failed(Failure::PositionOverflow))
    );
    assert_eq!(state.source_position(), usize::MAX);
    assert_eq!(
        state.reserve_input(0),
        Err(OperationError::Failed(Failure::PositionOverflow))
    );
    assert_eq!(
        state.finish(Progress::ZERO),
        Err(OperationError::Failed(Failure::PositionOverflow))
    );
}

#[test]
fn invalid_internal_source_commits_fail_closed_without_panicking() {
    let mut oversized = Lifecycle::new();
    let span = oversized.reserve_input(2).unwrap();
    assert_eq!(
        oversized.commit_input(span, 3),
        Err(OperationError::Failed(Failure::Backend(
            BackendFault::ImpossibleState
        )))
    );

    let mut stale = Lifecycle::new();
    let span = stale.reserve_input(2).unwrap();
    stale.commit_input(span, 1).unwrap();
    assert_eq!(
        stale.commit_input(span, 1),
        Err(OperationError::Failed(Failure::Backend(
            BackendFault::ImpossibleState
        )))
    );
}

#[test]
fn detailed_input_debug_is_redacted_and_kind_is_stable() {
    let error = InputError::InvalidByte {
        index: 123_456,
        byte: 0xfe,
    };
    let mut debug = StackText::new();
    let mut display = StackText::new();
    core::fmt::write(&mut debug, format_args!("{error:?}")).unwrap();
    core::fmt::write(&mut display, format_args!("{error}")).unwrap();
    let debug = debug.as_str();
    let display = display.as_str();

    assert_eq!(error.kind(), InputErrorKind::InvalidByte);
    assert_eq!(error.kind().as_str(), "invalid-byte");
    assert!(!debug.contains("123456"));
    assert!(!debug.contains("fe"));
    assert!(display.contains("123456"));
    assert!(display.contains("fe"));
}

#[test]
fn reporting_and_atomicity_identifiers_are_stable() {
    assert_eq!(Status::NeedInput.as_str(), "need-input");
    assert_eq!(BackendClass::Scalar.as_str(), "scalar");
    assert_eq!(BackendClass::Accelerated.as_str(), "accelerated");
    assert_eq!(BackendClass::Checked.as_str(), "checked");
    assert_eq!(AssuranceClass::Ordinary.as_str(), "ordinary");
    assert_eq!(AssuranceClass::CheckedBackend.as_str(), "checked-backend");
    assert_eq!(AssuranceClass::Secret.as_str(), "secret");
    assert_eq!(AssuranceClass::HighAssurance.as_str(), "high-assurance");
    assert_eq!(ProtocolScope::Core.as_str(), "core");
    assert_eq!(ProtocolScope::Compatibility.as_str(), "compatibility");
    assert_eq!(ProtocolScope::Companion.as_str(), "companion");
    assert_eq!(Atomicity::Unchanged.as_str(), "unchanged");
    assert_eq!(Atomicity::Rollback.as_str(), "rollback");
    assert_eq!(Atomicity::CommittedPrefix.as_str(), "committed-prefix");
    assert_eq!(Atomicity::IrrevocableSink.as_str(), "irrevocable-sink");
}
