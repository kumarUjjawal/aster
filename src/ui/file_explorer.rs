use crate::model::preview::PreviewState;
use crate::services::markdown::Block;
use crate::ui::text_utils::ellipsize_chars;
use crate::ui::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, px,
};

pub struct FileExplorerView {
    preview: Entity<PreviewState>,
    outline_scroll_handle: ScrollHandle,
    width: f32,
}

impl FileExplorerView {
    pub fn new(preview: Entity<PreviewState>) -> Self {
        Self {
            preview,
            outline_scroll_handle: ScrollHandle::new(),
            width: 200.0,
        }
    }

    pub fn set_width(&mut self, width: f32, cx: &mut gpui::Context<Self>) {
        self.width = width;
        cx.notify();
    }
}

impl Render for FileExplorerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let outline_items: Vec<(usize, u32, String)> = self
            .preview
            .read(cx)
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Heading(level, runs) => {
                    let text = runs.iter().map(|r| r.text.as_str()).collect::<String>();
                    let title = text.trim().to_string();
                    if title.is_empty() {
                        None
                    } else {
                        Some((*level, title))
                    }
                }
                _ => None,
            })
            .enumerate()
            .map(|(ordinal, (level, title))| (ordinal, level, title))
            .collect();
        let has_outline = !outline_items.is_empty();
        let preview = self.preview.clone();

        let outline_elements: Vec<_> = outline_items
            .into_iter()
            .map(|(ordinal, level, title)| {
                let indent = (level.saturating_sub(1) as f32) * 10.0;
                let preview = preview.clone();
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
                            let _ = preview.update(cx, |preview, cx| {
                                preview.pending_outline_jump = Some(ordinal);
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
