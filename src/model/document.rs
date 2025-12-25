use camino::Utf8PathBuf;
use ropey::Rope;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::{collections::hash_map::DefaultHasher};

#[derive(Clone)]
pub struct DocumentState {
    pub path: Option<Utf8PathBuf>,
    pub rope: Rope,
    pub dirty: bool,
    pub revision: u64,
    pub last_saved_hash: u64,
    pub cursor: usize,
    pub selection: Option<Range<usize>>, // character indices
    pub selection_anchor: Option<usize>, // starting point for shift/drag selections
}

impl DocumentState {
    pub fn new_empty() -> Self {
        Self {
            path: None,
            rope: Rope::new(),
            dirty: false,
            revision: 0,
            last_saved_hash: 0,
            cursor: 0,
            selection: None,
            selection_anchor: None,
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.cursor = self.rope.len_chars();
        self.clear_selection();
        self.bump_revision();
        self.dirty = self.current_hash() != self.last_saved_hash;
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn set_cursor(&mut self, idx: usize) {
        self.cursor = idx.min(self.len_chars());
        self.clear_selection();
    }

    pub fn set_selection(&mut self, start: usize, end: usize) {
        let (start, end) = if start <= end { (start, end) } else { (end, start) };
        self.selection = if start == end {
            None
        } else {
            Some(start.min(self.len_chars())..end.min(self.len_chars()))
        };
        self.cursor = end.min(self.len_chars());
        self.selection_anchor = Some(start.min(self.len_chars()));
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
    }

    pub fn selection_range(&self) -> Option<Range<usize>> {
        self.selection.clone()
    }

    pub fn selection_bytes(&self) -> Option<Range<usize>> {
        self.selection.clone().map(|r| self.char_range_to_bytes(r))
    }

    pub fn delete_selection(&mut self) -> Option<usize> {
        if let Some(range) = self.selection.clone() {
            self.delete_range(range.clone());
            let new_cursor = range.start.min(self.len_chars());
            self.cursor = new_cursor;
            self.clear_selection();
            Some(new_cursor)
        } else {
            None
        }
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
        self.bump_revision();
        self.dirty = true;
        self.clear_selection();
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        if range.start >= range.end || range.end > self.rope.len_chars() {
            return;
        }
        self.rope.remove(range);
        self.bump_revision();
        self.dirty = true;
        self.cursor = self.cursor.min(self.rope.len_chars());
        self.clear_selection();
    }

    pub fn select_all(&mut self) {
        let len = self.len_chars();
        self.selection = if len == 0 { None } else { Some(0..len) };
        self.selection_anchor = Some(0);
        self.cursor = len;
    }

    pub fn char_to_byte(&self, char_idx: usize) -> usize {
        let clamped = char_idx.min(self.len_chars());
        self.rope.char_to_byte(clamped)
    }

    pub fn byte_to_char(&self, byte_idx: usize) -> usize {
        let clamped = byte_idx.min(self.len_bytes());
        self.rope.byte_to_char(clamped)
    }

    pub fn char_range_to_bytes(&self, range: Range<usize>) -> Range<usize> {
        let start = self.char_to_byte(range.start);
        let end = self.char_to_byte(range.end);
        start..end
    }

    pub fn slice_chars(&self, range: Range<usize>) -> String {
        self.rope.slice(range).to_string()
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn save_snapshot(&mut self) {
        self.last_saved_hash = self.current_hash();
        self.dirty = false;
    }

    fn current_hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.rope.hash(&mut h);
        h.finish()
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
