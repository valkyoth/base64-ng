use alloc::vec::Vec;
use std::panic::{AssertUnwindSafe, catch_unwind, panic_any};

use crate::adapter_secret::WipingOwnedInput;

#[test]
fn owned_secret_input_wipes_complete_capacity_during_unwind() {
    let mut encoded = Vec::with_capacity(32);
    encoded.extend_from_slice(b"c2VjcmV0");

    let result = catch_unwind(AssertUnwindSafe(|| {
        let input = WipingOwnedInput::new(&mut encoded);
        assert_eq!(input.as_slice(), b"c2VjcmV0");
        panic_any("injected secret decode panic");
    }));

    assert!(result.is_err());
    assert_eq!(encoded.len(), encoded.capacity());
    assert!(encoded.iter().all(|byte| *byte == 0));
}
