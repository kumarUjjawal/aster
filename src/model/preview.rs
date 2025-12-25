use gpui::SharedString;

#[derive(Clone)]
pub struct PreviewState {
    pub rendered: SharedString,
    pub source_revision: u64,
}

impl PreviewState {
    pub fn new() -> Self {
        Self {
            rendered: SharedString::from(""),
            source_revision: 0,
        }
    }
}
