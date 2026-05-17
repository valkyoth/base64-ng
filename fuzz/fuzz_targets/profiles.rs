#![no_main]

use base64_ng::{
    Alphabet, BCRYPT, CRYPT, Engine, LineEnding, LineWrap, MIME, PEM, PEM_CRLF, Profile, STANDARD,
    decode_alphabet_byte,
};
use libfuzzer_sys::fuzz_target;

struct ReverseAlphabet;

impl Alphabet for ReverseAlphabet {
    const ENCODE: [u8; 64] = *b"/+9876543210zyxwvutsrqponmlkjihgfedcbaZYXWVUTSRQPONMLKJIHGFEDCBA";

    fn decode(byte: u8) -> Option<u8> {
        decode_alphabet_byte(byte, &Self::ENCODE)
    }
}

base64_ng::define_alphabet! {
    struct DotSlashAlphabet = b"./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
}

const REVERSE_PROFILE: Profile<ReverseAlphabet, true> =
    Profile::new(Engine::new(), Some(LineWrap::new(12, LineEnding::Lf)));
const DOT_SLASH_PROFILE: Profile<DotSlashAlphabet, false> =
    Profile::new(Engine::new(), Some(LineWrap::new(8, LineEnding::CrLf)));

fuzz_target!(|data: &[u8]| {
    exercise_profile(data, STANDARD.profile());
    exercise_profile(data, MIME);
    exercise_profile(data, PEM);
    exercise_profile(data, PEM_CRLF);
    exercise_profile(data, BCRYPT);
    exercise_profile(data, CRYPT);
    exercise_profile(data, REVERSE_PROFILE);
    exercise_profile(data, DOT_SLASH_PROFILE);

    exercise_invalid_profile_input(data, MIME);
    exercise_invalid_profile_input(data, BCRYPT);
    exercise_invalid_profile_input(data, REVERSE_PROFILE);
    exercise_invalid_profile_input(data, DOT_SLASH_PROFILE);
});

fn exercise_profile<A, const PAD: bool>(input: &[u8], profile: Profile<A, PAD>)
where
    A: Alphabet,
{
    assert!(profile.is_valid());

    let encoded = profile.encode_vec(input).expect("encoded length fits");
    assert_eq!(profile.encoded_len(input.len()).expect("encoded length fits"), encoded.len());
    assert_eq!(profile.checked_encoded_len(input.len()), Some(encoded.len()));
    assert!(profile.validate(&encoded));
    assert_eq!(profile.validate_result(&encoded), Ok(()));
    assert_eq!(profile.decoded_len(&encoded), Ok(input.len()));

    let mut encoded_output = vec![0xa5; encoded.len() + input.len() % 5 + 1];
    let encoded_len = profile
        .encode_slice_clear_tail(input, &mut encoded_output)
        .expect("profile encoding into large enough output succeeds");
    assert_eq!(encoded_len, encoded.len());
    assert_eq!(&encoded_output[..encoded_len], encoded);
    assert!(encoded_output[encoded_len..].iter().all(|byte| *byte == 0));

    let decoded = profile
        .decode_vec(&encoded)
        .expect("encoded profile output decodes");
    assert_eq!(decoded, input);

    let mut decoded_output = vec![0xa5; input.len() + 3];
    let decoded_len = profile
        .decode_slice_clear_tail(&encoded, &mut decoded_output)
        .expect("profile decoding into large enough output succeeds");
    assert_eq!(decoded_len, input.len());
    assert_eq!(&decoded_output[..decoded_len], input);
    assert!(decoded_output[decoded_len..].iter().all(|byte| *byte == 0));

    let mut in_place = encoded.clone();
    let in_place_decoded = profile
        .decode_in_place_clear_tail(&mut in_place)
        .expect("profile in-place decode succeeds")
        .to_vec();
    assert_eq!(in_place_decoded, input);
    assert!(in_place[input.len()..].iter().all(|byte| *byte == 0));

    if !encoded.is_empty() {
        let mut too_small = vec![0xa5; encoded.len() - 1];
        let err = profile
            .encode_slice_clear_tail(input, &mut too_small)
            .expect_err("profile encoded output exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::EncodeError::OutputTooSmall {
                required: encoded.len(),
                available: encoded.len() - 1,
            }
        );
        assert!(too_small.iter().all(|byte| *byte == 0));
    }

    if !input.is_empty() {
        let mut too_small = vec![0xa5; input.len() - 1];
        let err = profile
            .decode_slice_clear_tail(&encoded, &mut too_small)
            .expect_err("profile decoded output exceeds buffer length");
        assert_eq!(
            err,
            base64_ng::DecodeError::OutputTooSmall {
                required: input.len(),
                available: input.len() - 1,
            }
        );
        assert!(too_small.iter().all(|byte| *byte == 0));
    }
}

fn exercise_invalid_profile_input<A, const PAD: bool>(input: &[u8], profile: Profile<A, PAD>)
where
    A: Alphabet,
{
    let len_result = profile.decoded_len(input);
    let validate_result = profile.validate_result(input);
    let vec_result = profile.decode_vec(input);

    assert_eq!(profile.validate(input), validate_result.is_ok());

    match (&validate_result, &vec_result) {
        (Ok(()), Ok(decoded)) => {
            let expected_len = len_result.expect("valid profile input has decoded length");
            assert_eq!(expected_len, decoded.len());
        }
        (Err(validate_err), Err(decode_err)) => assert_eq!(validate_err, decode_err),
        results => panic!("profile validation and decode disagreed: {results:?}"),
    }

    let mut output = vec![0xa5; len_result.unwrap_or(0) + 3];
    let clear_tail_result = profile.decode_slice_clear_tail(input, &mut output);

    match (clear_tail_result, vec_result) {
        (Ok(written), Ok(decoded)) => {
            assert_eq!(&output[..written], decoded);
            assert!(output[written..].iter().all(|byte| *byte == 0));
        }
        (Err(clear_tail_err), Err(decode_err)) => {
            assert_eq!(clear_tail_err, decode_err);
            assert!(output.iter().all(|byte| *byte == 0));
        }
        results => panic!("profile clear-tail decode disagreed: {results:?}"),
    }
}
