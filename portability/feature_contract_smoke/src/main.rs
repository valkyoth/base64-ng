use core::mem::{align_of, needs_drop, size_of};
use core::panic::{RefUnwindSafe, UnwindSafe};

use base64_ng::{
    DecodedBuffer, EncodedBuffer, Engine, LineEnding, LineWrap, Profile, STANDARD, Standard,
};

const ENGINE: Engine<Standard, true> = Engine::new();
const PROFILE: Profile<Standard, true> = Profile::new(ENGINE, None);
const WRAP: LineWrap = LineWrap::new(64, LineEnding::Lf);

fn assert_auto_traits<T: Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}

fn assert_behavior() {
    let mut encoded = [0u8; 8];
    let written = STANDARD.encode_slice(b"hello", &mut encoded).unwrap();
    assert_eq!(&encoded[..written], b"aGVsbG8=");

    let mut decoded = [0u8; 5];
    let written = STANDARD.decode_slice(&encoded, &mut decoded).unwrap();
    assert_eq!(&decoded[..written], b"hello");

    let _ = ENGINE;
    let _ = PROFILE;
    let _ = WRAP;
}

fn main() {
    assert_auto_traits::<Engine<Standard, true>>();
    assert_auto_traits::<Profile<Standard, true>>();
    assert_auto_traits::<EncodedBuffer<64>>();
    assert_auto_traits::<DecodedBuffer<64>>();
    assert_auto_traits::<LineWrap>();
    assert_behavior();

    println!(
        "{}:{}:{};{}:{}:{};{}:{}:{};{}:{}:{};{}:{}:{}",
        size_of::<Engine<Standard, true>>(),
        align_of::<Engine<Standard, true>>(),
        needs_drop::<Engine<Standard, true>>(),
        size_of::<Profile<Standard, true>>(),
        align_of::<Profile<Standard, true>>(),
        needs_drop::<Profile<Standard, true>>(),
        size_of::<EncodedBuffer<64>>(),
        align_of::<EncodedBuffer<64>>(),
        needs_drop::<EncodedBuffer<64>>(),
        size_of::<DecodedBuffer<64>>(),
        align_of::<DecodedBuffer<64>>(),
        needs_drop::<DecodedBuffer<64>>(),
        size_of::<LineWrap>(),
        align_of::<LineWrap>(),
        needs_drop::<LineWrap>(),
    );
}
