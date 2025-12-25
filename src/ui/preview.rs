use crate::model::preview::PreviewState;
use crate::services::markdown::BlockKind;
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
            .min_w(px(200.))
            .bg(Theme::panel_alt())
            .border_1()
            .border_color(Theme::border())
            .p(px(12.))
            .text_sm()
            .text_color(Theme::text())
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(blocks.into_iter().map(|b| {
                        let mut row = div().text_color(Theme::text());
                        row = match b.kind {
                            BlockKind::Heading(level) => match level {
                                1 => row.text_2xl().font_weight(FontWeight::BOLD),
                                2 => row.text_xl().font_weight(FontWeight::BOLD),
                                _ => row.text_lg().font_weight(FontWeight::BOLD),
                            },
                            BlockKind::CodeBlock => row
                                .font_family("Menlo")
                                .bg(Theme::border())
                                .p(px(8.))
                                .rounded(px(4.)),
                            BlockKind::ListItem => row,
                            BlockKind::Paragraph => row,
                        };
                        if b.bold {
                            row = row.font_weight(FontWeight::BOLD);
                        }
                        if b.italic {
                            row = row.italic();
                        }
                        row.child(SharedString::from(b.text))
                    })),
            )
    }
}
