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
        }
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.cursor = self.rope.len_chars();
        self.bump_revision();
        self.dirty = self.current_hash() != self.last_saved_hash;
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx, text);
        self.bump_revision();
        self.dirty = true;
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        if range.start >= range.end || range.end > self.rope.len_chars() {
            return;
        }
        self.rope.remove(range);
        self.bump_revision();
        self.dirty = true;
        self.cursor = self.cursor.min(self.rope.len_chars());
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
