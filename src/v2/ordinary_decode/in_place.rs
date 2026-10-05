//! Forward compaction with a preserved, disjoint source for each writer call.

use super::{Prepared, prepare, vector, write_parts};
use crate::{CodecSettings, DecodeValidation, OneShotError};

const INPUT_CHUNK: usize = 1024;

// This capability cannot escape this module or be constructed from caller
// supplied lengths. Validation borrows its exact exclusive source before the
// first write; no replacement input, settings or fallible operation follows.
struct Validated<'a> {
    buffer: &'a mut [u8],
    config: Prepared,
    required: usize,
}

pub(crate) fn decode(
    settings: CodecSettings,
    buffer: &mut [u8],
    validation: DecodeValidation,
) -> Result<usize, OneShotError> {
    let (config, required) = {
        let proof = prepare(settings, buffer, validation)?;
        (proof.configuration(), proof.len())
    };
    Ok(Validated {
        buffer,
        config,
        required,
    }
    .write())
}

impl Validated<'_> {
    fn write(self) -> usize {
        let Self {
            buffer,
            config,
            required,
        } = self;
        let (Some(family), Some(backend)) = (config.family, config.backend) else {
            super::super::in_place::decode_forward(config.settings, buffer, buffer.len());
            return required;
        };
        let table = family.table();
        let body = buffer.len().saturating_sub(1) / 4 * 4;
        let mut scratch = [0; INPUT_CHUNK];
        let mut read = 0;
        while read < body {
            let count = (body - read).min(INPUT_CHUNK);
            // All source bytes are copied before any overlapping destination
            // is borrowed. write <= read and write + 3*count/4 <= read + count:
            // neither the next chunk nor the reserved final quantum is touched.
            scratch[..count].copy_from_slice(&buffer[read..read + count]);
            let write = read / 4 * 3;
            let output = &mut buffer[write..write + count / 4 * 3];
            let consumed = vector::write(backend, family, &scratch[..count], output, &mut ());
            // Rejection (even after partial stores) returns zero. The retained
            // source permits complete scalar repair with no error after mutation.
            write_parts(
                &scratch[consumed..count],
                &[],
                &mut output[consumed / 4 * 3..],
                &mut [],
                |byte| table[usize::from(byte)],
            );
            crate::wipe_bytes(&mut scratch[..count]);
            read += count;
        }
        let tail = buffer.len() - body;
        scratch[..tail].copy_from_slice(&buffer[body..]);
        write_parts(
            &[],
            &scratch[..tail],
            &mut [],
            &mut buffer[body / 4 * 3..required],
            |byte| table[usize::from(byte)],
        );
        crate::wipe_bytes(&mut scratch[..tail]);
        required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_place_bulk_miri_preserved_source_and_scalar_repair() {
        let codec = crate::STRICT_STANDARD_UNPADDED;
        for len in [2, 3, 4, 16, 1024, 1026, 2051, 2052] {
            let mut buffer = [b'A'; 2052];
            let (mut config, required) = {
                let proof = prepare(
                    codec.settings(),
                    &buffer[..len],
                    DecodeValidation::ScalarReference,
                )
                .unwrap();
                (proof.configuration(), proof.len())
            };
            // Force the chunk loop even without native SIMD (including Miri).
            // Scalar has no vector width, so every staged chunk is repaired.
            config.backend = Some(crate::runtime::Backend::Scalar);
            assert_eq!(
                Validated {
                    buffer: &mut buffer[..len],
                    config,
                    required
                }
                .write(),
                required
            );
            assert!(buffer[..required].iter().all(|&byte| byte == 0));
            assert!(buffer[required..].iter().all(|&byte| byte == b'A'));
        }
    }
}
