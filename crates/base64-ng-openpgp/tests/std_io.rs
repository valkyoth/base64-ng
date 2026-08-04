#![allow(missing_docs)]
#![cfg(feature = "std")]

use std::io::{Cursor, Read, Write};

use base64_ng_openpgp::{
    ArmorType, ChecksumGeneration, ChecksumPolicy, GenerationOptions, OpenPgpErrorKind,
    OpenPgpLimits, read_armor_document, write_armor_block,
};

fn pattern(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| (index * 73 + 19).to_le_bytes()[0])
        .collect()
}

#[test]
fn bounded_reader_and_writer_round_trip_short_io() {
    struct ShortWriter(Vec<u8>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let take = bytes.len().min(3);
            self.0.extend_from_slice(&bytes[..take]);
            Ok(take)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let payload = pattern(1025);
    let mut writer = ShortWriter(Vec::new());
    write_armor_block(
        &mut writer,
        ArmorType::Message,
        &[],
        &payload,
        OpenPgpLimits::default(),
        GenerationOptions::new(ChecksumGeneration::LegacyCrc24),
    )
    .unwrap();
    let document = read_armor_document(
        Cursor::new(writer.0),
        OpenPgpLimits::default(),
        ChecksumPolicy::RequireValidCrc24,
    )
    .unwrap();
    assert_eq!(document.blocks()[0].contents(), payload);
}

#[test]
fn overreporting_io_is_rejected_without_panicking() {
    struct OverreportingReader;
    impl Read for OverreportingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            Ok(buffer.len() + 1)
        }
    }

    struct OverreportingWriter;
    impl Write for OverreportingWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            Ok(bytes.len() + 1)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let read_error = read_armor_document(
        OverreportingReader,
        OpenPgpLimits::default(),
        ChecksumPolicy::Rfc9580,
    )
    .unwrap_err();
    assert_eq!(read_error.kind(), OpenPgpErrorKind::Io);

    let write_error = write_armor_block(
        OverreportingWriter,
        ArmorType::Message,
        &[],
        b"packet",
        OpenPgpLimits::default(),
        GenerationOptions::default(),
    )
    .unwrap_err();
    assert_eq!(write_error.kind(), OpenPgpErrorKind::Io);
}
