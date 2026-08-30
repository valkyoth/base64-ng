use base64_ng::{
    Alphabet, Base64, Codec, DecodeError, Engine, Failure, OneShotError, OperationError, STANDARD,
    STANDARD_NO_PAD, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, Standard, URL_SAFE, URL_SAFE_NO_PAD, UrlSafe,
    ct,
};
use base64_ng_bytes::{Base64BytesExt, BytesError, BytesErrorKind};
use base64_ng_sanitization::{CtDecodeSanitizationExt, SecretVecDecodeError};
use base64_ng_serde::{Base64Standard, Base64UrlSafeNoPad};
use bytes::Bytes;
use std::io::Write;

mod v2;

const CORPUS: &str = include_str!("../../v1/cases.tsv");
const EXPECTED_HEADER: &str = "id\tprofile\toperation\tinput_hex\tencoded\tdecision\t\
error_class\terror_offset\teof\tpartitions\tcommitted_prefix_hex\tcore_one_shot\t\
core_stream\tbytes\ttokio\tserde\tsanitization";

#[derive(Clone, Copy)]
enum Profile {
    StandardPadded,
    StandardUnpadded,
    UrlSafePadded,
    UrlSafeUnpadded,
}

struct Case<'a> {
    id: &'a str,
    profile: Profile,
    operation: &'a str,
    input: Vec<u8>,
    encoded: &'a [u8],
    error_class: &'a str,
    error_offset: Option<usize>,
    partitions: Vec<usize>,
    committed_prefix: Vec<u8>,
    core_one_shot_contract: &'a str,
    core_stream_contract: &'a str,
    bytes_contract: &'a str,
    tokio_contract: &'a str,
    serde_contract: &'a str,
    sanitization_contract: &'a str,
}

fn main() {
    let mut lines = CORPUS.lines();
    assert_eq!(lines.next(), Some(EXPECTED_HEADER));
    let cases: Vec<_> = lines.map(parse_case).collect();
    assert!(!cases.is_empty());

    for case in &cases {
        match case.profile {
            Profile::StandardPadded => {
                exercise::<Standard, true, _>(case, STANDARD, STRICT_STANDARD_PADDED);
            }
            Profile::StandardUnpadded => {
                exercise::<Standard, false, _>(case, STANDARD_NO_PAD, STRICT_STANDARD_UNPADDED);
            }
            Profile::UrlSafePadded => {
                exercise::<UrlSafe, true, _>(case, URL_SAFE, STRICT_URL_SAFE_PADDED);
            }
            Profile::UrlSafeUnpadded => {
                exercise::<UrlSafe, false, _>(case, URL_SAFE_NO_PAD, STRICT_URL_SAFE_UNPADDED);
            }
        }
    }

    println!("semantic corpus: {} cases passed", cases.len());
}

fn exercise<A, const PAD: bool, S>(case: &Case<'_>, engine: Engine<A, PAD>, codec: Base64<S>)
where
    A: Alphabet,
    S: Codec,
{
    match case.operation {
        "round-trip" => exercise_success(case, engine, codec),
        "decode-error" => exercise_failure(case, engine, codec),
        other => panic!("{}: unknown operation {other}", case.id),
    }
}

fn exercise_success<A, const PAD: bool, S>(
    case: &Case<'_>,
    engine: Engine<A, PAD>,
    codec: Base64<S>,
)
where
    A: Alphabet,
    S: Codec,
{
    assert_eq!(case.core_one_shot_contract, "byte-identical", "{}", case.id);
    assert_eq!(case.core_stream_contract, "byte-identical", "{}", case.id);
    assert_eq!(case.bytes_contract, "byte-identical", "{}", case.id);
    assert_eq!(case.tokio_contract, "byte-identical", "{}", case.id);

    v2::exercise_success(case, &codec);

    assert_eq!(
        engine.encode_vec(&case.input).unwrap(),
        case.encoded,
        "{}",
        case.id
    );
    assert_eq!(
        engine.decode_vec(case.encoded).unwrap(),
        case.input,
        "{}",
        case.id
    );

    let mut encoded_stream = engine.encoder_writer(Vec::new());
    write_partitioned(&mut encoded_stream, &case.input, &case.partitions);
    assert_eq!(
        encoded_stream.finish().unwrap(),
        case.encoded,
        "{}",
        case.id
    );

    let mut decoded_stream = engine.decoder_writer(Vec::new());
    write_partitioned(&mut decoded_stream, case.encoded, &case.partitions);
    assert_eq!(decoded_stream.finish().unwrap(), case.input, "{}", case.id);

    assert_eq!(
        codec
            .encode_buf(Bytes::copy_from_slice(&case.input))
            .unwrap()
            .as_ref(),
        case.encoded
    );
    assert_eq!(
        codec
            .decode_buf(Bytes::copy_from_slice(case.encoded))
            .unwrap()
            .as_ref(),
        case.input
    );
    assert_eq!(
        base64_ng_tokio::encode_to_vec(&codec, &case.input).unwrap(),
        case.encoded
    );
    assert_eq!(
        base64_ng_tokio::decode_to_vec(&codec, case.encoded).unwrap(),
        case.input
    );

    exercise_serde_success(case);
    exercise_sanitization_success(case);
}

fn exercise_failure<A, const PAD: bool, S>(
    case: &Case<'_>,
    engine: Engine<A, PAD>,
    codec: Base64<S>,
)
where
    A: Alphabet,
    S: Codec,
{
    v2::exercise_failure(case, &codec);

    let error = engine.decode_vec(case.encoded).unwrap_err();
    assert_error(case, error);

    let mut one_shot_output = [0xa5; 64];
    let one_shot_error = engine
        .decode_slice(case.encoded, &mut one_shot_output)
        .unwrap_err();
    assert_error(case, one_shot_error);
    match case.core_one_shot_contract {
        "atomic-unchanged" => assert!(one_shot_output.iter().all(|byte| *byte == 0xa5)),
        "committed-prefix" => {
            assert_eq!(
                &one_shot_output[..case.committed_prefix.len()],
                case.committed_prefix
            );
            assert!(
                one_shot_output[case.committed_prefix.len()..]
                    .iter()
                    .all(|byte| *byte == 0xa5),
                "{}: one-shot decode modified bytes beyond the committed prefix",
                case.id
            );
        }
        other => panic!("{}: unknown one-shot contract {other}", case.id),
    }

    assert_eq!(
        case.core_stream_contract, "irrevocable-sink-progress",
        "{}",
        case.id
    );
    let mut stream = engine.decoder_writer(Vec::new());
    let stream_result = write_partitioned_result(&mut stream, case.encoded, &case.partitions)
        .and_then(|()| stream.try_finish());
    assert!(
        stream_result.is_err(),
        "{}: stream accepted malformed input",
        case.id
    );
    let stream_output = stream.into_inner();
    assert_eq!(stream_output, case.committed_prefix, "{}", case.id);

    assert_eq!(case.bytes_contract, "atomic-unchanged", "{}", case.id);
    let mut expected_output = [0u8; 64];
    let expected_error = codec
        .decode_into(case.encoded, &mut expected_output)
        .unwrap_err();
    let bytes_error = codec
        .decode_buf(Bytes::copy_from_slice(case.encoded))
        .unwrap_err();
    assert_bytes_error(case, bytes_error, expected_error);

    assert_eq!(case.tokio_contract, "atomic-unchanged", "{}", case.id);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let mut reader = case.encoded;
    let mut tokio_output = b"unchanged".to_vec();
    let tokio_result = runtime.block_on(base64_ng_tokio::decode_reader_to_writer(
        &codec,
        &mut reader,
        &mut tokio_output,
    ));
    assert!(tokio_result.is_err());
    assert_eq!(tokio_output, b"unchanged");

    exercise_serde_failure(case);
    exercise_sanitization_failure(case);
}

fn exercise_serde_success(case: &Case<'_>) {
    match (case.profile, case.serde_contract) {
        (_, "not-applicable") => {}
        (Profile::StandardPadded, "byte-identical") => {
            let wrapped = Base64Standard::new(case.input.clone());
            let json = serde_json::to_string(&wrapped).unwrap();
            assert_eq!(
                json,
                format!("\"{}\"", String::from_utf8_lossy(case.encoded))
            );
            let decoded: Base64Standard = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.as_bytes(), case.input);
        }
        (Profile::UrlSafeUnpadded, "byte-identical") => {
            let wrapped = Base64UrlSafeNoPad::new(case.input.clone());
            let json = serde_json::to_string(&wrapped).unwrap();
            assert_eq!(
                json,
                format!("\"{}\"", String::from_utf8_lossy(case.encoded))
            );
            let decoded: Base64UrlSafeNoPad = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded.as_bytes(), case.input);
        }
        _ => panic!("{}: unsupported serde contract", case.id),
    }
}

fn exercise_serde_failure(case: &Case<'_>) {
    match (case.profile, case.serde_contract) {
        (_, "not-applicable") => {}
        (Profile::StandardPadded, "atomic-unchanged") => {
            let json = format!("\"{}\"", String::from_utf8_lossy(case.encoded));
            assert!(serde_json::from_str::<Base64Standard>(&json).is_err());
        }
        (Profile::UrlSafeUnpadded, "atomic-unchanged") => {
            let json = format!("\"{}\"", String::from_utf8_lossy(case.encoded));
            assert!(serde_json::from_str::<Base64UrlSafeNoPad>(&json).is_err());
        }
        _ => panic!("{}: unsupported serde failure contract", case.id),
    }
}

fn exercise_sanitization_success(case: &Case<'_>) {
    match (case.profile, case.sanitization_contract) {
        (_, "not-applicable") => {}
        (Profile::StandardPadded, "byte-identical") => {
            let secret = ct::STANDARD.decode_secret_vec(case.encoded).unwrap();
            secret.with_secret(|bytes| assert_eq!(bytes, case.input));
        }
        (Profile::UrlSafeUnpadded, "byte-identical") => {
            let secret = ct::URL_SAFE_NO_PAD.decode_secret_vec(case.encoded).unwrap();
            secret.with_secret(|bytes| assert_eq!(bytes, case.input));
        }
        _ => panic!("{}: unsupported sanitization contract", case.id),
    }
}

fn exercise_sanitization_failure(case: &Case<'_>) {
    match (case.profile, case.sanitization_contract) {
        (_, "not-applicable") => {}
        (Profile::StandardPadded, "opaque-reject") => {
            let error = ct::STANDARD.decode_secret_vec(case.encoded).unwrap_err();
            assert!(
                matches!(
                    error,
                    SecretVecDecodeError::Decode(
                        DecodeError::InvalidInput | DecodeError::InvalidLength
                    )
                ),
                "{}: secret surface exposed a localized error",
                case.id
            );
        }
        (Profile::UrlSafeUnpadded, "opaque-reject") => {
            let error = ct::URL_SAFE_NO_PAD
                .decode_secret_vec(case.encoded)
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    SecretVecDecodeError::Decode(
                        DecodeError::InvalidInput | DecodeError::InvalidLength
                    )
                ),
                "{}: secret surface exposed a localized error",
                case.id
            );
        }
        _ => panic!("{}: unsupported sanitization failure contract", case.id),
    }
}

fn write_partitioned<W: Write>(writer: &mut W, input: &[u8], partitions: &[usize]) {
    write_partitioned_result(writer, input, partitions).unwrap();
}

fn write_partitioned_result<W: Write>(
    writer: &mut W,
    input: &[u8],
    partitions: &[usize],
) -> std::io::Result<()> {
    let mut offset = 0usize;
    for &requested in partitions {
        let end = offset.saturating_add(requested).min(input.len());
        writer.write_all(&input[offset..end])?;
        offset = end;
    }
    if offset < input.len() {
        writer.write_all(&input[offset..])?;
    }
    Ok(())
}

fn assert_error(case: &Case<'_>, error: DecodeError) {
    assert_eq!(error.kind().as_str(), case.error_class, "{}", case.id);
    let offset = match error {
        DecodeError::InvalidByte { index, .. }
        | DecodeError::InvalidPadding { index }
        | DecodeError::InvalidLineWrap { index } => Some(index),
        _ => None,
    };
    assert_eq!(offset, case.error_offset, "{}", case.id);
}

fn assert_bytes_error(case: &Case<'_>, error: BytesError, expected: OneShotError) {
    let BytesErrorKind::Operation(OperationError::Failed(Failure::Input(error))) = error.kind()
    else {
        panic!("{}: unexpected bytes error {error}", case.id);
    };
    let OneShotError::Input(expected) = expected else {
        panic!("{}: unexpected 2.0 one-shot error {expected}", case.id);
    };
    assert_eq!(error, expected, "{}", case.id);
}

fn parse_case(line: &str) -> Case<'_> {
    let columns: Vec<_> = line.split('\t').collect();
    assert_eq!(columns.len(), 17, "invalid corpus row: {line}");
    assert!(matches!(columns[5], "canonical" | "reject"));
    assert!(matches!(
        columns[8],
        "complete" | "malformed" | "incomplete"
    ));
    for contract in &columns[11..] {
        assert!(!contract.is_empty());
    }
    Case {
        id: columns[0],
        profile: match columns[1] {
            "standard-pad" => Profile::StandardPadded,
            "standard-no-pad" => Profile::StandardUnpadded,
            "url-safe-pad" => Profile::UrlSafePadded,
            "url-safe-no-pad" => Profile::UrlSafeUnpadded,
            other => panic!("unknown profile {other}"),
        },
        operation: columns[2],
        input: decode_hex(columns[3]),
        encoded: columns[4].as_bytes(),
        error_class: columns[6],
        error_offset: (columns[7] != "-").then(|| columns[7].parse().unwrap()),
        partitions: columns[9]
            .split(',')
            .map(|value| value.parse().unwrap())
            .collect(),
        committed_prefix: decode_hex(columns[10]),
        core_one_shot_contract: columns[11],
        core_stream_contract: columns[12],
        bytes_contract: columns[13],
        tokio_contract: columns[14],
        serde_contract: columns[15],
        sanitization_contract: columns[16],
    }
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert!(input.len().is_multiple_of(2));
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).unwrap();
            u8::from_str_radix(text, 16).unwrap()
        })
        .collect()
}
