use tokio::io;

pub(crate) struct OutputQueue<const CAP: usize> {
    buffer: [u8; CAP],
    start: usize,
    len: usize,
}

impl<const CAP: usize> OutputQueue<CAP> {
    pub(crate) const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            start: 0,
            len: 0,
        }
    }

    pub(crate) const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) const fn len(&self) -> usize {
        self.len
    }

    pub(crate) const fn available_capacity(&self) -> usize {
        CAP - self.len
    }

    pub(crate) fn push_slice(&mut self, input: &[u8]) -> io::Result<()> {
        if input.len() > self.available_capacity() {
            return Err(io::Error::other(
                "base64-ng-tokio output queue capacity exceeded",
            ));
        }

        let mut read = 0;
        while read < input.len() {
            let write = (self.start + self.len) % CAP;
            self.buffer[write] = input[read];
            self.len += 1;
            read += 1;
        }

        Ok(())
    }

    pub(crate) fn copy_front(&self, output: &mut [u8]) -> usize {
        let count = core::cmp::min(self.len, output.len());
        let first = core::cmp::min(count, CAP - self.start);
        output[..first].copy_from_slice(&self.buffer[self.start..self.start + first]);

        let second = count - first;
        if second > 0 {
            output[first..first + second].copy_from_slice(&self.buffer[..second]);
        }

        count
    }

    pub(crate) fn discard_front(&mut self, count: usize) {
        let count = core::cmp::min(count, self.len);
        let first = core::cmp::min(count, CAP - self.start);
        crate::wipe_bytes(&mut self.buffer[self.start..self.start + first]);

        let second = count - first;
        if second > 0 {
            crate::wipe_bytes(&mut self.buffer[..second]);
        }

        self.start = (self.start + count) % CAP;
        self.len -= count;
        if self.len == 0 {
            self.start = 0;
        }
    }

    pub(crate) fn clear_all(&mut self) {
        crate::wipe_bytes(&mut self.buffer);
        self.start = 0;
        self.len = 0;
    }
}

impl<const CAP: usize> Drop for OutputQueue<CAP> {
    fn drop(&mut self) {
        self.clear_all();
    }
}

#[cfg(test)]
mod tests {
    use super::OutputQueue;

    #[test]
    fn discard_and_clear_zero_retained_storage() {
        let mut queue = OutputQueue::<8>::new();
        queue.push_slice(b"secrets!").unwrap();
        queue.discard_front(3);
        assert!(queue.buffer[..3].iter().all(|byte| *byte == 0));
        queue.clear_all();
        assert!(queue.buffer.iter().all(|byte| *byte == 0));
        assert_eq!(queue.start, 0);
        assert_eq!(queue.len, 0);
    }
}
