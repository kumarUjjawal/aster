use std::ops::Range;

/// Represents a single edit operation that can be undone/redone
#[derive(Clone)]
pub struct EditOperation {
    /// Full text before the edit
    pub old_text: String,
    /// Full text after the edit
    pub new_text: String,
    /// Cursor position before the edit
    pub old_cursor: usize,
    /// Cursor position after the edit
    pub new_cursor: usize,
    /// Selection range before edit (if any)
    pub old_selection: Option<Range<usize>>,
    /// Selection range after edit (if any)
    pub new_selection: Option<Range<usize>>,
}

/// Manages undo/redo history for document editing
#[derive(Clone)]
pub struct UndoHistory {
    /// Stack of undo-able operations (most recent at end)
    undo_stack: Vec<EditOperation>,
    /// Stack of redo-able operations (most recent at end)
    redo_stack: Vec<EditOperation>,
    /// Maximum number of operations to keep
    max_history: usize,
}

impl UndoHistory {
    /// Create a new undo history with the given maximum size
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    /// Create with default limit of 100 operations
    pub fn default() -> Self {
        Self::new(100)
    }

    /// Push a new operation onto the undo stack
    /// Clears the redo stack and enforces the history limit
    pub fn push(&mut self, op: EditOperation) {
        self.undo_stack.push(op);
        self.redo_stack.clear();

        // Enforce history limit
        while self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
    }

    /// Pop an operation from the undo stack and push to redo stack
    /// Returns the operation if available
    pub fn undo(&mut self) -> Option<EditOperation> {
        if let Some(op) = self.undo_stack.pop() {
            self.redo_stack.push(op.clone());
            Some(op)
        } else {
            None
        }
    }

    /// Pop an operation from the redo stack and push to undo stack
    /// Returns the operation if available
    pub fn redo(&mut self) -> Option<EditOperation> {
        if let Some(op) = self.redo_stack.pop() {
            self.undo_stack.push(op.clone());
            Some(op)
        } else {
            None
        }
    }

    /// Clear all history (called when opening a new file)
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Check if undo is available
    #[allow(dead_code)]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    #[allow(dead_code)]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of undo operations available
    #[allow(dead_code)]
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo operations available
    #[allow(dead_code)]
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}
