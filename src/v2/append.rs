//! Rollback-capable append operations for controlled allocation containers.

use alloc::{string::String, vec::Vec};

use super::{
    contracts::BackendFault,
    ordinary::OneShotError,
    specifications::{Base64, Codec},
};

impl<S: Codec> Base64<S> {
    /// Appends encoded output to `destination` transactionally.
    ///
    /// The existing prefix and entry length are restored on every returned
    /// crate error and during unwinding. Capacity growth is not rolled back,
    /// and process-aborting allocation failure is outside this guarantee.
    /// Returns the number of appended bytes.
    pub fn encode_append(
        &self,
        input: &[u8],
        destination: &mut String,
    ) -> Result<usize, OneShotError> {
        self.encode_append_inner(
            input,
            destination,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_, _| Ok(()),
        )
    }

    /// Appends decoded output to `destination` transactionally.
    ///
    /// Strict validation and exact sizing precede allocation and mutation.
    /// The existing prefix and entry length are restored on every returned
    /// crate error and during unwinding. Returns the appended plaintext bytes.
    pub fn decode_append(
        &self,
        input: &[u8],
        destination: &mut Vec<u8>,
    ) -> Result<usize, OneShotError> {
        self.decode_append_inner(
            input,
            destination,
            |output, required| {
                output
                    .try_reserve_exact(required)
                    .map_err(|_| OneShotError::AllocationFailed {
                        requested: required,
                    })
            },
            |_| Ok(()),
        )
    }

    fn encode_append_inner<R, H>(
        &self,
        input: &[u8],
        destination: &mut String,
        reserve: R,
        mut after_chunk: H,
    ) -> Result<usize, OneShotError>
    where
        R: FnOnce(&mut String, usize) -> Result<(), OneShotError>,
        H: FnMut(&mut String, usize) -> Result<(), OneShotError>,
    {
        let required = self.encoded_len(input.len())?;
        destination
            .len()
            .checked_add(required)
            .ok_or(OneShotError::LengthOverflow)?;
        reserve(destination, required)?;

        let mut rollback = StringRollback::new(destination);
        for chunk in self.encoded_chunks(input)? {
            let text = chunk
                .as_str()
                .map_err(|_| OneShotError::Backend(BackendFault::ImpossibleState))?;
            rollback.destination().push_str(text);
            after_chunk(rollback.destination(), text.len())?;
        }
        rollback.commit();
        Ok(required)
    }

    fn decode_append_inner<R, H>(
        &self,
        input: &[u8],
        destination: &mut Vec<u8>,
        reserve: R,
        after_decode: H,
    ) -> Result<usize, OneShotError>
    where
        R: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
        H: FnOnce(&mut Vec<u8>) -> Result<(), OneShotError>,
    {
        let required = self.decoded_len(input)?;
        let original_len = destination.len();
        let total = original_len
            .checked_add(required)
            .ok_or(OneShotError::LengthOverflow)?;
        reserve(destination, required)?;

        let mut rollback = VecRollback::new(destination);
        rollback.destination().resize(total, 0);
        self.decode_into(input, &mut rollback.destination()[original_len..total])?;
        after_decode(rollback.destination())?;
        rollback.commit();
        Ok(required)
    }

    #[cfg(test)]
    pub(super) fn encode_append_with_hooks<R, H>(
        &self,
        input: &[u8],
        destination: &mut String,
        reserve: R,
        after_chunk: H,
    ) -> Result<usize, OneShotError>
    where
        R: FnOnce(&mut String, usize) -> Result<(), OneShotError>,
        H: FnMut(&mut String, usize) -> Result<(), OneShotError>,
    {
        self.encode_append_inner(input, destination, reserve, after_chunk)
    }

    #[cfg(test)]
    pub(super) fn decode_append_with_hooks<R, H>(
        &self,
        input: &[u8],
        destination: &mut Vec<u8>,
        reserve: R,
        after_decode: H,
    ) -> Result<usize, OneShotError>
    where
        R: FnOnce(&mut Vec<u8>, usize) -> Result<(), OneShotError>,
        H: FnOnce(&mut Vec<u8>) -> Result<(), OneShotError>,
    {
        self.decode_append_inner(input, destination, reserve, after_decode)
    }
}

struct StringRollback<'a> {
    destination: &'a mut String,
    original_len: usize,
    committed: bool,
}

impl<'a> StringRollback<'a> {
    fn new(destination: &'a mut String) -> Self {
        let original_len = destination.len();
        Self {
            destination,
            original_len,
            committed: false,
        }
    }

    fn destination(&mut self) -> &mut String {
        self.destination
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for StringRollback<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.destination.truncate(self.original_len);
        }
    }
}

struct VecRollback<'a> {
    destination: &'a mut Vec<u8>,
    original_len: usize,
    committed: bool,
}

impl<'a> VecRollback<'a> {
    fn new(destination: &'a mut Vec<u8>) -> Self {
        let original_len = destination.len();
        Self {
            destination,
            original_len,
            committed: false,
        }
    }

    fn destination(&mut self) -> &mut Vec<u8> {
        self.destination
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for VecRollback<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.destination.truncate(self.original_len);
        }
    }
}
