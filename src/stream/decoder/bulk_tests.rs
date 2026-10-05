use super::*;
use crate::{DecodeValidation, STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};

// Frozen quantum loop: intentionally never calls queue_bulk.
fn reference_write<A: Alphabet, const PAD: bool>(
    decoder: &mut Decoder<Vec<u8>, A, PAD>,
    input: &[u8],
) -> io::Result<usize> {
    decoder.drain_output()?;
    let mut consumed = 0;
    while consumed < input.len() {
        let pending = decoder.pending_len();
        let take = (4 - pending).min(input.len() - consumed);
        if pending + take == 4 && decoder.output.available_capacity() < 3 {
            break;
        }
        match decoder.queue_update(&input[consumed..consumed + take]) {
            Ok(accepted) => consumed += accepted,
            Err(_) if consumed != 0 => return Ok(consumed),
            Err(error) => return Err(error),
        }
        if decoder.finished || take < 4 - pending {
            break;
        }
    }
    Ok(consumed)
}

fn compare<A: Alphabet, const PAD: bool>(engine: Engine<A, PAD>) {
    let input = engine.encode_vec(&[0xfb; 1025]).unwrap();
    for policy in [DecodeValidation::Auto, DecodeValidation::ScalarReference] {
        for pending in 0..4 {
            for bad in 0..=input.len() {
                let mut input = input.clone();
                if bad < input.len() {
                    input[bad] = b'!';
                }
                let mut fast = Decoder::new(Vec::new(), engine).with_validation(policy);
                let mut slow = Decoder::new(Vec::new(), engine).with_validation(policy);
                fast.write_all(&vec![b'A'; pending]).unwrap();
                slow.write_all(&vec![b'A'; pending]).unwrap();
                let actual = fast.write(&input);
                let expected = reference_write(&mut slow, &input);
                assert_eq!(
                    actual.as_ref().ok(),
                    expected.as_ref().ok(),
                    "pending={pending}, bad={bad}"
                );
                assert_eq!(
                    actual.err().map(|e| e.to_string()),
                    expected.err().map(|e| e.to_string())
                );
                assert_eq!(fast.driver, slow.driver);
                assert_eq!((fast.failed, fast.finished), (slow.failed, slow.finished));
                let mut actual = [0xa5; 1024];
                let mut expected = actual;
                assert_eq!(
                    fast.output.copy_front(&mut actual),
                    slow.output.copy_front(&mut expected)
                );
                assert_eq!(actual, expected);
                assert_eq!(fast.get_ref(), slow.get_ref());
                assert_eq!(fast.buffered_output_capacity(), 1024);
            }
        }
    }
}

#[test]
fn stream_bulk_matches_quantum_progress_errors_state_and_entire_queue() {
    compare(STANDARD);
    compare(STANDARD_NO_PAD);
    compare(URL_SAFE);
    compare(URL_SAFE_NO_PAD);
}

#[test]
fn stream_bulk_miri_bounded_pending_and_rejection() {
    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(b"A").unwrap();
    let mut invalid = [b'A'; 516];
    invalid[512] = b'!';
    let accepted = decoder.write(&invalid).unwrap();
    assert_eq!(accepted, 511);
    assert!(decoder.is_failed());
    assert_eq!(decoder.buffered_output_len(), 384);
    assert_eq!(decoder.get_ref().as_slice(), []);

    let mut decoder = Decoder::new(Vec::new(), STANDARD);
    decoder.write_all(&[b'A'; 1364]).unwrap();
    assert_eq!(decoder.finish().unwrap(), vec![0; 1023]);
}
