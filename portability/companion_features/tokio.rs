use std::vec;

#[test]
fn companion_round_trip_and_rejection() {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(async {
            let codec = base64_ng::STRICT_STANDARD_PADDED;
            let plain = payload();
            let mut encoded = Vec::new();
            companion::encode_reader_to_writer(&codec, &mut plain.as_slice(), &mut encoded)
                .await
                .unwrap();
            let mut decoded = Vec::new();
            companion::decode_reader_to_writer(&codec, &mut encoded.as_slice(), &mut decoded)
                .await
                .unwrap();
            assert_eq!(decoded, plain);
            *encoded.last_mut().unwrap() = b'!';
            let mut output = vec![0xa5; 13];
            assert!(
                companion::decode_reader_to_writer(&codec, &mut encoded.as_slice(), &mut output)
                    .await
                    .is_err()
            );
            assert_eq!(output, [0xa5; 13]);
        });
    secret_boundary();
}
