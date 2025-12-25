use crate::services::markdown::Block;

#[derive(Clone)]
pub struct PreviewState {
    pub blocks: Vec<Block>,
    pub source_revision: u64,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            source_revision: 0,
        }
    }
}
