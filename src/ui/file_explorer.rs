use crate::model::document::DocumentState;
use crate::ui::text_utils::ellipsize_chars;
use crate::ui::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, px,
};

#[derive(Clone, Debug)]
struct OutlineItem {
    ordinal: usize,
    level: u32,
    title: String,
    byte_start: usize,
}

pub struct FileExplorerView {
    document: Entity<DocumentState>,
    outline_scroll_handle: ScrollHandle,
    width: f32,
    cached_outline: Option<(u64, Vec<OutlineItem>)>,
}

impl FileExplorerView {
    pub fn new(document: Entity<DocumentState>) -> Self {
        Self {
            document,
            outline_scroll_handle: ScrollHandle::new(),
            width: 200.0,
            cached_outline: None,
        }
    }

    pub fn set_width(&mut self, width: f32, cx: &mut gpui::Context<Self>) {
        self.width = width;
        cx.notify();
    }
}

impl Render for FileExplorerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let doc_revision = self.document.read(cx).revision;
        let outline_items = if let Some((cached_revision, items)) = &self.cached_outline {
            if *cached_revision == doc_revision {
                items.clone()
            } else {
                let text = self.document.read(cx).text();
                let parsed = parse_outline_items(&text);
                self.cached_outline = Some((doc_revision, parsed.clone()));
                parsed
            }
        } else {
            let text = self.document.read(cx).text();
            let parsed = parse_outline_items(&text);
            self.cached_outline = Some((doc_revision, parsed.clone()));
            parsed
        };
        let has_outline = !outline_items.is_empty();
        let document = self.document.clone();

        let outline_elements: Vec<_> = outline_items
            .into_iter()
            .map(|item| {
                let ordinal = item.ordinal;
                let level = item.level;
                let title = item.title;
                let byte_start = item.byte_start;
                let indent = (level.saturating_sub(1) as f32) * 10.0;
                let document = document.clone();
                div()
                    .id(("outline-entry", ordinal))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pl(px(8. + indent))
                    .pr(px(8.))
                    .py(px(3.))
                    .cursor_pointer()
                    .hover(|this| this.bg(Theme::panel_alt()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _: &MouseDownEvent, _, cx| {
                            let _ = document.update(cx, |doc, cx| {
                                let cursor = doc.byte_to_char(byte_start);
                                doc.set_cursor(cursor);
                                cx.notify();
                            });
                        }),
                    )
                    .child(
                        div()
                            .w(px(4.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(Theme::accent())
                            .flex_shrink_0(),
                    )
                    .child(
                        div()
                            .text_sm()
                            .overflow_hidden()
                            .flex_1()
                            .text_color(Theme::text())
                            .child(ellipsize_chars(&title, 64)),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(px(self.width))
            .bg(Theme::sidebar())
            .flex_shrink_0()
            .child(
                div()
                    .px(px(10.))
                    .py(px(6.))
                    .text_xs()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(Theme::muted())
                    .child("OUTLINE"),
            )
            .child(
                div()
                    .id("outline-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.outline_scroll_handle)
                    .when(has_outline, |this| this.children(outline_elements))
                    .when(!has_outline, |this| {
                        this.child(
                            div()
                                .px(px(10.))
                                .py(px(8.))
                                .text_sm()
                                .text_color(Theme::muted())
                                .child("No headings"),
                        )
                    }),
            )
    }
}

fn parse_outline_items(text: &str) -> Vec<OutlineItem> {
    let mut items = Vec::new();
    let mut byte_offset = 0usize;

    for raw_line in text.split_inclusive('\n') {
        let line = raw_line.trim_end_matches('\n').trim_end_matches('\r');
        if line.is_empty() {
            byte_offset += raw_line.len();
            continue;
        }

        let leading = leading_whitespace_bytes(line);
        let content = &line[leading..];
        if let Some((marker_len, level)) = heading_prefix(content) {
            let heading_start = byte_offset + leading;
            let text_start = heading_start + marker_len;
            if text_start <= byte_offset + line.len() {
                let title = text[text_start..(byte_offset + line.len())]
                    .trim()
                    .to_string();
                if !title.is_empty() {
                    items.push(OutlineItem {
                        ordinal: items.len(),
                        level: level as u32,
                        title,
                        byte_start: heading_start,
                    });
                }
            }
        }

        byte_offset += raw_line.len();
    }

    items
}

fn leading_whitespace_bytes(line: &str) -> usize {
    line.char_indices()
        .find_map(|(idx, ch)| if ch.is_whitespace() { None } else { Some(idx) })
        .unwrap_or(line.len())
}

fn heading_prefix(content: &str) -> Option<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i] == b'#' {
        i += 1;
    }
    let hash_count = i;
    if hash_count == 0 || hash_count > 6 {
        return None;
    }
    if i < bytes.len() && bytes[i].is_ascii_whitespace() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        return Some((i, hash_count));
    }
    None
}
