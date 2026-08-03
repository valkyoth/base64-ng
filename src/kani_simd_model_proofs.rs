//! Portable block models for architecture-specific SIMD admission.
//!
//! These proofs cover arithmetic, masks, and wrapper bounds. They do not
//! execute or prove target intrinsics, inline assembly, or register cleanup.

fn encode_indices(input: [u8; 12]) -> [u8; 16] {
    let mut output = [0u8; 16];
    let mut group = 0;
    while group < 4 {
        let read = group * 3;
        let write = group * 4;
        let first = input[read];
        let second = input[read + 1];
        let third = input[read + 2];
        output[write] = first >> 2;
        output[write + 1] = ((first & 3) << 4) | (second >> 4);
        output[write + 2] = ((second & 15) << 2) | (third >> 6);
        output[write + 3] = third & 63;
        group += 1;
    }
    output
}

fn decode_indices(input: [u8; 16]) -> [u8; 12] {
    let mut output = [0u8; 12];
    let mut group = 0;
    while group < 4 {
        let read = group * 4;
        let write = group * 3;
        output[write] = (input[read] << 2) | (input[read + 1] >> 4);
        output[write + 1] = (input[read + 1] << 4) | (input[read + 2] >> 2);
        output[write + 2] = (input[read + 2] << 6) | input[read + 3];
        group += 1;
    }
    output
}

fn classify_ascii(byte: u8, url_safe: bool) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' if !url_safe => Some(62),
        b'/' if !url_safe => Some(63),
        b'-' if url_safe => Some(62),
        b'_' if url_safe => Some(63),
        _ => None,
    }
}

fn encode_ascii(value: u8, url_safe: bool) -> u8 {
    match value {
        0..=25 => b'A' + value,
        26..=51 => b'a' + (value - 26),
        52..=61 => b'0' + (value - 52),
        62 if url_safe => b'-',
        63 if url_safe => b'_',
        62 => b'+',
        63 => b'/',
        _ => 0,
    }
}

#[kani::proof]
#[kani::unwind(17)]
fn portable_simd_block_transform_round_trips_all_input_bits() {
    let input = kani::any::<[u8; 12]>();
    let indices = encode_indices(input);
    assert!(indices.iter().all(|value| *value < 64));
    assert!(decode_indices(indices) == input);
}

#[kani::proof]
fn portable_simd_ascii_classifier_is_bijective_per_lane() {
    let byte = kani::any::<u8>();
    let url_safe = kani::any::<bool>();
    if let Some(value) = classify_ascii(byte, url_safe) {
        assert!(value < 64);
        assert!(encode_ascii(value, url_safe) == byte);
    }

    let value = kani::any::<u8>() & 63;
    let encoded = encode_ascii(value, url_safe);
    assert!(classify_ascii(encoded, url_safe) == Some(value));
}

#[kani::proof]
#[kani::unwind(17)]
fn portable_simd_validity_mask_requires_every_active_lane() {
    let valid = kani::any::<[bool; 16]>();
    let mut mask = 0u16;
    let mut lane = 0;
    while lane < valid.len() {
        if valid[lane] {
            mask |= 1u16 << lane;
        }
        lane += 1;
    }
    assert!((mask == u16::MAX) == valid.iter().all(|lane_valid| *lane_valid));
}

fn prove_wrapper_bounds(input_len: usize, input_block: usize, output_block: usize) {
    let blocks = input_len / input_block;
    let read = blocks * input_block;
    let write = blocks * output_block;

    assert!(read <= input_len);
    assert!(read.is_multiple_of(input_block));
    assert!(write == read / input_block * output_block);
    assert!(input_len - read < input_block);
}

#[kani::proof]
fn portable_simd_wrapper_cursors_are_bounded_for_every_backend_width() {
    let input_len = usize::from(kani::any::<u8>());
    prove_wrapper_bounds(input_len, 12, 16);
    prove_wrapper_bounds(input_len, 24, 32);
    prove_wrapper_bounds(input_len, 48, 64);

    let encoded_len = usize::from(kani::any::<u8>());
    prove_wrapper_bounds(encoded_len, 16, 12);
    prove_wrapper_bounds(encoded_len, 32, 24);
    prove_wrapper_bounds(encoded_len, 64, 48);
}

#[derive(Clone, Copy)]
struct InitializedOutputModel {
    initialized: usize,
    visible: usize,
    capacity: usize,
}

impl InitializedOutputModel {
    const fn write(self, count: usize) -> Option<Self> {
        let Some(initialized) = self.initialized.checked_add(count) else {
            return None;
        };
        if initialized > self.capacity {
            return None;
        }
        Some(Self {
            initialized,
            ..self
        })
    }

    const fn commit(self, count: usize) -> Option<Self> {
        if count > self.initialized || count > self.capacity {
            return None;
        }
        Some(Self {
            visible: count,
            ..self
        })
    }
}

#[kani::proof]
fn caller_visible_commit_never_exceeds_initialized_bytes() {
    let capacity = usize::from(kani::any::<u8>());
    let written = usize::from(kani::any::<u8>());
    let committed = usize::from(kani::any::<u8>());
    let initial = InitializedOutputModel {
        initialized: 0,
        visible: 0,
        capacity,
    };
    let Some(after_write) = initial.write(written) else {
        return;
    };
    if let Some(after_commit) = after_write.commit(committed) {
        assert!(after_commit.visible <= after_commit.initialized);
        assert!(after_commit.initialized <= after_commit.capacity);
    }
}
