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
