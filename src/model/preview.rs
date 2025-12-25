use crate::services::markdown::RenderBlock;

#[derive(Clone)]
pub struct PreviewState {
    pub blocks: Vec<RenderBlock>,
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
