use crate::services::fs::{pick_save_path, write_atomic};
use crate::ui::root::RootView;
use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_component::notification::NotificationList;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

pub fn run() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(900.), px(650.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let document = cx.new(|_| RootView::new_document());
                let preview = cx.new(|_| RootView::new_preview());
                let notifications = cx.new(|cx| NotificationList::new(window, cx));
                let editor_view = cx.new(|_| RootView::build_editor(document.clone()));
                let preview_view = cx.new(|_| RootView::build_preview(preview.clone()));

                window.on_window_should_close(cx, {
                    let document = document.clone();
                    move |_, cx| {
                        let is_dirty = document.read_with(cx, |d, _| d.dirty);
                        if !is_dirty {
                            return true;
                        }

                        let choice = MessageDialog::new()
                            .set_level(MessageLevel::Warning)
                            .set_title("Unsaved changes")
                            .set_description("Save changes before closing?")
                            .set_buttons(MessageButtons::YesNoCancelCustom(
                                "Save".to_string(),
                                "Don't Save".to_string(),
                                "Cancel".to_string(),
                            ))
                            .show();

                        let mut save = || {
                            let current_path = document.read_with(cx, |d, _| d.path.clone());
                            let target = current_path.or_else(|| pick_save_path(None));
                            let Some(path) = target else {
                                return false;
                            };

                            let contents = document.read_with(cx, |d, _| d.text());
                            match write_atomic(&path, &contents) {
                                Ok(()) => {
                                    let _ = document.update(cx, |d, cx| {
                                        d.path = Some(path.clone());
                                        d.save_snapshot();
                                        cx.notify();
                                    });
                                    true
                                }
                                Err(err) => {
                                    MessageDialog::new()
                                        .set_level(MessageLevel::Error)
                                        .set_title("Save failed")
                                        .set_description(format!(
                                            "Failed to save {}: {}",
                                            path, err
                                        ))
                                        .set_buttons(MessageButtons::Ok)
                                        .show();
                                    false
                                }
                            }
                        };

                        match choice {
                            MessageDialogResult::Ok | MessageDialogResult::Yes => save(),
                            MessageDialogResult::No => true,
                            MessageDialogResult::Custom(label) => match label.as_str() {
                                "Save" => save(),
                                "Don't Save" => true,
                                _ => false,
                            },
                            _ => false,
                        }
                    }
                });

                cx.new(|_| {
                    RootView::new(document, preview, editor_view, preview_view, notifications)
                })
            },
        )
        .expect("failed to open window");
    });
}
