#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    #[cfg(fuzzing)]
    base64_ng_fuzz::incremental::exercise(data);
    #[cfg(not(fuzzing))]
    {
        let _ = data;
        panic!("v2_incremental requires cargo-fuzz cfg(fuzzing) for its scalar oracle");
    }
});
