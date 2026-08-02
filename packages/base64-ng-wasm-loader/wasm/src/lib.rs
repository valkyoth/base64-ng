#![no_std]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

use base64_ng::{
    DecodeError, EncodeError, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD, web,
};

const MAX_INPUT: usize = 1024 * 1024;
const MAX_OUTPUT: usize = MAX_INPUT.div_ceil(3) * 4;
const NO_INDEX: u32 = u32::MAX;

const ERROR_NONE: u32 = 0;
const ERROR_INVALID_CODEC: u32 = 1;
const ERROR_INPUT_LIMIT: u32 = 2;
const ERROR_INVALID_INPUT: u32 = 3;
const ERROR_INVALID_LENGTH: u32 = 4;
const ERROR_INVALID_BYTE: u32 = 5;
const ERROR_INVALID_PADDING: u32 = 6;
const ERROR_OUTPUT_CAPACITY: u32 = 7;
const ERROR_BACKEND: u32 = 8;

#[repr(align(16))]
struct SharedBuffer<const N: usize>(UnsafeCell<[u8; N]>);

// SAFETY: Each wasm instance is built without shared memory and exports only
// synchronous functions. The JavaScript loader does not expose the instance,
// memory, or raw pointers, so calls cannot overlap or re-enter this storage.
#[allow(unsafe_code)]
unsafe impl<const N: usize> Sync for SharedBuffer<N> {}

#[derive(Clone, Copy)]
struct ErrorState {
    code: u32,
    index: u32,
    required: u32,
}

struct SharedError(UnsafeCell<ErrorState>);

// SAFETY: The same single-instance, synchronous, non-shared-memory contract as
// `SharedBuffer` serializes all access to this diagnostic cell.
#[allow(unsafe_code)]
unsafe impl Sync for SharedError {}

static INPUT: SharedBuffer<MAX_INPUT> = SharedBuffer(UnsafeCell::new([0; MAX_INPUT]));
static OUTPUT: SharedBuffer<MAX_OUTPUT> = SharedBuffer(UnsafeCell::new([0; MAX_OUTPUT]));
static OUTPUT_HIGH_WATER: AtomicU32 = AtomicU32::new(0);
static LAST_ERROR: SharedError = SharedError(UnsafeCell::new(ErrorState {
    code: ERROR_NONE,
    index: NO_INDEX,
    required: 0,
}));

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_abi_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_artifact_posture() -> u32 {
    u32::from(cfg!(all(feature = "simd", target_feature = "simd128")))
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_input_ptr() -> *mut u8 {
    INPUT.0.get().cast::<u8>()
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_input_capacity() -> u32 {
    u32::try_from(MAX_INPUT).unwrap_or(u32::MAX)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_output_ptr() -> *mut u8 {
    OUTPUT.0.get().cast::<u8>()
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_output_capacity() -> u32 {
    u32::try_from(MAX_OUTPUT).unwrap_or(u32::MAX)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_last_error_code() -> u32 {
    // SAFETY: Export calls are serialized by the loader contract.
    unsafe { (*LAST_ERROR.0.get()).code }
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_last_error_index() -> u32 {
    // SAFETY: Export calls are serialized by the loader contract.
    unsafe { (*LAST_ERROR.0.get()).index }
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_last_error_required() -> u32 {
    // SAFETY: Export calls are serialized by the loader contract.
    unsafe { (*LAST_ERROR.0.get()).required }
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_encoded_length(input_len: u32, codec: u32) -> i32 {
    reset_error();
    let Ok(input_len) = checked_input_len(input_len) else {
        return -1;
    };
    let result = match codec {
        0 | 2 => base64_ng::checked_encoded_len(input_len, true),
        1 | 3 => base64_ng::checked_encoded_len(input_len, false),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    result.map_or_else(|| fail(ERROR_BACKEND, None, None), result_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_decoded_length(input_len: u32, codec: u32) -> i32 {
    reset_error();
    let Ok(input) = input_slice(input_len) else {
        return -1;
    };
    let result = match codec {
        0 => STANDARD.decoded_len(input),
        1 => STANDARD_NO_PAD.decoded_len(input),
        2 => URL_SAFE.decoded_len(input),
        3 => URL_SAFE_NO_PAD.decoded_len(input),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    result.map_or_else(map_decode_error, result_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_encode(input_len: u32, codec: u32) -> i32 {
    reset_error();
    let Ok(input) = input_slice(input_len) else {
        return -1;
    };
    let output = output_slice();
    let result = match codec {
        0 => STANDARD.encode_slice(input, output),
        1 => STANDARD_NO_PAD.encode_slice(input, output),
        2 => URL_SAFE.encode_slice(input, output),
        3 => URL_SAFE_NO_PAD.encode_slice(input, output),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    result.map_or_else(map_encode_error, written_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_decode(input_len: u32, codec: u32) -> i32 {
    reset_error();
    let Ok(input) = input_slice(input_len) else {
        return -1;
    };
    let validation = match codec {
        0 => STANDARD.validate_result(input),
        1 => STANDARD_NO_PAD.validate_result(input),
        2 => URL_SAFE.validate_result(input),
        3 => URL_SAFE_NO_PAD.validate_result(input),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    if let Err(error) = validation {
        return map_decode_error(error);
    }
    let required = match codec {
        0 => STANDARD.decoded_len(input),
        1 => STANDARD_NO_PAD.decoded_len(input),
        2 => URL_SAFE.decoded_len(input),
        3 => URL_SAFE_NO_PAD.decoded_len(input),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    let required = match required {
        Ok(required) => required,
        Err(error) => return map_decode_error(error),
    };
    record_output_high_water(required);
    let output = output_slice();
    let result = match codec {
        0 => STANDARD.decode_slice(input, output),
        1 => STANDARD_NO_PAD.decode_slice(input, output),
        2 => URL_SAFE.decode_slice(input, output),
        3 => URL_SAFE_NO_PAD.decode_slice(input, output),
        _ => return fail(ERROR_INVALID_CODEC, None, None),
    };
    result.map_or_else(map_decode_error, written_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_decode_forgiving(input_len: u32) -> i32 {
    reset_error();
    let Ok(input) = input_slice(input_len) else {
        return -1;
    };
    let Ok(text) = core::str::from_utf8(input) else {
        return fail(ERROR_INVALID_INPUT, None, None);
    };
    web::FORGIVING
        .decode_into(text, output_slice())
        .map_or_else(|_| fail(ERROR_INVALID_INPUT, None, None), written_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_decoded_length_forgiving(input_len: u32) -> i32 {
    reset_error();
    let Ok(input) = input_slice(input_len) else {
        return -1;
    };
    let Ok(text) = core::str::from_utf8(input) else {
        return fail(ERROR_INVALID_INPUT, None, None);
    };
    web::FORGIVING
        .decoded_len(text)
        .map_or_else(|_| fail(ERROR_INVALID_INPUT, None, None), result_length)
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_clear() {
    wipe(INPUT.0.get().cast(), MAX_INPUT);
    wipe(OUTPUT.0.get().cast(), MAX_OUTPUT);
    OUTPUT_HIGH_WATER.store(0, Ordering::Release);
    reset_error();
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_clear_used(input_used: u32, output_used: u32) {
    let input_len = usize::try_from(input_used)
        .unwrap_or(MAX_INPUT)
        .min(MAX_INPUT);
    let reported_output_len = usize::try_from(output_used)
        .unwrap_or(MAX_OUTPUT)
        .min(MAX_OUTPUT);
    let actual_output_len = usize::try_from(OUTPUT_HIGH_WATER.swap(0, Ordering::AcqRel))
        .unwrap_or(MAX_OUTPUT)
        .min(MAX_OUTPUT);
    let output_len = reported_output_len.max(actual_output_len);
    wipe(INPUT.0.get().cast(), input_len);
    wipe(OUTPUT.0.get().cast(), output_len);
    reset_error();
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn base64_ng_self_test() -> i32 {
    let input = b"base64-ng wasm";
    // SAFETY: The source and fixed input buffer are disjoint and large enough.
    unsafe { core::ptr::copy_nonoverlapping(input.as_ptr(), INPUT.0.get().cast(), input.len()) };
    let encoded = base64_ng_encode(u32::try_from(input.len()).unwrap_or(0), 0);
    if encoded <= 0 {
        return 1;
    }
    let encoded_len = usize::try_from(encoded).unwrap_or(0);
    // SAFETY: Both buffers are disjoint and `encoded_len` came from the codec.
    unsafe {
        core::ptr::copy_nonoverlapping(
            OUTPUT.0.get().cast::<u8>(),
            INPUT.0.get().cast::<u8>(),
            encoded_len,
        );
    }
    let decoded = base64_ng_decode(u32::try_from(encoded_len).unwrap_or(0), 0);
    if decoded != i32::try_from(input.len()).unwrap_or(-1) {
        return 2;
    }
    // SAFETY: The decoded length equals `input.len()` and OUTPUT is initialized.
    let actual = unsafe { core::slice::from_raw_parts(OUTPUT.0.get().cast::<u8>(), input.len()) };
    if actual != input {
        return 3;
    }
    base64_ng_clear();
    0
}

fn checked_input_len(input_len: u32) -> Result<usize, ()> {
    let length = usize::try_from(input_len).map_err(|_| ())?;
    if length > MAX_INPUT {
        fail(ERROR_INPUT_LIMIT, None, Some(MAX_INPUT));
        return Err(());
    }
    Ok(length)
}

#[allow(unsafe_code)]
fn input_slice(input_len: u32) -> Result<&'static [u8], ()> {
    let length = checked_input_len(input_len)?;
    // SAFETY: The checked length is within INPUT and calls are serialized.
    Ok(unsafe { core::slice::from_raw_parts(INPUT.0.get().cast::<u8>(), length) })
}

#[allow(unsafe_code)]
fn output_slice() -> &'static mut [u8] {
    // SAFETY: Export calls are serialized and OUTPUT is distinct from INPUT.
    unsafe { core::slice::from_raw_parts_mut(OUTPUT.0.get().cast::<u8>(), MAX_OUTPUT) }
}

fn result_length(length: usize) -> i32 {
    i32::try_from(length).unwrap_or_else(|_| fail(ERROR_BACKEND, None, None))
}

fn written_length(length: usize) -> i32 {
    let result = result_length(length);
    if result >= 0 {
        record_output_high_water(length);
    }
    result
}

fn record_output_high_water(length: usize) {
    if let Ok(length) = u32::try_from(length) {
        OUTPUT_HIGH_WATER.fetch_max(length, Ordering::AcqRel);
    }
}

fn map_encode_error(error: EncodeError) -> i32 {
    match error {
        EncodeError::OutputTooSmall { required, .. } => {
            fail(ERROR_OUTPUT_CAPACITY, None, Some(required))
        }
        _ => fail(ERROR_BACKEND, None, None),
    }
}

fn map_decode_error(error: DecodeError) -> i32 {
    match error {
        DecodeError::InvalidInput => fail(ERROR_INVALID_INPUT, None, None),
        DecodeError::InvalidLength => fail(ERROR_INVALID_LENGTH, None, None),
        DecodeError::InvalidByte { index, .. } => fail(ERROR_INVALID_BYTE, Some(index), None),
        DecodeError::InvalidPadding { index } => fail(ERROR_INVALID_PADDING, Some(index), None),
        DecodeError::InvalidLineWrap { index } => fail(ERROR_INVALID_INPUT, Some(index), None),
        DecodeError::OutputTooSmall { required, .. }
        | DecodeError::StagingTooSmall { required, .. } => {
            fail(ERROR_OUTPUT_CAPACITY, None, Some(required))
        }
    }
}

#[allow(unsafe_code)]
fn fail(code: u32, index: Option<usize>, required: Option<usize>) -> i32 {
    // SAFETY: Export calls are serialized by the loader contract.
    unsafe {
        *LAST_ERROR.0.get() = ErrorState {
            code,
            index: index
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(NO_INDEX),
            required: required
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0),
        };
    }
    -1
}

#[allow(unsafe_code)]
fn reset_error() {
    // SAFETY: Export calls are serialized by the loader contract.
    unsafe {
        *LAST_ERROR.0.get() = ErrorState {
            code: ERROR_NONE,
            index: NO_INDEX,
            required: 0,
        };
    }
}

#[allow(unsafe_code)]
fn wipe(pointer: *mut u8, length: usize) {
    for offset in 0..length {
        // SAFETY: Both static buffers provide `length` writable bytes and calls
        // are serialized. Volatile stores resist ordinary dead-store removal.
        unsafe { core::ptr::write_volatile(pointer.add(offset), 0) };
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
