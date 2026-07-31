#![allow(unsafe_code)]

use core::{
    alloc::{GlobalAlloc, Layout},
    fmt::Write,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::alloc::System;

use base64_ng::{
    CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED, ValidatedAlphabet,
};

struct CountingAllocator;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: Every operation delegates to `System` with the original pointer and
// layout. The atomics observe calls without changing allocator semantics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: Delegates the unchanged valid allocator request.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: Delegates the unchanged pointer and allocation layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: Delegates the unchanged pointer/layout and requested size.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn display_and_formatter_paths_allocate_zero_heap_blocks() {
    let runtime = CodecBuilder::new(
        ValidatedAlphabet::new(
            *b"ZYXABCDEFGHIJKLMNOPQRSTUVWzyxabcdefghijklmnopqrstuvw0123456789-_",
        )
        .unwrap(),
    )
    .encode_padding(EncodePadding::Unpadded)
    .decode_padding(DecodePadding::Forbid)
    .build()
    .unwrap();
    let input = b"allocation-free formatter evidence";

    let mut warmup = StackWriter::new();
    STRICT_STANDARD_PADDED
        .encode_to_fmt(input, &mut warmup)
        .unwrap();

    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Relaxed);
    let mut built_in = StackWriter::new();
    let display = STRICT_STANDARD_PADDED.display(input).unwrap();
    write!(&mut built_in, "{display}").unwrap();
    let mut custom = StackWriter::new();
    runtime.encode_to_fmt(input, &mut custom).unwrap();
    COUNTING.store(false, Ordering::Relaxed);

    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
    assert!(!built_in.as_bytes().is_empty());
    assert!(!custom.as_bytes().is_empty());
}

struct StackWriter {
    bytes: [u8; 128],
    len: usize,
}

impl StackWriter {
    const fn new() -> Self {
        Self {
            bytes: [0; 128],
            len: 0,
        }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl core::fmt::Write for StackWriter {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let end = self.len.checked_add(text.len()).ok_or(core::fmt::Error)?;
        let output = self.bytes.get_mut(self.len..end).ok_or(core::fmt::Error)?;
        output.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
