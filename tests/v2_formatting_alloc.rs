#![allow(unsafe_code)]

use core::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
    fmt::Write,
};
use std::alloc::System;

use base64_ng::{
    CodecBuilder, DecodePadding, EncodePadding, STRICT_STANDARD_PADDED, ValidatedAlphabet,
};

struct CountingAllocator;

std::thread_local! {
    static COUNTER: Cell<(bool, usize)> = const { Cell::new((false, 0)) };
}

fn record_allocation() {
    // Only observe this thread; libtest/platform activity on other threads is
    // not part of the synchronous formatter contract. TLS initialization is const.
    let _ = COUNTER.try_with(|counter| {
        let (enabled, count) = counter.get();
        if enabled {
            counter.set((true, count + 1));
        }
    });
}

fn measure(action: impl FnOnce()) -> usize {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTER.with(|counter| counter.set((false, 0)));
        }
    }
    COUNTER.with(|counter| counter.set((true, 0)));
    let _reset = Reset;
    action();
    COUNTER.with(|counter| counter.replace((false, 0)).1)
}

// SAFETY: Every operation delegates to `System` with the original pointer and
// layout. The thread-local counter does not change allocator semantics.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: Delegates the unchanged valid allocator request.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: Delegates the unchanged pointer and allocation layout.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
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

    // Keep platform and formatter one-time initialization outside the measured
    // region, then prove every steady-state library path remains allocation-free.
    let mut built_in_warmup = StackWriter::new();
    STRICT_STANDARD_PADDED
        .encode_to_fmt(input, &mut built_in_warmup)
        .unwrap();
    let mut display_warmup = StackWriter::new();
    let display = STRICT_STANDARD_PADDED.display(input).unwrap();
    write!(&mut display_warmup, "{display}").unwrap();
    let mut custom_warmup = StackWriter::new();
    runtime.encode_to_fmt(input, &mut custom_warmup).unwrap();
    assert!(!built_in_warmup.as_bytes().is_empty());
    assert!(!display_warmup.as_bytes().is_empty());
    assert!(!custom_warmup.as_bytes().is_empty());

    let mut built_in = StackWriter::new();
    let mut custom = StackWriter::new();
    let allocations = measure(|| {
        let display = STRICT_STANDARD_PADDED.display(input).unwrap();
        write!(&mut built_in, "{display}").unwrap();
        runtime.encode_to_fmt(input, &mut custom).unwrap();
    });

    assert_eq!(allocations, 0);
    assert!(!built_in.as_bytes().is_empty());
    assert!(!custom.as_bytes().is_empty());
}

#[test]
fn strict_decode_validation_and_writing_allocate_zero_heap_blocks() {
    let input = [b'A'; 4096];
    let mut output = [0xff; 3072];
    let _ = base64_ng::initialize_backends();
    STRICT_STANDARD_PADDED
        .decode_into(&input, &mut output)
        .unwrap();
    for policy in [
        base64_ng::DecodeValidation::Auto,
        base64_ng::DecodeValidation::ScalarReference,
    ] {
        assert_eq!(
            measure(|| {
                assert_eq!(
                    STRICT_STANDARD_PADDED.decoded_len_with_validation(&input, policy),
                    Ok(3072)
                );
                assert_eq!(
                    STRICT_STANDARD_PADDED.decode_into_with_validation(&input, &mut output, policy),
                    Ok(3072)
                );
                assert_eq!(
                    base64_ng::STANDARD.decode_slice_with_validation(&input, &mut output, policy),
                    Ok(3072)
                );
            }),
            0
        );
    }
    assert_eq!(output, [0; 3072]);
}

#[test]
fn counter_detects_allocations_and_reallocations() {
    let mut bytes = Vec::<u8>::new();
    assert!(measure(|| bytes.reserve_exact(std::hint::black_box(16))) > 0);
    let capacity = bytes.capacity();
    assert!(measure(|| bytes.reserve_exact(std::hint::black_box(capacity + 1))) > 0);
    std::hint::black_box(bytes);
}

#[test]
fn counter_excludes_background_thread_allocations() {
    use std::sync::atomic::{AtomicU8, Ordering};
    let progress = AtomicU8::new(0);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            while progress.load(Ordering::Acquire) != 1 {
                std::hint::spin_loop();
            }
            let allocations = measure(|| {
                std::hint::black_box(vec![0_u8; std::hint::black_box(128)]);
            });
            progress.store(2, Ordering::Release);
            allocations
        });
        let allocations = measure(|| {
            progress.store(1, Ordering::Release);
            while progress.load(Ordering::Acquire) != 2 {
                std::hint::spin_loop();
            }
        });
        assert_eq!(allocations, 0);
        assert!(worker.join().unwrap() > 0);
    });
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
