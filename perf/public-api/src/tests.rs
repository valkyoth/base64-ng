use super::*;

#[test]
fn benchmark_operations_reject_real_malformed_input() {
    for operation in operations::names() {
        for position in [0, 31, 32, 63, 64, 683] {
            let mut input = oracle::encode(oracle::Profile::StandardPadded, &vec![0x5a; 512]);
            input[position] = b'!';
            let mut work = Work::new(748);
            let result = operations::apply(
                &base64_ng::STRICT_STANDARD_PADDED,
                base64_ng::STANDARD,
                operation,
                false,
                &input,
                &mut work,
                7,
            );
            assert!(result.is_err(), "{operation} accepted malformed input");
            if matches!(operation, "canonical" | "validate") {
                assert!(work.output.iter().all(|byte| *byte == 0xa5));
            }
        }
    }
}

#[test]
fn public_operations_match_the_independent_oracle() {
    macro_rules! exercise {
        ($codec:expr, $engine:expr, $profile:expr) => {
            for size in [0, 1, 2, 3, 15, 16, 17, 31, 32, 33, 63, 64, 65, 512] {
                let raw = input(size, "random");
                let encoded = oracle::encode($profile, &raw);
                for operation in operations::names() {
                    for encode in [false, true] {
                        if encode && operation == "validate" {
                            continue;
                        }
                        for fragment in [1, 7, 1024] {
                            let mut work = Work::new(encoded.len() + 64);
                            let (input, expected) = if encode {
                                (&raw, &encoded)
                            } else {
                                (&encoded, &raw)
                            };
                            let result = operations::apply(
                                &$codec, $engine, operation, encode, input, &mut work, fragment,
                            );
                            verify(result, &work, operation, encode, expected, false);
                        }
                    }
                }
            }
        };
    }
    exercise!(
        base64_ng::STRICT_STANDARD_PADDED,
        base64_ng::STANDARD,
        oracle::Profile::StandardPadded
    );
    exercise!(
        base64_ng::STRICT_STANDARD_UNPADDED,
        base64_ng::STANDARD_NO_PAD,
        oracle::Profile::StandardUnpadded
    );
    exercise!(
        base64_ng::STRICT_URL_SAFE_PADDED,
        base64_ng::URL_SAFE,
        oracle::Profile::UrlSafePadded
    );
    exercise!(
        base64_ng::STRICT_URL_SAFE_UNPADDED,
        base64_ng::URL_SAFE_NO_PAD,
        oracle::Profile::UrlSafeUnpadded
    );
}

#[test]
#[should_panic(expected = "wrong benchmark output")]
fn wrong_output_is_not_a_valid_benchmark() {
    verify(Ok(3), &Work::new(16), "canonical", false, b"foo", false);
}

#[test]
#[should_panic(expected = "valid benchmark operation failed")]
fn error_only_call_is_not_a_valid_benchmark() {
    verify(Err(()), &Work::new(16), "canonical", false, b"foo", false);
}

#[test]
#[should_panic(expected = "malformed input accepted")]
fn successful_malformed_decode_is_not_a_rejection_benchmark() {
    verify(Ok(0), &Work::new(16), "canonical", false, b"", true);
}
