use alloc::vec::Vec;

use base64_ng::{Base64, BodyCodec, Codec, DecodedArray};

use crate::adapter::{AdapterError, DecodeInput, body_decoded_len, decode_body_into, map_one_shot};
use crate::limits::{enforce_body_input_limit, enforce_codec_input_limit};

pub(crate) struct VecDecoder<'a, C> {
    pub(crate) codec: &'a Base64<C>,
    pub(crate) maximum_decoded: usize,
}

impl<C: Codec> DecodeInput for VecDecoder<'_, C> {
    type Output = Vec<u8>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        enforce_codec_input_limit(self.codec, input.len(), self.maximum_decoded)?;
        self.codec
            .decode_to_vec_with_limit(input, self.maximum_decoded)
            .map_err(map_one_shot)
    }
}

pub(crate) struct BoundedDecoder<'a, C, const CAP: usize> {
    pub(crate) codec: &'a Base64<C>,
}

impl<C: Codec, const CAP: usize> DecodeInput for BoundedDecoder<'_, C, CAP> {
    type Output = DecodedArray<CAP>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        enforce_codec_input_limit(self.codec, input.len(), CAP)?;
        self.codec.decode_bounded(input).map_err(map_one_shot)
    }
}

pub(crate) struct BodyVecDecoder<'a, C> {
    pub(crate) body: &'a BodyCodec<C>,
    pub(crate) maximum_decoded: usize,
}

impl<C: Codec> DecodeInput for BodyVecDecoder<'_, C> {
    type Output = Vec<u8>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        enforce_body_input_limit(self.body, input.len(), self.maximum_decoded)?;
        let required = body_decoded_len(self.body, input)?;
        if required > self.maximum_decoded {
            return Err(AdapterError::OutputLimit);
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(required)
            .map_err(|_| AdapterError::AllocationFailed)?;
        output.resize(required, 0);
        decode_body_into(self.body, input, &mut output)?;
        Ok(output)
    }
}

pub(crate) struct BodyBoundedDecoder<'a, C, const CAP: usize> {
    pub(crate) body: &'a BodyCodec<C>,
}

impl<C: Codec, const CAP: usize> DecodeInput for BodyBoundedDecoder<'_, C, CAP> {
    type Output = DecodedArray<CAP>;

    fn decode(self, input: &[u8]) -> Result<Self::Output, AdapterError> {
        enforce_body_input_limit(self.body, input.len(), CAP)?;
        let required = body_decoded_len(self.body, input)?;
        if required > CAP {
            return Err(AdapterError::OutputLimit);
        }
        let mut output = [0u8; CAP];
        let written = decode_body_into(self.body, input, &mut output)?;
        DecodedArray::from_array(output, written).map_err(|_| AdapterError::InternalFailure)
    }
}
