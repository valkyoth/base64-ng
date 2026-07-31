use super::{Status, web, web::ForgivingError};

#[test]
fn forgiving_core_is_available_without_allocation() {
    let mut output = [0xa5; 4];
    let written = web::FORGIVING
        .decode_into(" Z\tg\n=\x0c=\r ", &mut output)
        .unwrap();
    assert_eq!(&output[..written], b"f");
    assert_eq!(output[written], 0xa5);
    assert_eq!(
        web::FORGIVING.decode_into("Zg=", &mut output),
        Err(ForgivingError::InvalidInput)
    );
}

#[test]
fn forgiving_incremental_core_drains_without_allocation() {
    let mut decoder = web::FORGIVING.decoder();
    let mut output = [0u8; 3];
    let first = decoder.update("Zm", &mut output[..1]).unwrap();
    assert_eq!(first.status(), Status::NeedInput);
    let second = decoder.update("9v", &mut output[..1]).unwrap();
    assert_eq!(second.progress().output_produced(), 1);
    let third = decoder.finish(&mut output[1..]).unwrap();
    assert_eq!(third.status(), Status::Complete);
    assert_eq!(third.progress().output_produced(), 2);
    assert_eq!(output, *b"foo");
}
