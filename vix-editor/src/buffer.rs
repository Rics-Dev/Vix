use ropey::Rope;
use crate::edit::{EditLog, Op};

pub struct Buffer {
    pub rope: Rope,
    pub log:  EditLog,
    /// Whether the buffer has unsaved changes.
    pub dirty: bool,
}

impl Buffer {
    pub fn new() -> Self {
        Self { rope: Rope::new(), log: EditLog::default(), dirty: false }
    }

    pub fn from_str(s: &str) -> Self {
        Self { rope: Rope::from_str(s), log: EditLog::default(), dirty: false }
    }

    // ── core mutators ─────────────────────────────────────────────────────

    /// Insert `text` at byte offset `at`.
    pub fn insert(&mut self, at: usize, text: &str) {
        let char_idx = self.rope.byte_to_char(at);
        self.rope.insert(char_idx, text);
        let op = Op::Insert { at, text: text.to_owned() };
        self.log.push(op);
        self.dirty = true;
    }

    /// Delete the byte range `at..at+len`.
    pub fn delete(&mut self, at: usize, len: usize) {
        let start = self.rope.byte_to_char(at);
        let end   = self.rope.byte_to_char(at + len);
        let removed: String = self.rope.slice(start..end).into();
        self.rope.remove(start..end);
        let op = Op::Delete { at, text: removed };
        self.log.push(op);
        self.dirty = true;
    }

    // ── undo / redo ───────────────────────────────────────────────────────

    pub fn undo(&mut self) {
        if let Some(op) = self.log.undo() {
            self.apply_raw(&op);
        }
    }

    pub fn redo(&mut self) {
        if let Some(op) = self.log.redo() {
            self.apply_raw(&op);
        }
    }

    /// Apply an op WITHOUT recording it in the log.
    /// Used by undo/redo to replay inverse ops.
    fn apply_raw(&mut self, op: &Op) {
        match op {
            Op::Insert { at, text } => {
                let ci = self.rope.byte_to_char(*at);
                self.rope.insert(ci, text);
            }
            Op::Delete { at, text } => {
                let start = self.rope.byte_to_char(*at);
                let end   = start + text.chars().count();
                self.rope.remove(start..end);
            }
        }
        self.dirty = true;
    }

    // ── read helpers ──────────────────────────────────────────────────────

    pub fn len_bytes(&self) -> usize { self.rope.len_bytes() }
    pub fn len_lines(&self) -> usize { self.rope.len_lines() }

    pub fn line(&self, idx: usize) -> ropey::RopeSlice<'_> {
        self.rope.line(idx)
    }

    /// Byte offset of the start of `line_idx`.
    pub fn line_to_byte(&self, line_idx: usize) -> usize {
        self.rope.line_to_byte(line_idx)
    }

    pub fn byte_to_line(&self, byte: usize) -> usize {
        self.rope.byte_to_line(byte)
    }
}
