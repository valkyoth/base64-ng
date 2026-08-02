#![allow(missing_docs)]
#![allow(unsafe_code)]

use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::alloc::System;

use base64_ng::STRICT_STANDARD_PADDED;
use base64_ng_bytes::Base64BytesExt;
use bytes::{Buf, Bytes};

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_SIZES: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

fn record_allocation(size: usize) {
    if COUNTING.load(Ordering::Relaxed) {
        let index = ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        if let Some(slot) = ALLOCATION_SIZES.get(index) {
            slot.store(size, Ordering::Relaxed);
        }
    }
}

// SAFETY: Every operation delegates to System with the unchanged pointer and
// layout. Atomics observe allocation calls without changing their semantics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation(layout.size());
        // SAFETY: Delegates the original valid allocator request.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: Delegates the original allocation pointer and layout.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation(size);
        // SAFETY: Delegates the original allocation and requested new size.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn fragmented_transaction_avoids_second_payload_allocation() {
    let first = Bytes::from_static(b"fragmented ");
    let second = Bytes::from_static(b"input without ");
    let third = Bytes::from_static(b"coalescing");
    let input = first.chain(second).chain(third);

    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    let encoded = STRICT_STANDARD_PADDED.encode_buf(input).unwrap();
    COUNTING.store(false, Ordering::Relaxed);

    assert_eq!(
        (ALLOCATIONS.load(Ordering::Relaxed), allocation_sizes()),
        (1, [48, 0, 0, 0])
    );
    assert_eq!(
        &encoded[..],
        b"ZnJhZ21lbnRlZCBpbnB1dCB3aXRob3V0IGNvYWxlc2Npbmc="
    );

    ALLOCATIONS.store(0, Ordering::Relaxed);
    clear_allocation_sizes();
    COUNTING.store(true, Ordering::Relaxed);
    let decoded = STRICT_STANDARD_PADDED.decode_buf(encoded).unwrap();
    COUNTING.store(false, Ordering::Relaxed);

    let allocation_count = ALLOCATIONS.load(Ordering::Relaxed);
    let sizes = allocation_sizes();
    assert!((1..=2).contains(&allocation_count));
    assert_eq!(sizes[0], 48);
    if allocation_count == 2 {
        assert!(
            sizes[1] < sizes[0],
            "unexpected payload-sized copy: {sizes:?}"
        );
    }
    assert_eq!(decoded, b"fragmented input without coalescing"[..]);
}

fn clear_allocation_sizes() {
    for size in &ALLOCATION_SIZES {
        size.store(0, Ordering::Relaxed);
    }
}

fn allocation_sizes() -> [usize; 4] {
    core::array::from_fn(|index| ALLOCATION_SIZES[index].load(Ordering::Relaxed))
}
