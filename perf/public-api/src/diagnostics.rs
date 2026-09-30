use super::oracle;
use base64_ng::{Alphabet, Base64, Codec, Engine};

pub fn run() {
    inspect(
        &base64_ng::STRICT_STANDARD_PADDED,
        base64_ng::STANDARD,
        oracle::Profile::StandardPadded,
    );
    inspect(
        &base64_ng::STRICT_STANDARD_UNPADDED,
        base64_ng::STANDARD_NO_PAD,
        oracle::Profile::StandardUnpadded,
    );
    inspect(
        &base64_ng::STRICT_URL_SAFE_PADDED,
        base64_ng::URL_SAFE,
        oracle::Profile::UrlSafePadded,
    );
    inspect(
        &base64_ng::STRICT_URL_SAFE_UNPADDED,
        base64_ng::URL_SAFE_NO_PAD,
        oracle::Profile::UrlSafeUnpadded,
    );
}

fn inspect<S: Codec, A: Alphabet, const PAD: bool>(
    codec: &Base64<S>,
    engine: Engine<A, PAD>,
    profile: oracle::Profile,
) {
    let mut corpus = Vec::new();
    for len in 0..=8 {
        corpus.push(vec![b'A'; len]);
        for first in 0..len {
            for byte in [b'!', b'=', b'/', b'_', 0xff] {
                let mut input = vec![b'A'; len];
                input[first] = byte;
                corpus.push(input.clone());
                for second in first + 1..len {
                    let mut multi = input.clone();
                    multi[second] = b'=';
                    corpus.push(multi);
                }
            }
        }
    }
    for position in 0..4 {
        for byte in 0..=255 {
            let mut input = b"Zg==".to_vec();
            input[position] = byte;
            corpus.push(input);
        }
    }
    for input in corpus {
        let expected = oracle::decode(profile, &input);
        for capacity in [0, 16] {
            let mut canonical = [0xa5; 16];
            let mut historical = canonical;
            let current = codec.decode_into(&input, &mut canonical[..capacity]);
            let old = engine.decode_slice(&input, &mut historical[..capacity]);
            if current.is_err() {
                assert_eq!(canonical, [0xa5; 16]);
            }
            if capacity == 16 {
                assert_eq!(current.is_ok(), expected.is_ok());
                assert_eq!(old.is_ok(), expected.is_ok());
                if let Ok(ref expected) = expected {
                    assert_eq!(&canonical[..current.unwrap()], expected);
                    assert_eq!(&historical[..old.unwrap()], expected);
                }
            }
            // Retain full diagnostics and mutation, not only success/failure parity.
            println!(
                "{profile:?}|{input:02x?}|{capacity}|{current:?}|{old:?}|{canonical:02x?}|{historical:02x?}"
            );
        }
    }
}
