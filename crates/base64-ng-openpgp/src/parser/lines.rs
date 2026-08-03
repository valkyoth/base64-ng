use crate::{OpenPgpError, OpenPgpErrorKind};

#[derive(Clone, Copy)]
pub(super) struct Line<'a> {
    pub bytes: &'a [u8],
    pub start: usize,
    pub span_len: usize,
}

pub(super) struct Lines<'a> {
    input: &'a [u8],
    cursor: usize,
    max_line: usize,
}

impl<'a> Lines<'a> {
    pub const fn new(input: &'a [u8], max_line: usize) -> Self {
        Self {
            input,
            cursor: 0,
            max_line,
        }
    }

    pub fn next_line(&mut self) -> Result<Option<Line<'a>>, OpenPgpError> {
        if self.cursor == self.input.len() {
            return Ok(None);
        }
        let start = self.cursor;
        while self.cursor < self.input.len() && !matches!(self.input[self.cursor], b'\r' | b'\n') {
            self.cursor += 1;
            if self.cursor - start > self.max_line {
                return Err(OpenPgpError::at(
                    OpenPgpErrorKind::PhysicalLineTooLong,
                    start,
                ));
            }
        }
        let end = self.cursor;
        if self.cursor < self.input.len() {
            if self.input[self.cursor] == b'\r'
                && self.cursor + 1 < self.input.len()
                && self.input[self.cursor + 1] == b'\n'
            {
                self.cursor += 2;
            } else {
                self.cursor += 1;
            }
        }
        Ok(Some(Line {
            bytes: &self.input[start..end],
            start,
            span_len: self.cursor - start,
        }))
    }
}
