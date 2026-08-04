use super::{DecoderReader, EncoderReader, Status, valid_finish_progress, valid_update_progress};
use base64_ng::STRICT_STANDARD_PADDED;
use tokio::io::AsyncReadExt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProgressFault {
    OversizedUpdate,
    OversizedFinish,
    IncompleteFinish,
}

pub(super) const fn update_produced(
    fault: Option<ProgressFault>,
    produced: usize,
    capacity: usize,
) -> usize {
    match fault {
        Some(ProgressFault::OversizedUpdate) => capacity.saturating_add(1),
        _ => produced,
    }
}

pub(super) const fn finish_progress(
    fault: Option<ProgressFault>,
    produced: usize,
    capacity: usize,
    status: Status,
) -> (usize, Status) {
    match fault {
        Some(ProgressFault::OversizedFinish) => (capacity.saturating_add(1), status),
        Some(ProgressFault::IncompleteFinish) => (produced, Status::NeedInput),
        _ => (produced, status),
    }
}

macro_rules! assert_failed_and_cleared {
    ($reader:expr) => {{
        assert!($reader.is_failed());
        assert_eq!($reader.output_pos, 0);
        assert_eq!($reader.output_len, 0);
        assert!($reader.input.iter().all(|byte| *byte == 0));
        assert!($reader.output.iter().all(|byte| *byte == 0));
    }};
}

#[test]
fn progress_contract_rejects_untrusted_bounds_and_status() {
    assert!(valid_update_progress(4, 4, 3, 3));
    assert!(!valid_update_progress(4, 3, 3, 3));
    assert!(!valid_update_progress(4, 4, 4, 3));

    assert!(valid_finish_progress(3, 3, Status::Complete));
    assert!(!valid_finish_progress(4, 3, Status::Complete));
    assert!(!valid_finish_progress(3, 3, Status::NeedInput));
}

#[tokio::test]
async fn injected_update_progress_fails_and_clears_both_readers() {
    let marker = u8::try_from(std::process::id() & 0xff).unwrap() | 1;

    let mut encoder = EncoderReader::new(&b"abc"[..], &STRICT_STANDARD_PADDED);
    encoder.input.fill(marker);
    encoder.output.fill(marker);
    encoder.progress_fault = Some(ProgressFault::OversizedUpdate);
    assert!(encoder.read(&mut [0u8; 8]).await.is_err());
    assert_failed_and_cleared!(encoder);

    let mut decoder = DecoderReader::new(&b"YWJj"[..], &STRICT_STANDARD_PADDED);
    decoder.input.fill(marker);
    decoder.output.fill(marker);
    decoder.progress_fault = Some(ProgressFault::OversizedUpdate);
    assert!(decoder.read(&mut [0u8; 8]).await.is_err());
    assert_failed_and_cleared!(decoder);
}

#[tokio::test]
async fn injected_finish_progress_fails_and_clears_both_readers() {
    for fault in [
        ProgressFault::OversizedFinish,
        ProgressFault::IncompleteFinish,
    ] {
        let marker = u8::try_from(std::process::id() & 0xff).unwrap() | 1;

        let mut encoder = EncoderReader::new(&b""[..], &STRICT_STANDARD_PADDED);
        encoder.input.fill(marker);
        encoder.output.fill(marker);
        encoder.progress_fault = Some(fault);
        assert!(encoder.read(&mut [0u8; 8]).await.is_err());
        assert_failed_and_cleared!(encoder);

        let mut decoder = DecoderReader::new(&b""[..], &STRICT_STANDARD_PADDED);
        decoder.input.fill(marker);
        decoder.output.fill(marker);
        decoder.progress_fault = Some(fault);
        assert!(decoder.read(&mut [0u8; 8]).await.is_err());
        assert_failed_and_cleared!(decoder);
    }
}
