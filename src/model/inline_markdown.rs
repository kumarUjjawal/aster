use crate::services::syntax::SyntaxSpan;
use std::sync::Arc;

/// Incremental inline-markdown presentation state consumed by the editor.
#[derive(Clone)]
pub struct InlineMarkdownState {
    pub spans: Arc<Vec<SyntaxSpan>>,
    pub source_revision: u64,
    pub parse_millis: f32,
    pub dropped_updates: u64,
}

impl InlineMarkdownState {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(Vec::new()),
            source_revision: 0,
            parse_millis: 0.0,
            dropped_updates: 0,
        }
    }
}
