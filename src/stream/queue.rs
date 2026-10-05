use std::io;

pub(super) struct OutputQueue<const CAP: usize> {
    buffer: [u8; CAP],
    start: usize,
    len: usize,
}

impl<const CAP: usize> OutputQueue<CAP> {
    pub(super) const fn new() -> Self {
        Self {
            buffer: [0; CAP],
            start: 0,
            len: 0,
        }
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(super) const fn len(&self) -> usize {
        self.len
    }

    pub(super) const fn capacity(&self) -> usize {
        self.len + self.available_capacity()
    }

    pub(super) fn push_slice(&mut self, input: &[u8]) -> io::Result<()> {
        if input.len() > self.available_capacity() {
            return Err(io::Error::other(
                "base64 stream output queue capacity exceeded",
            ));
        }

        if !input.is_empty() {
            let write = (self.start + self.len) % CAP;
            let first = input.len().min(CAP - write);
            self.buffer[write..write + first].copy_from_slice(&input[..first]);
            self.buffer[..input.len() - first].copy_from_slice(&input[first..]);
            self.len += input.len();
        }

        Ok(())
    }

    pub(super) fn copy_front(&self, output: &mut [u8]) -> usize {
        let count = core::cmp::min(self.len, output.len());
        let first = core::cmp::min(count, CAP - self.start);
        output[..first].copy_from_slice(&self.buffer[self.start..self.start + first]);

        let second = count - first;
        if second > 0 {
            output[first..first + second].copy_from_slice(&self.buffer[..second]);
        }

        count
    }

    pub(super) fn discard_front(&mut self, count: usize) {
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

    pub(super) fn pop_slice(&mut self, output: &mut [u8]) -> usize {
        let count = self.copy_front(output);
        self.discard_front(count);
        count
    }

    pub(super) fn clear_all(&mut self) {
        crate::wipe_bytes(&mut self.buffer);
        self.start = 0;
        self.len = 0;
    }

    pub(super) const fn available_capacity(&self) -> usize {
        CAP - self.len
    }
}

#[cfg(test)]
mod bulk_tests {
    use super::OutputQueue;

    #[test]
    fn bulk_queue_wrap_capacity_and_wiping_match_a_linear_queue() {
        for discarded in 0..=16 {
            for appended in 0..=discarded {
                let mut queue = OutputQueue::<16>::new();
                queue.push_slice(&[1; 16]).unwrap();
                queue.discard_front(discarded);
                assert!(queue.buffer[..discarded].iter().all(|byte| *byte == 0));
                queue.push_slice(&vec![2; appended]).unwrap();
                let mut actual = [0xa5; 16];
                let len = queue.copy_front(&mut actual);
                let mut expected = vec![1; 16 - discarded];
                expected.extend_from_slice(&vec![2; appended]);
                assert_eq!(&actual[..len], expected);
                let before = queue.buffer;
                assert!(queue.push_slice(&[3; 17]).is_err());
                assert_eq!(queue.buffer, before);
                queue.clear_all();
                assert_eq!(queue.buffer, [0; 16]);
            }
        }
        let mut empty = OutputQueue::<0>::new();
        empty.push_slice(&[]).unwrap();
        assert!(empty.push_slice(&[1]).is_err());
    }
}
