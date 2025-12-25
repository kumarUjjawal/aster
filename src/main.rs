use gpui::{
    div, prelude::*, px, rgb, App, Application, Bounds, Context, SharedString, Window,
    WindowBounds, WindowOptions,
};

fn main() {
    // Minimal gpui window with split placeholders; real editor/preview wired later.
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, gpui::size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| RootView::default()),
        )
        .expect("failed to open window");
    });
}

#[derive(Default)]
struct RootView {
    title: SharedString,
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .bg(rgb(0x121212))
            .text_color(rgb(0xffffff))
            .child(
                div()
                    .p(px(12.))
                    .text_lg()
                    .child(
                        if self.title.is_empty() {
                            SharedString::from("Markdown Editor (stub)")
                        } else {
                            self.title.clone()
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .p(px(12.))
                    .child(
                        div()
                            .flex_grow()
                            .min_w(px(200.))
                            .bg(rgb(0x1f1f1f))
                            .border_1()
                            .border_color(rgb(0x2f2f2f))
                            .p(px(12.))
                            .text_sm()
                            .child("Editor pane placeholder"),
                    )
                    .child(
                        div()
                            .flex_grow()
                            .min_w(px(200.))
                            .bg(rgb(0x181818))
                            .border_1()
                            .border_color(rgb(0x2f2f2f))
                            .p(px(12.))
                            .text_sm()
                            .child("Preview pane placeholder"),
                    ),
            )
    }
}
