use gpui::{div, px, IntoElement, ParentElement, Rgba, SharedString, Styled};

pub fn tag(text: impl Into<SharedString>, color: Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .bg(color)
        .rounded(px(6.))
        .px(px(8.))
        .py(px(4.))
        .text_sm()
        .child(text.into())
}
