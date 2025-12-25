use crate::ui::root::RootView;
use gpui::{px, size, App, AppContext, Application, Bounds, WindowBounds, WindowOptions};

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.), px(650.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let document = cx.new(|_| RootView::new_document());
                    let preview = cx.new(|_| RootView::new_preview());
                    let editor_view = cx.new(|_| RootView::build_editor(document.clone()));
                    let preview_view = cx.new(|_| RootView::build_preview(preview.clone()));
                    RootView::new(document, preview, editor_view, preview_view)
                })
            },
        )
        .expect("failed to open window");
    });
}
