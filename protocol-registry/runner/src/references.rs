use base64::Engine as _;
use base64::alphabet::Alphabet;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use base64_ng::{
    BCRYPT_ALPHABET_NO_PAD, BINHEX_ALPHABET, CRYPT_ALPHABET_NO_PAD, IMAP_MUTF7_ALPHABET_NO_PAD,
    PBKDF2_ALPHABET_NO_PAD, STRICT_STANDARD_PADDED, STRICT_STANDARD_UNPADDED,
    STRICT_URL_SAFE_PADDED, STRICT_URL_SAFE_UNPADDED, compat,
};
use base64ct::{Base64Pbkdf2, Encoding as _};

use crate::Case;

pub(crate) fn run(cases: &[Case]) {
    let strict = [
        STRICT_STANDARD_PADDED.settings(),
        STRICT_STANDARD_UNPADDED.settings(),
        STRICT_URL_SAFE_PADDED.settings(),
        STRICT_URL_SAFE_UNPADDED.settings(),
    ];
    for settings in strict {
        let external = engine(settings);
        for input in [b"".as_slice(), b"f", b"fo", b"foo", &[0xfb, 0xff]] {
            let ours = base64_ng::CodecBuilder::new(*settings.alphabet())
                .encode_padding(settings.encode_padding())
                .decode_padding(settings.decode_padding())
                .trailing_bits(settings.trailing_bits())
                .build()
                .unwrap();
            assert_eq!(
                ours.encode_to_string(input).unwrap().as_bytes(),
                external.encode(input).as_bytes()
            );
        }
    }

    let named = [
        ("core-bcrypt", BCRYPT_ALPHABET_NO_PAD),
        ("core-crypt", CRYPT_ALPHABET_NO_PAD),
        ("core-imap", IMAP_MUTF7_ALPHABET_NO_PAD),
    ];
    for (id, codec) in named {
        let case = find(cases, id);
        assert_eq!(
            codec.encode_to_string(&case.plain).unwrap().as_bytes(),
            case.wire
        );
        assert_eq!(codec.decode_to_vec(&case.wire).unwrap(), case.plain);
        assert_eq!(
            codec.encode_to_string(&case.plain).unwrap().as_bytes(),
            engine(codec.settings()).encode(&case.plain).as_bytes()
        );
    }

    let binhex = base64_ng::CodecBuilder::new(BINHEX_ALPHABET)
        .encode_padding(base64_ng::EncodePadding::Unpadded)
        .decode_padding(base64_ng::DecodePadding::Forbid)
        .build()
        .unwrap();
    let case = find(cases, "core-binhex");
    assert_eq!(
        binhex.encode_to_string(&case.plain).unwrap().as_bytes(),
        case.wire
    );

    let pbkdf2 = find(cases, "core-pbkdf2");
    let mut expected = [0u8; 16];
    let expected = Base64Pbkdf2::encode(&pbkdf2.plain, &mut expected).unwrap();
    assert_eq!(
        PBKDF2_ALPHABET_NO_PAD
            .encode_to_string(&pbkdf2.plain)
            .unwrap()
            .as_bytes(),
        expected.as_bytes()
    );

    let compatibility = [
        compat::STANDARD_PADDED_PADDING_INDIFFERENT,
        compat::STANDARD_UNPADDED_PADDING_INDIFFERENT,
        compat::STANDARD_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::STANDARD_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::STANDARD_PADDED_FULL_COMPATIBILITY,
        compat::STANDARD_UNPADDED_FULL_COMPATIBILITY,
        compat::URL_SAFE_PADDED_PADDING_INDIFFERENT,
        compat::URL_SAFE_UNPADDED_PADDING_INDIFFERENT,
        compat::URL_SAFE_PADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::URL_SAFE_UNPADDED_ALLOW_NONCANONICAL_TRAILING_BITS,
        compat::URL_SAFE_PADDED_FULL_COMPATIBILITY,
        compat::URL_SAFE_UNPADDED_FULL_COMPATIBILITY,
    ];
    let corpus = [
        b"Z".as_slice(),
        b"Zg",
        b"Zh",
        b"Zg=",
        b"Zh=",
        b"Zg==",
        b"Zh==",
        b"AA/A",
        b"AA_A",
    ];
    for codec in compatibility {
        let external = engine(codec.settings());
        for input in corpus {
            assert_eq!(codec.decode_to_vec(input).ok(), external.decode(input).ok());
        }
    }
}

fn engine(settings: base64_ng::CodecSettings) -> GeneralPurpose {
    let table = core::str::from_utf8(settings.alphabet().as_array()).unwrap();
    let alphabet = Alphabet::new(table).unwrap();
    let config = GeneralPurposeConfig::new()
        .with_encode_padding(settings.encode_padding() == base64_ng::EncodePadding::Padded)
        .with_decode_padding_mode(match settings.decode_padding() {
            base64_ng::DecodePadding::RequireCanonical => DecodePaddingMode::RequireCanonical,
            base64_ng::DecodePadding::Forbid => DecodePaddingMode::RequireNone,
            base64_ng::DecodePadding::Indifferent => DecodePaddingMode::Indifferent,
        })
        .with_decode_allow_trailing_bits(
            settings.trailing_bits() == base64_ng::TrailingBits::AllowNonCanonical,
        );
    GeneralPurpose::new(&alphabet, config)
}

fn find<'a>(cases: &'a [Case], id: &str) -> &'a Case {
    cases.iter().find(|case| case.id == id).unwrap()
}
