use crate::commands::{
    About, CloseWindow, Copy, Cut, NewFile, OpenFile, Paste, Quit, SaveFile, SaveFileAs, SelectAll,
};
use crate::services::assets::AsterAssetSource;
use crate::services::fs::{read_to_string, write_atomic};
use crate::ui::root::RootView;
use camino::Utf8PathBuf;
use gpui::{
    App, AppContext, Application, Bounds, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType,
    Window, WindowBounds, WindowOptions, px, size,
};
use gpui_component::notification::NotificationList;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use url::Url;

pub fn run() {
    let pending_urls: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let pending_urls_for_callback = pending_urls.clone();

    let app = Application::new().with_assets(AsterAssetSource::new());
    app.on_open_urls(move |urls| {
        let mut queue = pending_urls_for_callback
            .lock()
            .expect("open url queue lock poisoned");
        queue.extend(urls);
    });

    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        cx.activate(true);

        cx.bind_keys([
            KeyBinding::new("cmd-n", NewFile, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("cmd-s", SaveFile, None),
            KeyBinding::new("shift-cmd-s", SaveFileAs, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-x", Cut, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
            KeyBinding::new("cmd-a", SelectAll, None),
        ]);

        cx.set_menus(vec![
            Menu {
                name: "Aster".into(),
                items: vec![
                    MenuItem::action("About Aster", About),
                    MenuItem::separator(),
                    MenuItem::os_submenu("Services", SystemMenuType::Services),
                    MenuItem::separator(),
                    MenuItem::action("Quit Aster", Quit),
                ],
            },
            Menu {
                name: "File".into(),
                items: vec![
                    MenuItem::action("New", NewFile),
                    MenuItem::action("Open…", OpenFile),
                    MenuItem::separator(),
                    MenuItem::action("Save", SaveFile),
                    MenuItem::action("Save As…", SaveFileAs),
                    MenuItem::separator(),
                    MenuItem::action("Close Window", CloseWindow),
                ],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Cut", Cut, OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, OsAction::Paste),
                    MenuItem::separator(),
                    MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                ],
            },
        ]);

        cx.on_action(|_: &Quit, cx| {
            let windows = cx.window_stack().unwrap_or_else(|| cx.windows());

            for window in windows.iter().copied() {
                let Some(handle) = window.downcast::<RootView>() else {
                    continue;
                };

                let can_quit = handle
                    .update(cx, |root, window, cx| root.confirm_before_quit(window, cx))
                    .unwrap_or(true);
                if !can_quit {
                    return;
                }
            }

            // Close windows ourselves (bypasses `on_window_should_close`) and then quit.
            for window in windows {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            }

            cx.quit();
        });
        cx.on_action(|_: &About, _cx| {
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title("About Aster")
                .set_description(format!(
                    "Aster\n\nVersion {}\n\nA lightweight Markdown editor built with Rust + GPUI.",
                    env!("CARGO_PKG_VERSION")
                ))
                .set_buttons(MessageButtons::Ok)
                .show();
        });

        let _ = open_window(cx, None);

        let args: Vec<String> = std::env::args().skip(1).collect();
        for arg in args {
            if let Some(path) = parse_open_target(&arg) {
                open_path_in_active_window_or_new(cx, path);
            }
        }

        let queue = pending_urls.clone();
        cx.to_async()
            .spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(200))
                        .await;
                    let urls: Vec<String> = {
                        let mut q = queue.lock().expect("open url queue lock poisoned");
                        let mut drained = Vec::new();
                        while let Some(u) = q.pop_front() {
                            drained.push(u);
                        }
                        drained
                    };

                    if urls.is_empty() {
                        continue;
                    }

                    let _ = cx.update(|cx| {
                        for url in urls {
                            if let Some(path) = parse_open_target(&url) {
                                open_path_in_active_window_or_new(cx, path);
                            }
                        }
                    });
                }
            })
            .detach();
    });
}

fn open_window(cx: &mut App, initial_path: Option<Utf8PathBuf>) -> anyhow::Result<()> {
    let bounds = Bounds::centered(None, size(px(900.), px(650.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| build_root_view(window, cx, initial_path.clone()),
    )?;
    Ok(())
}

fn build_root_view(
    window: &mut Window,
    cx: &mut App,
    initial_path: Option<Utf8PathBuf>,
) -> gpui::Entity<RootView> {
    let document = cx.new(|_| RootView::new_document());
    let preview = cx.new(|_| RootView::new_preview());
    let file_tree = cx.new(|_| RootView::new_file_tree());
    let notifications = cx.new(|cx| NotificationList::new(window, cx));
    let editor_view = cx.new(|_| RootView::build_editor(document.clone()));
    let preview_view = cx.new(|_| RootView::build_preview(preview.clone()));
    let file_explorer_view = cx.new(|_| RootView::build_file_explorer(file_tree.clone()));

    // Initialize file tree with current working directory
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(utf8_cwd) = Utf8PathBuf::try_from(cwd) {
            let _ = file_tree.update(cx, |tree, cx| {
                tree.set_root(utf8_cwd, cx);
            });
        }
    }

    if let Some(path) = initial_path.as_ref() {
        if let Ok(text) = read_to_string(path) {
            let _ = document.update(cx, |d, cx| {
                d.path = Some(path.clone());
                d.set_text(&text);
                d.save_snapshot();
                cx.notify();
            });
        }
    }

    install_should_close_prompt(window, cx, document.clone());
    cx.new(|_| RootView::new(document, preview, file_tree, editor_view, preview_view, file_explorer_view, notifications))
}

fn install_should_close_prompt(
    window: &mut Window,
    cx: &mut App,
    document: gpui::Entity<crate::model::document::DocumentState>,
) {
    window.on_window_should_close(cx, {
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
                // Only save if we have an existing path - avoid blocking file dialog
                let Some(path) = current_path else {
                    // No path - need to use Save As, which requires async dialog
                    // Cancel the close and notify user to save first
                    MessageDialog::new()
                        .set_level(MessageLevel::Info)
                        .set_title("Save required")
                        .set_description("Please use Save As (Cmd+Shift+S) to save this file first.")
                        .set_buttons(MessageButtons::Ok)
                        .show();
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
                            .set_description(format!("Failed to save {}: {}", path, err))
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
}

fn parse_open_target(raw: &str) -> Option<Utf8PathBuf> {
    if let Ok(url) = Url::parse(raw) {
        if url.scheme() == "file" {
            if let Ok(path) = url.to_file_path() {
                return Utf8PathBuf::try_from(path).ok();
            }
        } else {
            return None;
        }
    }

    Utf8PathBuf::try_from(std::path::PathBuf::from(raw)).ok()
}

fn open_path_in_active_window_or_new(cx: &mut App, path: Utf8PathBuf) {
    if let Some(active_window) = cx.active_window() {
        if let Some(handle) = active_window.downcast::<RootView>() {
            let _ = handle.update(cx, |root, window, cx| {
                root.action_open_path(path.clone(), window, cx);
            });
            return;
        }
    }

    let _ = open_window(cx, Some(path));
}
