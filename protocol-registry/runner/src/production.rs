use base64_ng_imap::{ImapPayloadLimits, decode_modified_utf7_payload_to_vec};
use base64_ng_mime::{
    MimeBodyDecodePolicy, MimeBodyLimits, decode_mime_content_transfer_body_to_vec,
};
use base64_ng_multibase::{Base64MultibaseLimits, decode_base64_multibase_to_vec};
use base64_ng_openpgp::{ChecksumPolicy, OpenPgpLimits, parse_armor_document};
use base64_ng_password::{PasswordRecordLimits, parse_pbkdf2_record, parse_sha_crypt_record};
use base64_ng_pem::{PemLimits, PemParsePolicy, parse_pem_document};

use crate::Case;

pub(crate) fn evaluate(case: &Case) -> Option<Vec<u8>> {
    match case.registry_id.as_str() {
        "mime-body" => decode_mime_content_transfer_body_to_vec(
            &case.wire,
            MimeBodyDecodePolicy::Canonical,
            MimeBodyLimits::default(),
        )
        .ok()
        .map(|(bytes, _)| bytes),
        "pem-textual" => {
            let document =
                parse_pem_document(&case.wire, PemLimits::default(), PemParsePolicy::Strict)
                    .ok()?;
            (document.blocks().len() == 1).then(|| document.blocks()[0].contents().to_vec())
        }
        "multibase-base64" => {
            decode_base64_multibase_to_vec(&case.wire, Base64MultibaseLimits::new(4096, 4096, 4096))
                .ok()
                .map(base64_ng_multibase::DecodedBase64MultibaseVec::into_bytes)
        }
        "imap-mutf7-payload" => decode_modified_utf7_payload_to_vec(
            &case.wire,
            ImapPayloadLimits::new(4096, 4096, 4096),
        )
        .ok(),
        "passlib-pbkdf2" => parse_pbkdf2_record(&case.wire, PasswordRecordLimits::default())
            .ok()
            .map(|_| Vec::new()),
        "sha-crypt" => parse_sha_crypt_record(&case.wire, PasswordRecordLimits::default())
            .ok()
            .map(|_| Vec::new()),
        "openpgp-armor" => {
            let document = parse_armor_document(
                &case.wire,
                OpenPgpLimits::default(),
                ChecksumPolicy::Rfc9580,
            )
            .ok()?;
            (document.blocks().len() == 1).then(|| document.blocks()[0].contents().to_vec())
        }
        other => panic!("unregistered production surface {other}"),
    }
}
