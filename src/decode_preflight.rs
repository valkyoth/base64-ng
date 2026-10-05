//! Private validation-to-write boundary for ordinary decoding.

#[derive(Clone, Copy, Debug)]
pub(crate) enum Failure<E> {
    Input(E),
    Bounds,
    ClassifierDisagreement,
}

#[derive(Debug)]
pub(crate) struct CapacityError {
    pub(crate) required: usize,
    pub(crate) available: usize,
}

struct Layout {
    interior_input: usize,
    interior_output: usize,
    decoded_len: usize,
}

impl Layout {
    fn checked(input_len: usize, decoded_len: usize) -> Option<Self> {
        // Reserve the final quantum even when it is complete: only it may pad.
        let interior_quanta = input_len.saturating_sub(1) / 4;
        let interior_input = interior_quanta.checked_mul(4)?;
        let interior_output = interior_quanta.checked_mul(3)?;
        let tail_input = input_len.checked_sub(interior_input)?;
        let tail_output = decoded_len.checked_sub(interior_output)?;
        let valid = match tail_input {
            0 => tail_output == 0,
            2 => tail_output == 1,
            // Padding-indifferent codecs also accept a partially padded tail.
            3 => (1..=2).contains(&tail_output),
            4 => (1..=3).contains(&tail_output),
            _ => false,
        };
        valid.then_some(Self {
            interior_input,
            interior_output,
            decoded_len,
        })
    }
}

/// Owns the immutable configuration snapshot and borrows the exact validated
/// source. Not Clone/Copy; writing consumes it and accepts no replacement input
/// or settings. Only a completed validator can construct this value. The
/// internal callback must check the complete grammar, not only input shape.
pub(crate) struct Preflight<'a, C> {
    input: &'a [u8],
    configuration: C,
    layout: Layout,
}

impl<'a, C: Copy> Preflight<'a, C> {
    pub(crate) fn validate<E>(
        input: &'a [u8],
        configuration: C,
        validate: impl FnOnce(C, &[u8]) -> Result<usize, E>,
    ) -> Result<Self, Failure<E>> {
        let result = validate(configuration, input);
        Self::resolve(input, configuration, result, None)
    }

    fn resolve<E>(
        input: &'a [u8],
        configuration: C,
        reference: Result<usize, E>,
        classifier: Option<bool>,
    ) -> Result<Self, Failure<E>> {
        // A classifier never supplies diagnostics or authorizes writes on its
        // own. Disagreement cannot mint a proof, regardless of which accepted.
        if classifier.is_some_and(|valid| valid != reference.is_ok()) {
            return Err(Failure::ClassifierDisagreement);
        }
        let required = reference.map_err(Failure::Input)?;
        let layout = Layout::checked(input.len(), required).ok_or(Failure::Bounds)?;
        Ok(Self {
            input,
            configuration,
            layout,
        })
    }

    pub(crate) const fn len(&self) -> usize {
        self.layout.decoded_len
    }

    pub(crate) const fn configuration(&self) -> C {
        self.configuration
    }

    pub(crate) fn write(
        self,
        output: &mut [u8],
        writer: impl FnOnce(C, &[u8], &[u8], &mut [u8], &mut [u8]),
    ) -> Result<usize, CapacityError> {
        let required = self.len();
        if output.len() < required {
            return Err(CapacityError {
                required,
                available: output.len(),
            });
        }
        let (interior, tail) = self.input.split_at(self.layout.interior_input);
        let (body_output, tail_output) =
            output[..required].split_at_mut(self.layout.interior_output);
        writer(self.configuration, interior, tail, body_output, tail_output);
        Ok(required)
    }

    #[cfg(test)]
    pub(crate) fn classified_for_test<E>(
        input: &'a [u8],
        configuration: C,
        validate: impl FnOnce(C, &[u8]) -> Result<usize, E>,
        classified_valid: bool,
    ) -> Result<Self, Failure<E>> {
        Self::resolve(
            input,
            configuration,
            validate(configuration, input),
            Some(classified_valid),
        )
    }
}

#[cfg(test)]
mod tests;
