#![allow(missing_docs)]

use base64_ng::{DecodeValidation, STRICT_STANDARD_PADDED as CODEC, Status};
use base64_ng_bytes::{Base64BytesExt, BytesLimits};
use bytes::Buf;

#[test]
fn bulk_bytes_policy_preserves_fragments_limits_and_output_backpressure() {
    let raw = vec![0xfb; 8195];
    let encoded = CODEC.encode_to_string(&raw).unwrap();
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for fragment in [1, 7, 516, 4096] {
            for capacity in [1, 17, 1024, 4096] {
                let mut decoder =
                    CODEC.bytes_decoder_with_limits(BytesLimits::new(encoded.len(), raw.len()));
                let mut output = Vec::new();
                for mut input in encoded.as_bytes().chunks(fragment) {
                    while input.has_remaining() {
                        let mut scratch = vec![0xa5; capacity];
                        let step = decoder
                            .update_with_validation(&mut input, &mut scratch.as_mut_slice(), policy)
                            .unwrap();
                        output.extend_from_slice(&scratch[..step.progress().output_committed()]);
                    }
                }
                loop {
                    let mut scratch = [0; 3];
                    let step = decoder.finish(&mut scratch.as_mut_slice()).unwrap();
                    output.extend_from_slice(&scratch[..step.progress().output_committed()]);
                    if step.status() == Status::Complete {
                        break;
                    }
                }
                assert_eq!(output, raw);
                assert_eq!(decoder.source_position(), encoded.len());
                assert_eq!(decoder.output_committed(), raw.len());
            }
        }
        let mut decoder =
            CODEC.bytes_decoder_with_limits(BytesLimits::new(encoded.len() - 1, raw.len()));
        let mut output = Vec::new();
        let mut input = encoded.as_bytes();
        assert!(
            decoder
                .update_with_validation(&mut input, &mut output, policy)
                .is_err()
        );
        assert_eq!(input, encoded.as_bytes());
        assert_eq!(output, [] as [u8; 0]);
        assert!(decoder.is_failed());
    }
}

#[test]
fn bulk_bytes_late_invalid_fragment_retains_only_committed_prefix() {
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        let prefix = vec![b'A'; 4096];
        let mut input = prefix.as_slice().chain(b"AAA!".as_slice());
        let mut decoder = CODEC.bytes_decoder();
        let mut output = Vec::new();
        let error = decoder
            .update_with_validation(&mut input, &mut output, policy)
            .unwrap_err();
        assert_eq!(error.progress().input_consumed(), prefix.len());
        assert_eq!(error.progress().output_committed(), 3072);
        assert_eq!(input.remaining(), 4);
        assert_eq!(output, vec![0; 3072]);
        assert!(decoder.is_failed());
    }
}
