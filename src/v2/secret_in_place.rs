//! Fixed-work staged secret decode for finite caller-owned buffers.

#[cfg(test)]
use super::contracts::BackendFault;
use super::{
    in_place::{InPlaceError, require_disjoint_slices, require_input_prefix},
    specifications::{Base64, Codec, CodecSettings, DecodePadding},
};

struct SecretDecodeOutcome {
    written: usize,
    invalid: u8,
}

#[derive(Clone, Copy)]
enum InjectedFault {
    None,
    #[cfg(test)]
    AfterDecode,
    #[cfg(all(test, feature = "std"))]
    PanicAfterDecode,
}

struct StagingWipeGuard<'a> {
    bytes: &'a mut [u8],
}

impl<'a> StagingWipeGuard<'a> {
    fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes }
    }

    fn prefix_mut(&mut self, len: usize) -> &mut [u8] {
        &mut self.bytes[..len]
    }

    fn prefix(&self, len: usize) -> &[u8] {
        &self.bytes[..len]
    }
}

impl Drop for StagingWipeGuard<'_> {
    fn drop(&mut self) {
        crate::wipe_bytes(self.bytes);
    }
}

#[cfg(test)]
static FIXED_WORK_CALLS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static SYMBOL_SCANS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

impl<S: Codec> Base64<S> {
    /// Returns the private staging capacity required by secret in-place decode.
    ///
    /// The capacity depends only on the public encoded length. It deliberately
    /// reserves three candidate bytes for every complete or partial encoded
    /// block so padding validity is not inspected during preflight.
    pub fn secret_decode_staging_len(&self, input_len: usize) -> Result<usize, InPlaceError> {
        if !self.settings().permits_secret_processing() {
            return Err(InPlaceError::SecretPolicyUnsupported);
        }
        fixed_work_staging_len(input_len)
    }

    /// Decodes a secret-bearing encoded prefix through disjoint private staging.
    ///
    /// Preflight errors leave both buffers byte-for-byte unchanged. Every
    /// post-preflight input, valid or invalid, receives the same number of
    /// alphabet scans for its public length. Invalid input leaves `buffer`
    /// unchanged and wipes all staging. Success copies the staged plaintext
    /// into `buffer` only after the result gate, then wipes all staging.
    ///
    /// The complete `buffer` and `private_staging` byte ranges must be
    /// disjoint. Unsafe callers must validate raw ranges before constructing
    /// mutable references; this check cannot repair pre-existing aliased
    /// references. Validity and successful release become public after the
    /// result gate; the success-only copy is outside the fixed-work claim.
    pub fn decode_in_place_staged(
        &self,
        buffer: &mut [u8],
        input_len: usize,
        private_staging: &mut [u8],
    ) -> Result<usize, InPlaceError> {
        decode_in_place_staged_inner(
            self,
            buffer,
            input_len,
            private_staging,
            InjectedFault::None,
        )
    }
}

fn decode_in_place_staged_inner<S: Codec>(
    codec: &Base64<S>,
    buffer: &mut [u8],
    input_len: usize,
    private_staging: &mut [u8],
    injected_fault: InjectedFault,
) -> Result<usize, InPlaceError> {
    require_input_prefix(input_len, buffer.len())?;
    let settings = codec.settings();
    if !settings.permits_secret_processing() {
        return Err(InPlaceError::SecretPolicyUnsupported);
    }
    let staging_len = fixed_work_staging_len(input_len)?;
    if private_staging.len() < staging_len {
        return Err(InPlaceError::StagingTooSmall {
            required: staging_len,
            available: private_staging.len(),
        });
    }
    require_disjoint_slices(buffer, private_staging)?;

    let mut staging_guard = StagingWipeGuard::new(private_staging);
    let outcome = decode_secret_fixed_work(
        settings,
        &buffer[..input_len],
        staging_guard.prefix_mut(staging_len),
    );

    #[cfg(all(test, feature = "std"))]
    if matches!(injected_fault, InjectedFault::PanicAfterDecode) {
        std::panic::panic_any("reviewed staged secret cleanup test");
    }

    #[cfg(test)]
    if matches!(injected_fault, InjectedFault::AfterDecode) {
        crate::wipe_bytes(buffer);
        return Err(InPlaceError::Backend(BackendFault::ImpossibleState));
    }
    let _ = injected_fault;

    crate::ct_error_gate_barrier(outcome.invalid, 0);
    if core::hint::black_box(outcome.invalid) != 0 {
        return Err(InPlaceError::InvalidSecretInput);
    }

    buffer[..outcome.written].copy_from_slice(staging_guard.prefix(outcome.written));
    Ok(outcome.written)
}

fn fixed_work_staging_len(input_len: usize) -> Result<usize, InPlaceError> {
    let complete = (input_len / 4)
        .checked_mul(3)
        .ok_or(InPlaceError::LengthOverflow)?;
    if input_len.is_multiple_of(4) {
        Ok(complete)
    } else {
        complete.checked_add(3).ok_or(InPlaceError::LengthOverflow)
    }
}

fn decode_secret_fixed_work(
    settings: CodecSettings,
    input: &[u8],
    staging: &mut [u8],
) -> SecretDecodeOutcome {
    #[cfg(test)]
    FIXED_WORK_CALLS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    match settings.decode_padding() {
        DecodePadding::RequireCanonical => decode_padded(settings, input, staging),
        DecodePadding::Forbid => decode_unpadded(settings, input, staging),
        DecodePadding::Indifferent => SecretDecodeOutcome {
            written: 0,
            invalid: 0xff,
        },
    }
}

fn decode_padded(settings: CodecSettings, input: &[u8], staging: &mut [u8]) -> SecretDecodeOutcome {
    if input.is_empty() {
        return SecretDecodeOutcome {
            written: 0,
            invalid: 0,
        };
    }

    let mut invalid = if input.len().is_multiple_of(4) {
        0
    } else {
        0xff
    };
    let mut read = 0;
    let mut write = 0;
    let mut final_padding = 0u8;

    while read < input.len() {
        let actual = (input.len() - read).min(4);
        let bytes = read_block(input, read);
        let values = decode_block(settings, bytes);
        write_candidate(staging, write, values);
        let final_block = input.len() - read <= 4;

        if final_block {
            if actual != 4 {
                invalid = accumulate(invalid, 0xff);
            }
            let equals_third = crate::ct_mask_eq_u8(bytes[2], b'=');
            let equals_fourth = crate::ct_mask_eq_u8(bytes[3], b'=');
            let no_padding = !equals_third & !equals_fourth;
            let one_padding = !equals_third & equals_fourth;
            let two_padding = equals_third & equals_fourth;
            let malformed_padding = equals_third & !equals_fourth;
            let require_third = no_padding | one_padding;

            invalid = accumulate(invalid, !values[0].1);
            invalid = accumulate(invalid, !values[1].1);
            invalid = accumulate(invalid, !values[2].1 & require_third);
            invalid = accumulate(invalid, !values[3].1 & no_padding);
            invalid = accumulate(invalid, malformed_padding);
            invalid = accumulate(
                invalid,
                crate::ct_mask_nonzero_u8(values[1].0 & 0x0f) & two_padding,
            );
            invalid = accumulate(
                invalid,
                crate::ct_mask_nonzero_u8(values[2].0 & 0x03) & one_padding,
            );
            final_padding = (equals_third & 1) + (equals_fourth & 1);
        } else {
            invalid = accumulate(invalid, !values[0].1);
            invalid = accumulate(invalid, !values[1].1);
            invalid = accumulate(invalid, !values[2].1);
            invalid = accumulate(invalid, !values[3].1);
        }

        read += actual;
        write += 3;
    }

    SecretDecodeOutcome {
        written: write - usize::from(final_padding),
        invalid,
    }
}

fn decode_unpadded(
    settings: CodecSettings,
    input: &[u8],
    staging: &mut [u8],
) -> SecretDecodeOutcome {
    let mut invalid = 0u8;
    let mut read = 0;
    let mut write = 0;
    let mut visible = 0;

    while read < input.len() {
        let actual = (input.len() - read).min(4);
        let bytes = read_block(input, read);
        let values = decode_block(settings, bytes);
        write_candidate(staging, write, values);

        invalid = accumulate(invalid, !values[0].1);
        match actual {
            4 => {
                invalid = accumulate(invalid, !values[1].1);
                invalid = accumulate(invalid, !values[2].1);
                invalid = accumulate(invalid, !values[3].1);
                visible += 3;
            }
            3 => {
                invalid = accumulate(invalid, !values[1].1);
                invalid = accumulate(invalid, !values[2].1);
                invalid = accumulate(invalid, crate::ct_mask_nonzero_u8(values[2].0 & 0x03));
                visible += 2;
            }
            2 => {
                invalid = accumulate(invalid, !values[1].1);
                invalid = accumulate(invalid, crate::ct_mask_nonzero_u8(values[1].0 & 0x0f));
                visible += 1;
            }
            _ => invalid = accumulate(invalid, 0xff),
        }

        read += actual;
        write += 3;
    }

    SecretDecodeOutcome {
        written: visible,
        invalid,
    }
}

fn read_block(input: &[u8], read: usize) -> [u8; 4] {
    [
        input.get(read).copied().unwrap_or(0),
        input.get(read + 1).copied().unwrap_or(0),
        input.get(read + 2).copied().unwrap_or(0),
        input.get(read + 3).copied().unwrap_or(0),
    ]
}

fn decode_block(settings: CodecSettings, bytes: [u8; 4]) -> [(u8, u8); 4] {
    [
        decode_symbol(settings, bytes[0]),
        decode_symbol(settings, bytes[1]),
        decode_symbol(settings, bytes[2]),
        decode_symbol(settings, bytes[3]),
    ]
}

#[inline(never)]
fn decode_symbol(settings: CodecSettings, byte: u8) -> (u8, u8) {
    #[cfg(test)]
    SYMBOL_SCANS.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

    let mut decoded = 0u8;
    let mut valid = 0u8;
    let mut candidate = 0u8;
    while candidate < 64 {
        let matches = core::hint::black_box(crate::ct_mask_eq_u8(
            core::hint::black_box(byte),
            core::hint::black_box(settings.alphabet().as_array()[usize::from(candidate)]),
        ));
        decoded = accumulate(decoded, candidate & matches);
        valid = accumulate(valid, matches);
        candidate += 1;
    }
    (decoded, valid)
}

fn write_candidate(staging: &mut [u8], write: usize, values: [(u8, u8); 4]) {
    staging[write] = (values[0].0 << 2) | (values[1].0 >> 4);
    staging[write + 1] = (values[1].0 << 4) | (values[2].0 >> 2);
    staging[write + 2] = (values[2].0 << 6) | values[3].0;
}

fn accumulate(accumulator: u8, value: u8) -> u8 {
    crate::ct_accumulate_u8(accumulator, value)
}

#[cfg(test)]
pub(super) fn decode_with_injected_fault_for_test<S: Codec>(
    codec: &Base64<S>,
    buffer: &mut [u8],
    input_len: usize,
    private_staging: &mut [u8],
) -> Result<usize, InPlaceError> {
    decode_in_place_staged_inner(
        codec,
        buffer,
        input_len,
        private_staging,
        InjectedFault::AfterDecode,
    )
}

#[cfg(all(test, feature = "std"))]
pub(super) fn decode_with_injected_panic_for_test<S: Codec>(
    codec: &Base64<S>,
    buffer: &mut [u8],
    input_len: usize,
    private_staging: &mut [u8],
) {
    let _ = decode_in_place_staged_inner(
        codec,
        buffer,
        input_len,
        private_staging,
        InjectedFault::PanicAfterDecode,
    );
}

#[cfg(test)]
pub(super) fn reset_work_counters_for_test() {
    FIXED_WORK_CALLS.store(0, core::sync::atomic::Ordering::Relaxed);
    SYMBOL_SCANS.store(0, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
pub(super) fn work_counters_for_test() -> (usize, usize) {
    (
        FIXED_WORK_CALLS.load(core::sync::atomic::Ordering::Relaxed),
        SYMBOL_SCANS.load(core::sync::atomic::Ordering::Relaxed),
    )
}
