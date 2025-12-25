use crate::model::preview::PreviewState;
use crate::ui::theme::Theme;
use gpui::{div, px, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window};

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
        let rendered: SharedString = self.preview.read(cx).rendered.clone();
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
            .child(rendered)
    }
}
