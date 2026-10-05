macro_rules! reader_observers {
    () => {
        /// Returns whether this adapter has entered its absorbing failure state.
        #[must_use]
        pub const fn is_failed(&self) -> bool {
            self.failed
        }

        /// Returns whether finalization completed successfully.
        #[must_use]
        pub const fn is_complete(&self) -> bool {
            self.finished && self.output_pos == self.output_len
        }

        /// Returns bytes irrevocably read from the wrapped source.
        #[must_use]
        pub const fn input_read(&self) -> usize {
            self.input_read
        }

        /// Returns bytes accepted by the shared Base64 transformer.
        #[must_use]
        pub const fn source_position(&self) -> usize {
            self.source_accepted
        }

        /// Returns output bytes already delivered to callers.
        #[must_use]
        pub const fn output_delivered(&self) -> usize {
            self.output_delivered
        }

        /// Returns remaining source bytes for an exact frame, or `None` for EOF mode.
        #[must_use]
        pub const fn remaining_input(&self) -> Option<usize> {
            self.boundary.remaining()
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Boundary {
    Eof,
    Exact { remaining: usize },
}

impl Boundary {
    pub(super) fn read_cap(self, capacity: usize) -> usize {
        match self {
            Self::Eof => capacity,
            Self::Exact { remaining } => remaining.min(capacity),
        }
    }

    pub(super) fn consume(&mut self, count: usize) {
        if let Self::Exact { remaining } = self {
            *remaining -= count;
        }
    }

    pub(super) const fn remaining(self) -> Option<usize> {
        match self {
            Self::Eof => None,
            Self::Exact { remaining } => Some(remaining),
        }
    }
}
