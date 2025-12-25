use crate::model::preview::PreviewState;
use crate::services::markdown::{Block, InlineRun};
use crate::ui::theme::Theme;
use gpui::{
    div, px, Context, Entity, FontWeight, IntoElement, ParentElement, Render, SharedString, Styled,
    Window,
};

pub struct PreviewView {
    preview: Entity<PreviewState>,
}

impl PreviewView {
    pub fn new(preview: Entity<PreviewState>) -> Self {
        Self { preview }
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks = self.preview.read(cx).blocks.clone();
        div()
            .flex_grow()
            .min_w(px(320.))
            .bg(Theme::panel_alt())
            .border_1()
            .border_color(Theme::border())
            .p(px(18.))
            .text_sm()
            .text_color(Theme::text())
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(blocks.into_iter().map(render_block)),
            )
    }
}

fn render_block(block: Block) -> impl IntoElement {
    match block {
        Block::Heading(level, runs) => {
            let mut el = div().text_color(Theme::text());
            el = match level {
                1 => el.text_2xl().font_weight(FontWeight::BOLD).text_color(Theme::accent()),
                2 => el.text_xl().font_weight(FontWeight::BOLD).text_color(Theme::accent()),
                _ => el.text_lg().font_weight(FontWeight::BOLD).text_color(Theme::accent()),
            };
            el.child(render_inline_runs(runs))
        }
        Block::Paragraph(runs) => div().child(render_inline_runs(runs)),
        Block::ListItem(runs) => div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().text_color(Theme::accent()).text_lg().child("•"))
            .child(render_inline_runs(runs)),
        Block::CodeBlock(text) => div()
            .font_family("Menlo")
            .bg(Theme::border())
            .p(px(10.))
            .rounded(px(4.))
            .child(SharedString::from(text)),
        Block::Quote(runs) => div()
            .flex()
            .gap_2()
            .child(div().w(px(4.)).bg(Theme::strong()))
            .child(
                div()
                    .text_color(Theme::muted())
                    .italic()
                    .child(render_inline_runs(runs)),
            ),
    }
}

fn render_inline_runs(runs: Vec<InlineRun>) -> impl IntoElement {
    let lines = split_runs(runs);
    div().flex().flex_col().gap_1().children(lines.into_iter().map(|line| {
        div()
            .flex()
            .flex_row()
            .gap_1()
            .children(line.into_iter().map(render_inline_run))
    }))
}

fn render_inline_run(r: InlineRun) -> impl IntoElement {
    let mut span = div().child(SharedString::from(r.text));
    if r.bold {
        span = span.font_weight(FontWeight::BOLD);
    }
    if r.italic {
        span = span.italic();
    }
    if r.code {
        span = span
            .font_family("Menlo")
            .bg(Theme::border())
            .rounded(px(4.))
            .px(px(4.))
            .py(px(2.));
    }
    if r.link.is_some() {
        span = span.text_color(Theme::accent()).underline();
    }
    span
}

fn split_runs(runs: Vec<InlineRun>) -> Vec<Vec<InlineRun>> {
    let mut lines: Vec<Vec<InlineRun>> = vec![Vec::new()];
    for run in runs {
        let parts: Vec<&str> = run.text.split('\n').collect();
        for (idx, part) in parts.iter().enumerate() {
            if idx > 0 {
                lines.push(Vec::new());
            }
            if !part.is_empty() {
                lines.last_mut().unwrap().push(InlineRun {
                    text: part.to_string(),
                    bold: run.bold,
                    italic: run.italic,
                    code: run.code,
                    link: run.link.clone(),
                });
            }
        }
    }
    lines
}
