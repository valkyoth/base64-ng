use std::io::Cursor;
use std::mem::size_of;

use base64_ng::STANDARD;
use base64_ng::stream::{Decoder, DecoderReader, Encoder, EncoderReader};

pub fn print() {
    let feature_set =
        std::env::var("BASE64_NG_PERF_FEATURE_SET").unwrap_or_else(|_| "default".to_owned());
    println!("schema_version,category,name,feature_set,value,unit,method");
    row(
        &feature_set,
        "stack-bound",
        "in-place-input-staging",
        768,
        "bytes",
    );
    row(
        &feature_set,
        "stack-bound",
        "in-place-output-staging",
        1024,
        "bytes",
    );

    let encoder = Encoder::new(Vec::<u8>::new(), STANDARD);
    row(
        &feature_set,
        "adapter-pending-memory",
        "encoder-writer-output-capacity",
        encoder.buffered_output_capacity(),
        "bytes",
    );
    row(
        &feature_set,
        "adapter-size",
        "encoder-writer",
        size_of::<Encoder<Vec<u8>, base64_ng::Standard, true>>(),
        "bytes",
    );

    let decoder = Decoder::new(Vec::<u8>::new(), STANDARD);
    row(
        &feature_set,
        "adapter-pending-memory",
        "decoder-writer-output-capacity",
        decoder.buffered_output_capacity(),
        "bytes",
    );
    row(
        &feature_set,
        "adapter-size",
        "decoder-writer",
        size_of::<Decoder<Vec<u8>, base64_ng::Standard, true>>(),
        "bytes",
    );

    let encoder_reader = EncoderReader::new(Cursor::new(Vec::<u8>::new()), STANDARD);
    row(
        &feature_set,
        "adapter-pending-memory",
        "encoder-reader-output-capacity",
        encoder_reader.buffered_output_capacity(),
        "bytes",
    );
    row(
        &feature_set,
        "adapter-size",
        "encoder-reader",
        size_of::<EncoderReader<Cursor<Vec<u8>>, base64_ng::Standard, true>>(),
        "bytes",
    );

    let decoder_reader = DecoderReader::new(Cursor::new(Vec::<u8>::new()), STANDARD);
    row(
        &feature_set,
        "adapter-pending-memory",
        "decoder-reader-output-capacity",
        decoder_reader.buffered_output_capacity(),
        "bytes",
    );
    row(
        &feature_set,
        "adapter-size",
        "decoder-reader",
        size_of::<DecoderReader<Cursor<Vec<u8>>, base64_ng::Standard, true>>(),
        "bytes",
    );
}

fn row(feature_set: &str, category: &str, name: &str, value: usize, unit: &str) {
    println!("1,{category},{name},{feature_set},{value},{unit},source-and-size-of");
}
