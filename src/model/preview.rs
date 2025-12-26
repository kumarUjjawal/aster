use crate::services::markdown::Block;
use std::sync::Arc;

#[derive(Clone)]
pub struct PreviewState {
    pub blocks: Arc<Vec<Block>>,
    pub footnotes: Arc<Vec<Block>>,
    pub source_revision: u64,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(Vec::new()),
            footnotes: Arc::new(Vec::new()),
            source_revision: 0,
        }
    }
}

