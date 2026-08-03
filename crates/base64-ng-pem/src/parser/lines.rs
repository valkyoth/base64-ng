use crate::{PemError, PemErrorKind};

#[derive(Clone, Copy)]
pub(super) struct Line<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) start: usize,
    pub(super) span_len: usize,
    pub(super) ending: LineEnding,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum LineEnding {
    CrLf,
    Cr,
    Lf,
    None,
}

pub(super) struct Lines<'a> {
    input: &'a [u8],
    cursor: usize,
    max_line_bytes: usize,
}

impl<'a> Lines<'a> {
    pub(super) const fn new(input: &'a [u8], max_line_bytes: usize) -> Self {
        Self {
            input,
            cursor: 0,
            max_line_bytes,
        }
    }

    pub(super) fn next_line(&mut self) -> Result<Option<Line<'a>>, PemError> {
        if self.cursor == self.input.len() {
            return Ok(None);
        }
        let start = self.cursor;
        let mut end = start;
        while end < self.input.len() && !matches!(self.input[end], b'\r' | b'\n') {
            end += 1;
        }
        if end - start > self.max_line_bytes {
            return Err(PemError::at(PemErrorKind::PhysicalLineTooLong, start));
        }
        let (ending, ending_len) = if end == self.input.len() {
            (LineEnding::None, 0)
        } else if self.input[end] == b'\r' && self.input.get(end + 1) == Some(&b'\n') {
            (LineEnding::CrLf, 2)
        } else if self.input[end] == b'\r' {
            (LineEnding::Cr, 1)
        } else {
            (LineEnding::Lf, 1)
        };
        self.cursor = end + ending_len;
        Ok(Some(Line {
            bytes: &self.input[start..end],
            start,
            span_len: end - start + ending_len,
            ending,
        }))
    }
}
