use crate::model::preview::PreviewState;
use crate::services::markdown::{Block, InlineRun};
use crate::ui::theme::Theme;
use gpui::{
    App, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Window, div, point, px,
};

pub struct PreviewView {
    preview: Entity<PreviewState>,
    focus_handle: Option<FocusHandle>,
    scroll_handle: ScrollHandle,
}

impl PreviewView {
    pub fn new(preview: Entity<PreviewState>) -> Self {
        Self {
            preview,
            focus_handle: None,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks = self.preview.read(cx).blocks.clone();
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let scroll_handle = self.scroll_handle.clone();

        div()
            .id("preview_scroll")
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(Theme::panel_alt())
            .p(px(18.))
            .text_sm()
            .text_color(Theme::text())
            .overflow_y_scroll()
            .overflow_x_hidden()
            .scrollbar_width(px(10.))
            .track_scroll(&self.scroll_handle)
            .track_focus(&focus_handle)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                move |_: &MouseDownEvent, window: &mut Window, _cx: &mut App| {
                    focus_handle.focus(window);
                }
            })
            .on_key_down({
                let focus_handle = focus_handle.clone();
                let scroll_handle = scroll_handle.clone();
                move |event: &KeyDownEvent, window: &mut Window, _cx: &mut App| {
                    if !focus_handle.is_focused(window) {
                        return;
                    }

                    let key = event.keystroke.key.to_lowercase();
                    let modifiers = event.keystroke.modifiers;
                    let is_cmd = modifiers.platform || modifiers.control;

                    if is_cmd {
                        return;
                    }

                    if key == "pageup" || key == "pagedown" {
                        let max = scroll_handle.max_offset();
                        let offset = scroll_handle.offset();
                        let bounds = scroll_handle.bounds();
                        let page = bounds.size.height;
                        if page > px(0.) {
                            let amount = page * 0.9;
                            let delta = if key == "pagedown" { -amount } else { amount };
                            let mut new_offset = offset;
                            new_offset.y = (new_offset.y + delta).clamp(-max.height, px(0.));
                            scroll_handle.set_offset(point(new_offset.x, new_offset.y));
                            window.refresh();
                        }
                    }
                }
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_start()
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
                1 => el
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
                2 => el
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
                _ => el
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
            };
            el.child(render_inline_runs(runs))
        }
        Block::Paragraph(runs) => div().child(render_inline_runs(runs)),
        Block::ListItem(runs) => div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().text_color(Theme::accent()).text_lg().child("•"))
            .child(div().flex_1().min_w(px(0.)).child(render_inline_runs(runs))),
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
                    .flex_1()
                    .min_w(px(0.))
                    .text_color(Theme::muted())
                    .italic()
                    .child(render_inline_runs(runs)),
            ),
    }
}

fn render_inline_runs(runs: Vec<InlineRun>) -> impl IntoElement {
    let lines = split_runs(runs);
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(lines.into_iter().map(|line| {
            div()
                .flex()
                .flex_row()
                .flex_wrap()
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
