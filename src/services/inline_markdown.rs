use crate::model::document::EditDelta;
use crate::services::syntax::SyntaxSpan;
use crate::services::syntax::markdown_spans;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct InlineParseResult {
    pub spans: Vec<SyntaxSpan>,
    pub parse_millis: f32,
}

/// Computes markdown syntax spans for inline editor presentation.
///
/// This runs on a background thread and returns immutable span snapshots keyed
/// by document revision in the caller.
pub fn compute_inline_spans(source: &str, _last_edit: Option<&EditDelta>) -> InlineParseResult {
    let started = Instant::now();
    let _delta_bounds = _last_edit.map(|d| {
        (
            d.start_char,
            d.old_end_char,
            d.new_end_char,
            d.start_byte,
            d.old_end_byte,
            d.new_end_byte,
        )
    });
    let spans = markdown_spans(source);
    let parse_millis = started.elapsed().as_secs_f32() * 1000.0;
    InlineParseResult {
        spans,
        parse_millis,
    }
}
