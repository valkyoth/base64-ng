//! Retained grammar proof, not a retained authorization to execute SIMD.

use super::{
    BackendFault, CodecSettings, DecodeValidation, OneShotError, Preflight, Prepared, prepare,
};
use crate::{BackendHealthSnapshot, BackendHealthState, runtime::OperationKind};

pub(crate) struct ValidatedInput<'a> {
    proof: Preflight<'a, Prepared>,
    validation: DecodeValidation,
    validator_health: Option<BackendHealthSnapshot>,
}

impl<'a> ValidatedInput<'a> {
    pub(crate) fn new(
        settings: CodecSettings,
        input: &'a [u8],
        validation: DecodeValidation,
    ) -> Result<Self, OneShotError> {
        let mut proof = prepare(settings, input, validation)?;
        let mut validator_health = None;
        if validation == DecodeValidation::Auto
            && let Some(backend) = proof.configuration().backend
        {
            let health =
                super::super::backend_health::snapshot(OperationKind::StrictDecode, backend);
            if health.state == BackendHealthState::Healthy && health.generation != usize::MAX {
                validator_health = Some(health);
            } else {
                // Never retain a classifier result already known to be suspect.
                proof = prepare(settings, input, DecodeValidation::ScalarReference)?;
            }
        }
        Ok(Self {
            proof,
            validation,
            validator_health,
        })
    }

    pub(crate) const fn input(&self) -> &'a [u8] {
        self.proof.input()
    }
    pub(crate) const fn len(&self) -> usize {
        self.proof.len()
    }
    pub(crate) const fn validation(&self) -> DecodeValidation {
        self.validation
    }
    pub(crate) fn settings(&self) -> CodecSettings {
        self.proof.configuration().settings
    }

    pub(crate) fn writer_proof(&self) -> Result<Preflight<'_, Prepared>, OneShotError> {
        let changed = self.validator_health.is_some_and(|previous| {
            let current =
                super::super::backend_health::snapshot(previous.operation, previous.backend);
            current != previous || current.state != BackendHealthState::Healthy
        });
        if self.validation == DecodeValidation::ScalarReference || changed {
            let proof = prepare(
                self.settings(),
                self.input(),
                DecodeValidation::ScalarReference,
            )?;
            if proof.len() != self.len() {
                return Err(OneShotError::Backend(BackendFault::ImpossibleState));
            }
            Ok(proof)
        } else {
            Ok(self.proof.reborrow())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Family, write};
    use super::*;
    use crate::{
        STRICT_STANDARD_PADDED as CODEC, decode_validation::observation, runtime::Backend,
    };

    #[test]
    fn borrowed_view_changed_generation_revalidates_before_writing() {
        let mut retained =
            ValidatedInput::new(CODEC.settings(), b"Zm9v", DecodeValidation::Auto).unwrap();
        let mut old = super::super::super::backend_health::snapshot(
            OperationKind::StrictDecode,
            Backend::Avx2,
        );
        old.generation = old.generation.wrapping_add(1);
        retained.validator_health = Some(old);
        let before = observation::calls();
        let proof = retained.writer_proof().unwrap();
        assert_eq!(observation::calls(), before + 1);
        let mut output = [0xa5; 4];
        assert_eq!(write(proof, &mut output), Ok(3));
        assert_eq!(output, [b'f', b'o', b'o', 0xa5]);
    }

    #[test]
    fn borrowed_view_reference_length_disagreement_fails_closed() {
        // Test-only inconsistent cached measurement, not a public constructor.
        let retained = ValidatedInput {
            proof: Preflight::validate(
                b"Zm9v",
                Prepared {
                    settings: CODEC.settings(),
                    backend: None,
                    family: Some(Family::Standard),
                },
                |_, _| Ok::<_, OneShotError>(1),
            )
            .unwrap(),
            validation: DecodeValidation::ScalarReference,
            validator_health: None,
        };
        assert!(matches!(
            retained.writer_proof(),
            Err(OneShotError::Backend(BackendFault::ImpossibleState))
        ));
    }
}
