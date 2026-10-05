//! Transactional ordinary policy/report composition for static capabilities.

use super::StaticBackendToken;
use crate::{DecodeReport, DecodeValidation, OneShotError};

impl StaticBackendToken {
    /// Decodes strict Standard Base64 transactionally with an execution report.
    ///
    /// Unlike the historical [`Self::decode_standard`], every returned error
    /// leaves the destination unchanged and uses canonical [`OneShotError`]
    /// diagnostics. Both policies preserve checked output comparison.
    ///
    /// Only this token's backend may accelerate the call. Invalid tokens,
    /// unavailable ordinary kernels, and short inputs use scalar processing.
    /// This includes AVX-512 (not admitted for ordinary vector validation) and
    /// deployment-only support not recognized by the safe ordinary wrappers.
    /// Existing direct-token methods retain their existing backend support.
    pub fn decode_standard_with_report<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<(usize, DecodeReport), OneShotError> {
        let settings = if PAD {
            crate::STRICT_STANDARD_PADDED.settings()
        } else {
            crate::STRICT_STANDARD_UNPADDED.settings()
        };
        self.decode_reported(settings, input, output, validation)
    }

    /// Like [`Self::decode_standard_with_report`] for strict URL-safe Base64.
    pub fn decode_url_safe_with_report<const PAD: bool>(
        &self,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<(usize, DecodeReport), OneShotError> {
        let settings = if PAD {
            crate::STRICT_URL_SAFE_PADDED.settings()
        } else {
            crate::STRICT_URL_SAFE_UNPADDED.settings()
        };
        self.decode_reported(settings, input, output, validation)
    }

    fn decode_reported(
        &self,
        settings: crate::CodecSettings,
        input: &[u8],
        output: &mut [u8],
        validation: DecodeValidation,
    ) -> Result<(usize, DecodeReport), OneShotError> {
        use crate::v2::ordinary_decode::{decode_reported, static_backend};
        decode_reported(
            settings,
            input,
            output,
            validation,
            crate::v2::ordinary_decode::Selection::Static(static_backend(self, input.len())),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecodeValidator, runtime::Backend};

    #[test]
    fn composition_static_policies_preserve_grammar_and_token_backend() {
        let _ = crate::initialize_backends();
        for backend in [
            Backend::Ssse3Sse41,
            Backend::Avx2,
            Backend::Avx512Vbmi,
            Backend::Neon,
            Backend::WasmSimd128,
        ] {
            let Some(token) = StaticBackendToken::admit(backend, false) else {
                continue;
            };
            for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
                let input = [b'A'; 4096];
                let mut output = [0xa5; 3080];
                let (len, report) = token
                    .decode_standard_with_report::<true>(&input, &mut output, policy)
                    .unwrap();
                assert_eq!(len, 3072);
                assert_eq!(output[..3072], [0; 3072]);
                assert_eq!(output[3072..], [0xa5; 8]);
                assert!(
                    report
                        .selected_backend()
                        .is_none_or(|selected| selected == token.backend())
                );
                assert!(
                    report.output_backend() == Backend::Scalar
                        || report.output_backend() == token.backend()
                );
                if policy == DecodeValidation::ScalarReference {
                    assert_eq!(report.validator(), DecodeValidator::ScalarReference);
                }
                if report.output_backend() != Backend::Scalar {
                    assert_eq!(report.checked_output(), cfg!(feature = "checked-backend"));
                }
                for input in [b"Zh==".as_slice(), b"Zm9!", b"AAAA ", b"____"] {
                    let mut output = [0xa5; 16];
                    assert!(
                        token
                            .decode_standard_with_report::<true>(input, &mut output, policy)
                            .is_err()
                    );
                    assert_eq!(output, [0xa5; 16]);
                }
                let mut output = [0xa5; 4];
                assert_eq!(
                    token
                        .decode_url_safe_with_report::<false>(b"__8", &mut output, policy)
                        .unwrap()
                        .0,
                    2
                );
                assert_eq!(output, [255, 255, 0xa5, 0xa5]);
                assert!(
                    token
                        .decode_url_safe_with_report::<true>(b"__8", &mut output, policy)
                        .is_err()
                );
                assert!(
                    token
                        .decode_standard_with_report::<false>(b"AA==", &mut output, policy)
                        .is_err()
                );
                let mut short = [0xa5; 3];
                assert!(
                    token
                        .decode_standard_with_report::<true>(&input, &mut short, policy)
                        .is_err()
                );
                assert_eq!(short, [0xa5; 3]);
            }
            let stale = StaticBackendToken {
                generation: token.generation.wrapping_add(1),
                ..token
            };
            assert!(!stale.is_valid());
            let mut output = [0xa5; 3072];
            let (_, report) = stale
                .decode_standard_with_report::<true>(
                    &[b'A'; 4096],
                    &mut output,
                    DecodeValidation::Auto,
                )
                .unwrap();
            assert_eq!(report.selected_backend(), None);
            assert_eq!(report.output_backend(), Backend::Scalar);
            assert_eq!(output, [0; 3072]);
        }
    }

    #[test]
    fn composition_invalid_static_token_never_selects_another_backend() {
        // An invalid token is a unit-test fixture, not a public constructor.
        let token = StaticBackendToken {
            backend: Backend::Sve,
            generation: 0,
            _thread_bound: core::marker::PhantomData,
        };
        assert!(!token.is_valid());
        let mut output = [0xa5; 3072];
        for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
            let (_, report) = token
                .decode_standard_with_report::<true>(&[b'A'; 4096], &mut output, policy)
                .unwrap();
            assert_eq!(report.output_backend(), Backend::Scalar);
            assert_eq!(report.selected_backend(), None);
            assert_eq!(output, [0; 3072]);
        }
    }
}
