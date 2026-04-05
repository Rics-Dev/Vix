use crate::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction { Left, Right, Up, Down }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity { Char, Word, Line }

/// A cursor is just a byte offset into the buffer, plus a "column hint"
/// for sticky vertical movement (like vim's `gj`/`gk`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Cursor {
    pub byte: usize,
    /// The column we try to stay on when moving vertically.
    /// Reset on horizontal moves, preserved on vertical ones.
    sticky_col: usize,
}

impl Cursor {
    pub fn new() -> Self { Self::default() }

    // ── movement ──────────────────────────────────────────────────────────

    pub fn move_by(&mut self, dir: Direction, gran: Granularity, buf: &Buffer) {
        match (dir, gran) {
            (Direction::Left,  Granularity::Char) => self.move_left_char(buf),
            (Direction::Right, Granularity::Char) => self.move_right_char(buf),
            (Direction::Left,  Granularity::Word) => self.move_left_word(buf),
            (Direction::Right, Granularity::Word) => self.move_right_word(buf),
            (Direction::Up,    _)                 => self.move_up(buf),
            (Direction::Down,  _)                 => self.move_down(buf),
            (Direction::Left,  Granularity::Line) => self.move_line_start(buf),
            (Direction::Right, Granularity::Line) => self.move_line_end(buf),
        }
    }

    fn move_left_char(&mut self, buf: &Buffer) {
        if self.byte == 0 { return; }
        // Step back one UTF-8 char boundary.
        let rope = &buf.rope;
        let ci   = rope.byte_to_char(self.byte);
        if ci > 0 {
            self.byte = rope.char_to_byte(ci - 1);
        }
        self.reset_sticky(buf);
    }

    fn move_right_char(&mut self, buf: &Buffer) {
        let rope = &buf.rope;
        let ci   = rope.byte_to_char(self.byte);
        if ci < rope.len_chars() {
            self.byte = rope.char_to_byte(ci + 1);
        }
        self.reset_sticky(buf);
    }

    fn move_left_word(&mut self, buf: &Buffer) {
        let rope = &buf.rope;
        let mut ci = rope.byte_to_char(self.byte);
        // skip trailing non-word chars, then skip the word
        while ci > 0 && !is_word_char(rope.char(ci - 1)) { ci -= 1; }
        while ci > 0 &&  is_word_char(rope.char(ci - 1)) { ci -= 1; }
        self.byte = rope.char_to_byte(ci);
        self.reset_sticky(buf);
    }

    fn move_right_word(&mut self, buf: &Buffer) {
        let rope = &buf.rope;
        let len  = rope.len_chars();
        let mut ci = rope.byte_to_char(self.byte);
        while ci < len && !is_word_char(rope.char(ci)) { ci += 1; }
        while ci < len &&  is_word_char(rope.char(ci)) { ci += 1; }
        self.byte = rope.char_to_byte(ci);
        self.reset_sticky(buf);
    }

    fn move_up(&mut self, buf: &Buffer) {
        let line = buf.byte_to_line(self.byte);
        if line == 0 { self.byte = 0; return; }
        self.byte = self.clamped_col_on_line(line - 1, buf);
    }

    fn move_down(&mut self, buf: &Buffer) {
        let line = buf.byte_to_line(self.byte);
        if line + 1 >= buf.len_lines() { return; }
        self.byte = self.clamped_col_on_line(line + 1, buf);
    }

    fn move_line_start(&mut self, buf: &Buffer) {
        let line = buf.byte_to_line(self.byte);
        self.byte = buf.line_to_byte(line);
        self.reset_sticky(buf);
    }

    fn move_line_end(&mut self, buf: &Buffer) {
        let line    = buf.byte_to_line(self.byte);
        let start   = buf.line_to_byte(line);
        let content = buf.line(line);
        // trim trailing newline from the length
        let trimmed = content.as_str()
            .unwrap_or("")
            .trim_end_matches('\n')
            .trim_end_matches("\r\n");
        self.byte = start + trimmed.len();
        self.reset_sticky(buf);
    }

    // ── helpers ───────────────────────────────────────────────────────────

    /// Column (byte offset within the line) at the current position.
    pub fn col(&self, buf: &Buffer) -> usize {
        let line  = buf.byte_to_line(self.byte);
        let start = buf.line_to_byte(line);
        self.byte - start
    }

    /// Clamp `sticky_col` to the length of `line_idx`, return the byte.
    fn clamped_col_on_line(&self, line_idx: usize, buf: &Buffer) -> usize {
        let start   = buf.line_to_byte(line_idx);
        let content = buf.line(line_idx);
        let line_str = content.as_str().unwrap_or("");
        let line_len = line_str.trim_end_matches('\n')
                                .trim_end_matches("\r\n")
                                .len();
        start + self.sticky_col.min(line_len)
    }

    fn reset_sticky(&mut self, buf: &Buffer) {
        self.sticky_col = self.col(buf);
    }

    pub fn clamp(&mut self, buf: &Buffer) {
        self.byte = self.byte.min(buf.len_bytes().saturating_sub(1));
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
