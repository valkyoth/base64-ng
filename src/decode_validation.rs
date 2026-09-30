/// Validation strategy for ordinary, non-secret Base64 input.
///
/// This selects validation, not output-generation instructions or a weaker
/// grammar. Canonical strict Standard/URL-safe `Auto` uses a specialized portable
/// scalar validator; `ScalarReference` retains the original validator. Historical scalar
/// execution can combine validation with decoding; admitted SIMD execution
/// retains its full scalar prevalidation. Existing error precedence and output
/// mutation contracts are unchanged.
///
/// `checked-backend` remains additive: this option cannot disable redundant
/// output comparison, quarantine, or scalar retry. Static backend tokens and
/// deployment-policy checks remain separate and are not reconfigured here.
/// Neither variant is constant-time or suitable as a secret-decoding policy.
///
/// Secret decoders deliberately do not accept this option:
/// ```compile_fail
/// use base64_ng::{ct, DecodeValidation};
/// ct::STANDARD.decode_slice_with_validation(b"Zm9v", &mut [0; 3], DecodeValidation::Auto);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DecodeValidation {
    /// Use the admitted validation strategy, with scalar fallback.
    ///
    /// Canonical strict Standard/URL-safe settings use portable table validation.
    /// Other settings retain reference validation. Vector validation is not yet enabled.
    #[default]
    Auto,
    /// Retain complete scalar-reference grammar validation on every call.
    ///
    /// This does not force scalar output generation and is not a progressive
    /// decoding mode. Invalid input may still be rejected early.
    ScalarReference,
}

#[cfg(test)]
pub(crate) mod observation {
    extern crate std;
    std::thread_local! {
        static CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
        static FAST_CALLS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    }
    pub(crate) fn record() {
        CALLS.with(|calls| calls.set(calls.get() + 1));
    }
    pub(crate) fn calls() -> usize {
        CALLS.with(core::cell::Cell::get)
    }
    pub(crate) fn record_fast() {
        FAST_CALLS.with(|calls| calls.set(calls.get() + 1));
    }
    pub(crate) fn fast_calls() -> usize {
        FAST_CALLS.with(core::cell::Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeValidation, observation};

    #[test]
    fn selected_reference_reaches_scalar_implementations() {
        let input = [b'A'; 4096];
        let mut output = [0xff; 3072];
        // Initialize backend health before counting work from the actual call.
        crate::STANDARD.decode_slice(&input, &mut output).unwrap();
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            let before = observation::calls();
            assert_eq!(
                crate::STANDARD.decode_slice_with_validation(&input, &mut output, policy),
                Ok(3072)
            );
            assert!(observation::calls() > before);
            #[cfg(feature = "checked-backend")]
            if crate::decode_backend::last_test_execution()
                != crate::decode_backend::DecodeBackend::Scalar
            {
                // Four checked chunks each require scalar output comparison,
                // in addition to prevalidation of the complete input.
                assert!(observation::calls() >= before + 5);
            }
            assert_eq!(output, [0; 3072]);
            let before = observation::calls();
            let fast_before = observation::fast_calls();
            assert_eq!(
                crate::STRICT_STANDARD_PADDED.decode_into_with_validation(
                    &input,
                    &mut output,
                    policy
                ),
                Ok(3072)
            );
            if policy == DecodeValidation::ScalarReference {
                assert!(observation::calls() > before);
                assert_eq!(observation::fast_calls(), fast_before);
            } else {
                assert_eq!(observation::calls(), before);
                assert_eq!(observation::fast_calls(), fast_before + 1);
            }
            let before = observation::calls();
            assert_eq!(
                crate::STANDARD.validated_decoded_len_with_validation(&input, policy),
                Ok(3072)
            );
            assert!(observation::calls() > before);
        }
    }
}
