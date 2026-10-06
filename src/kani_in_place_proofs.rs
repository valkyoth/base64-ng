//! Bounded cursor proofs for the 2.0 finite-buffer in-place kernels.

use crate::v2::{encoded_tail_len, quantum_decoded_len, tail_decoded_len};

#[kani::proof]
#[kani::unwind(12)]
fn reverse_in_place_encode_never_overwrites_unread_input() {
    let input_len = usize::from(kani::any::<u8>() % 25);
    let padded = kani::any::<bool>();
    let complete = input_len / 3 * 4;
    let remainder = input_len % 3;
    let tail_len = encoded_tail_len(remainder, padded);
    let mut read = input_len;
    let mut write = complete + tail_len;

    if remainder != 0 {
        read -= remainder;
        write -= tail_len;
        assert!(write >= read);
    }
    while read != 0 {
        read -= 3;
        write -= 4;
        assert!(write >= read);
    }
    assert!(read == 0);
    assert!(write == 0);
}

#[kani::proof]
#[kani::unwind(12)]
fn forward_in_place_decode_writes_only_consumed_prefixes() {
    let complete_quanta = usize::from(kani::any::<u8>() % 9);
    let tail = usize::from(kani::any::<u8>() % 4);
    let mut read = 0usize;
    let mut write = 0usize;

    for _ in 0..complete_quanta {
        let third_is_padding = kani::any::<bool>();
        let fourth_is_padding = kani::any::<bool>();
        read += 4;
        write += quantum_decoded_len(third_is_padding, fourth_is_padding);
        assert!(write <= read);
    }
    let produced = tail_decoded_len(tail);
    if produced != 0 {
        read += tail;
        write += produced;
        assert!(write <= read);
    }
}

// Inductive arithmetic model of ordinary_decode/in_place.rs, not a proof of
// its memory operations or SIMD instructions. No allocation bounds the input.
#[kani::proof]
fn bulk_in_place_chunk_geometry_preserves_unread_suffix() {
    let len = kani::any::<usize>();
    let body = len.saturating_sub(1) / 4 * 4;
    let read = kani::any::<usize>();
    kani::assume(read < body && read % 4 == 0);
    let chunk = crate::v2::ordinary_decode::in_place::input_chunk_for_proof();
    let count = (body - read).min(chunk);
    let write = read / 4 * 3;
    let produced = count / 4 * 3;
    assert!(count > 0 && count <= chunk && count % 4 == 0);
    assert!(read + count <= body && body < len);
    assert!(write <= read && write + produced <= read + count);
    assert!(write + produced == (read + count) / 4 * 3);
    let consumed = kani::any::<usize>();
    kani::assume(consumed <= count && consumed % 4 == 0);
    assert!(consumed / 4 * 3 <= produced);
    assert!((count - consumed) / 4 * 3 == produced - consumed / 4 * 3);
    assert!(len - body <= 4);
}
