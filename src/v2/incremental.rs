//! Heapless incremental ordinary transforms.

use core::num::NonZeroUsize;

use super::{
    contracts::{Lifecycle, OperationError, Progress, Step},
    specifications::{Base64, Codec, CodecSettings, EncodePadding},
};

const INPUT_QUANTUM: usize = 3;
const OUTPUT_QUANTUM: usize = 4;

/// Heapless ordinary Base64 encoder state.
///
/// The state retains at most two incomplete input bytes and one four-byte
/// encoded quantum. It intentionally has no cleanup destructor; secret
/// transforms use a separate state type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EncoderState {
    settings: CodecSettings,
    tail: [u8; INPUT_QUANTUM],
    tail_len: usize,
    pending: [u8; OUTPUT_QUANTUM],
    pending_start: usize,
    pending_len: usize,
    lifecycle: Lifecycle,
}

impl EncoderState {
    pub(crate) const fn new(settings: CodecSettings) -> Self {
        Self {
            settings,
            tail: [0; INPUT_QUANTUM],
            tail_len: 0,
            pending: [0; OUTPUT_QUANTUM],
            pending_start: 0,
            pending_len: 0,
            lifecycle: Lifecycle::new(),
        }
    }

    /// Accepts an input prefix and writes as much encoded output as fits.
    pub(crate) fn update(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<Step, OperationError> {
        let span = self.lifecycle.reserve_input(input.len())?;
        let consumed =
            planned_input_consumption(self.tail_len, self.pending_len, input.len(), output.len());
        self.lifecycle.commit_input(span, consumed)?;

        let mut produced = self.drain_pending(output);
        let mut input_offset = 0;
        while input_offset < consumed {
            self.tail[self.tail_len] = input[input_offset];
            self.tail_len += 1;
            input_offset += 1;

            if self.tail_len == INPUT_QUANTUM {
                self.pending = encode_quantum(self.settings, self.tail);
                self.pending_start = 0;
                self.pending_len = OUTPUT_QUANTUM;
                self.tail_len = 0;
                produced += self.drain_pending(&mut output[produced..]);
            }
        }

        let progress = Progress::new(consumed, produced);
        if self.pending_len != 0 || consumed != input.len() {
            self.lifecycle.output_full(progress, NonZeroUsize::MIN)
        } else {
            self.lifecycle.need_input(progress)
        }
    }

    /// Emits the canonical final tail and completes the current message.
    pub(crate) fn finish(&mut self, output: &mut [u8]) -> Result<Step, OperationError> {
        if self.lifecycle.begin_finish()? {
            return self.lifecycle.finish(Progress::ZERO);
        }

        let mut produced = self.drain_pending(output);
        if self.pending_len == 0 && self.tail_len != 0 {
            self.pending_len = encode_tail(
                self.settings,
                &self.tail[..self.tail_len],
                &mut self.pending,
            );
            self.pending_start = 0;
            self.tail_len = 0;
            produced += self.drain_pending(&mut output[produced..]);
        }

        let progress = Progress::new(0, produced);
        if self.pending_len == 0 {
            self.lifecycle.finish(progress)
        } else {
            self.lifecycle.output_full(progress, NonZeroUsize::MIN)
        }
    }

    /// Resets the state for one unrelated message.
    pub(crate) fn reset(&mut self) {
        self.tail = [0; INPUT_QUANTUM];
        self.tail_len = 0;
        self.pending = [0; OUTPUT_QUANTUM];
        self.pending_start = 0;
        self.pending_len = 0;
        self.lifecycle.reset();
    }

    pub(crate) const fn source_position(&self) -> usize {
        self.lifecycle.source_position()
    }

    fn drain_pending(&mut self, output: &mut [u8]) -> usize {
        let written = self.pending_len.min(output.len());
        let pending_end = self.pending_start + written;
        output[..written].copy_from_slice(&self.pending[self.pending_start..pending_end]);
        self.pending_start = pending_end;
        self.pending_len -= written;
        if self.pending_len == 0 {
            self.pending_start = 0;
        }
        written
    }

    #[cfg(kani)]
    pub(crate) const fn proof_invariants(&self) -> bool {
        self.tail_len < INPUT_QUANTUM
            && self.pending_start <= OUTPUT_QUANTUM
            && self.pending_len <= OUTPUT_QUANTUM
            && self.pending_start + self.pending_len <= OUTPUT_QUANTUM
    }
}

impl<S: Codec> Base64<S> {
    /// Constructs a fresh heapless ordinary encoder.
    pub(crate) fn encoder(&self) -> EncoderState {
        EncoderState::new(self.settings())
    }
}

fn planned_input_consumption(
    initial_tail: usize,
    pending: usize,
    input: usize,
    output: usize,
) -> usize {
    let pending_written = pending.min(output);
    if pending_written != pending {
        return 0;
    }

    let mut available_output = output - pending_written;
    let mut tail = initial_tail;
    let mut consumed = 0;
    while consumed < input {
        let copied = (INPUT_QUANTUM - tail).min(input - consumed);
        consumed += copied;
        tail += copied;
        if tail != INPUT_QUANTUM {
            break;
        }

        tail = 0;
        let written = OUTPUT_QUANTUM.min(available_output);
        available_output -= written;
        if written != OUTPUT_QUANTUM {
            break;
        }
    }
    consumed
}

fn encode_quantum(settings: CodecSettings, input: [u8; INPUT_QUANTUM]) -> [u8; OUTPUT_QUANTUM] {
    let table = settings.alphabet().as_array();
    [
        table[usize::from(input[0] >> 2)],
        table[usize::from(((input[0] & 0x03) << 4) | (input[1] >> 4))],
        table[usize::from(((input[1] & 0x0f) << 2) | (input[2] >> 6))],
        table[usize::from(input[2] & 0x3f)],
    ]
}

fn encode_tail(settings: CodecSettings, input: &[u8], output: &mut [u8; 4]) -> usize {
    let table = settings.alphabet().as_array();
    output[0] = table[usize::from(input[0] >> 2)];
    output[1] = table[usize::from((input[0] & 0x03) << 4)];

    if input.len() == 1 {
        if settings.encode_padding() == EncodePadding::Padded {
            output[2] = b'=';
            output[3] = b'=';
            4
        } else {
            2
        }
    } else {
        output[1] = table[usize::from(((input[0] & 0x03) << 4) | (input[1] >> 4))];
        output[2] = table[usize::from((input[1] & 0x0f) << 2)];
        if settings.encode_padding() == EncodePadding::Padded {
            output[3] = b'=';
            4
        } else {
            3
        }
    }
}
