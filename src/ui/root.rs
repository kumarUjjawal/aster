use crate::commands::{CloseWindow, FontSizeDecrease, FontSizeIncrease, FontSizeReset, NewFile, OpenFile, OpenFolder, SaveFile, SaveFileAs};
use crate::model::document::DocumentState;
use crate::model::file_tree::FileTreeState;
use crate::model::preview::PreviewState;
use crate::services::fs::{pick_folder_async, pick_open_path_async, pick_save_path_async, read_to_string, write_atomic};
use crate::services::markdown::render_blocks;
use crate::services::settings::{self, Settings};
use crate::services::tasks::Debouncer;
use crate::ui::editor::EditorView;
use crate::ui::file_explorer::FileExplorerView;
use crate::ui::preview::PreviewView;
use crate::ui::theme::Theme;

use camino::Utf8PathBuf;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement, Render, Styled, Window, div, px, svg,
};
use gpui_component::{IconName, IconNamed};
use gpui_component::notification::NotificationList;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ViewMode {
    Split,
    Editor,
    Preview,
}

pub struct RootView {
    document: Entity<DocumentState>,
    preview: Entity<PreviewState>,
    file_tree: Entity<FileTreeState>,
    editor_view: Entity<crate::ui::editor::EditorView>,
    preview_view: Entity<crate::ui::preview::PreviewView>,
    file_explorer_view: Entity<crate::ui::file_explorer::FileExplorerView>,
    notifications: Entity<NotificationList>,
    preview_debounce: Debouncer<RootView>,
    view_mode: ViewMode,
    /// Cached document text to avoid O(n) rope-to-string conversion every frame
    cached_doc_text: Option<(u64, String)>,
    /// Current font size in points (8-32)
    font_size: f32,
    /// Current sidebar width in pixels
    sidebar_width: f32,
    /// Whether we're currently resizing the sidebar
    resizing_sidebar: bool,
}

impl RootView {
    pub fn new(
        document: Entity<DocumentState>,
        preview: Entity<PreviewState>,
        file_tree: Entity<FileTreeState>,
        editor_view: Entity<crate::ui::editor::EditorView>,
        preview_view: Entity<crate::ui::preview::PreviewView>,
        file_explorer_view: Entity<crate::ui::file_explorer::FileExplorerView>,
        notifications: Entity<NotificationList>,
    ) -> Self {
        Self {
            document,
            preview,
            file_tree,
            editor_view,
            preview_view,
            file_explorer_view,
            notifications,
            preview_debounce: Debouncer::new(Duration::from_millis(200)),
            view_mode: ViewMode::Split,
            cached_doc_text: None,
            font_size: settings::get_font_size(),
            sidebar_width: 200.0,
            resizing_sidebar: false,
        }
    }

    pub fn new_document() -> DocumentState {
        DocumentState::new_empty()
    }

    pub fn new_preview() -> PreviewState {
        PreviewState::new()
    }

    pub fn build_editor(document: Entity<DocumentState>) -> crate::ui::editor::EditorView {
        EditorView::new(document)
    }

    pub fn build_preview(preview: Entity<PreviewState>) -> crate::ui::preview::PreviewView {
        PreviewView::new(preview)
    }

    pub fn new_file_tree() -> FileTreeState {
        FileTreeState::new()
    }

    pub fn build_file_explorer(file_tree: Entity<FileTreeState>) -> crate::ui::file_explorer::FileExplorerView {
        FileExplorerView::new(file_tree)
    }

    fn save_document(
        &mut self,
        cx: &mut Context<Self>,
        force_save_as: bool,
    ) {
        let current_path = self.document.read(cx).path.clone();
        
        // If we have a path and not forcing save-as, save directly
        if !force_save_as {
            if let Some(path) = current_path {
                self.do_save_to_path_sync(path, cx);
                return;
            }
        }
        
        // Need to show file picker - use async dialog
        let receiver = pick_save_path_async(cx, current_path.as_ref());
        
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(path))) = receiver.await {
                if let Ok(mut utf8_path) = Utf8PathBuf::try_from(path) {
                    if utf8_path.extension().is_none() {
                        utf8_path.set_extension("md");
                    }
                    
                    // Read document contents and write synchronously
                    let contents_result = this.update(&mut *cx, |this, cx| {
                        this.document.read(cx).text()
                    });
                    
                    if let Ok(contents) = contents_result {
                        if write_atomic(&utf8_path, &contents).is_ok() {
                            let _ = this.update(&mut *cx, |this, cx| {
                                let _ = this.document.update(cx, |d, cx| {
                                    d.path = Some(utf8_path.clone());
                                    d.save_snapshot();
                                    cx.notify();
                                });
                                cx.add_recent_document(utf8_path.as_std_path());
                                // Note: Notifications require window context, skipping in async
                            });
                        }
                    }
                }
            }
        }).detach();
    }
    
    /// Synchronous save for when we have a path and window context
    fn do_save_to_path_sync(
        &mut self,
        mut path: Utf8PathBuf,
        cx: &mut Context<Self>,
    ) {
        if path.extension().is_none() {
            path.set_extension("md");
        }

        let contents = self.document.read(cx).text();
        match write_atomic(&path, &contents) {
            Ok(()) => {
                let _ = self.document.update(cx, |d, cx| {
                    d.path = Some(path.clone());
                    d.save_snapshot();
                    cx.notify();
                });
                cx.add_recent_document(path.as_std_path());
                // Skip notification here too - simplifies and avoids window context issues
            }
            Err(_err) => {
                // Silently fail for now - window context not available for notification 
            }
        }
    }

    fn confirm_can_discard_changes(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
        prompt: &str,
    ) -> bool {
        let is_dirty = self.document.read(cx).dirty;
        if !is_dirty {
            return true;
        }

        let choice = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("Unsaved changes")
            .set_description(prompt)
            .set_buttons(MessageButtons::YesNoCancelCustom(
                "Save".to_string(),
                "Don't Save".to_string(),
                "Cancel".to_string(),
            ))
            .show();

        let save_sync = |this: &mut Self, cx: &mut Context<Self>| -> bool {
            // Only save synchronously if we have an existing path
            let current_path = this.document.read(cx).path.clone();
            if let Some(path) = current_path {
                this.do_save_to_path_sync(path, cx);
                true
            } else {
                // No path - need async dialog, cancel for now
                // Start async save in background
                this.save_document(cx, false);
                false
            }
        };

        match choice {
            MessageDialogResult::Ok | MessageDialogResult::Yes => save_sync(self, cx),
            MessageDialogResult::No => true,
            MessageDialogResult::Custom(label) => match label.as_str() {
                "Save" => save_sync(self, cx),
                "Don't Save" => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn open_path(
        &mut self,
        path: &camino::Utf8PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_path_internal(path, cx);
    }
    
    /// Internal open path that doesn't require window - for async context
    fn open_path_internal(
        &mut self,
        path: &camino::Utf8PathBuf,
        cx: &mut Context<Self>,
    ) {
        match read_to_string(path) {
            Ok(text) => {
                let _ = self.document.update(cx, |d, cx| {
                    d.path = Some(path.clone());
                    d.set_text(&text);
                    d.save_snapshot();
                    cx.notify();
                });
                cx.add_recent_document(path.as_std_path());
            }
            Err(_err) => {
                // Silently fail for async context - no window for notification
            }
        }
    }

    fn action_new_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_can_discard_changes(window, cx, "Save changes before creating a new file?")
        {
            return;
        }

        let _ = self.document.update(cx, |d, cx| {
            d.path = None;
            d.set_text("");
            d.save_snapshot();
            cx.notify();
        });
        // No notification for new file - only save gets a notification
    }

    fn action_open_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_can_discard_changes(
            window,
            cx,
            "Save changes before opening another file?",
        ) {
            return;
        }

        // Use async file picker
        let receiver = pick_open_path_async(cx);
        
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    if let Ok(utf8_path) = Utf8PathBuf::try_from(path) {
                        let _ = this.update(&mut *cx, |this, cx| {
                            this.open_path_internal(&utf8_path, cx);
                        });
                    }
                }
            }
        }).detach();
    }

    pub fn action_open_path(
        &mut self,
        path: camino::Utf8PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.confirm_can_discard_changes(
            window,
            cx,
            "Save changes before opening another file?",
        ) {
            return;
        }
        self.open_path(&path, window, cx);
    }

    pub fn confirm_before_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        self.confirm_can_discard_changes(window, cx, "Save changes before quitting?")
    }

    fn action_save(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.save_document(cx, false);
    }

    fn action_save_as(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.save_document(cx, true);
    }

    fn action_open_folder(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = pick_folder_async(cx);
        let file_tree = self.file_tree.clone();
        
        cx.spawn(async move |_, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    if let Ok(utf8_path) = Utf8PathBuf::try_from(path) {
                        let _ = file_tree.update(cx, |tree, cx| {
                            tree.set_root(utf8_path, cx);
                        });
                    }
                }
            }
        }).detach();
    }

    fn action_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_can_discard_changes(window, cx, "Save changes before closing?") {
            return;
        }
        window.remove_window();
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check if file explorer has a pending file to open
        if let Some(path) = self.file_tree.update(cx, |tree, _| tree.take_pending_open()) {
            self.open_path(&path, window, cx);
        }

        let (doc_path, doc_dirty, doc_revision, word_count) = {
            self.document.update(cx, |doc, _| {
                (
                    doc.path.clone(),
                    doc.dirty,
                    doc.revision,
                    doc.get_word_count(),
                )
            })
        };

        // Use cached text if revision hasn't changed to avoid O(n) rope conversion
        let doc_text = if let Some((cached_rev, ref text)) = self.cached_doc_text {
            if cached_rev == doc_revision {
                text.clone()
            } else {
                let text = self.document.read(cx).text();
                self.cached_doc_text = Some((doc_revision, text.clone()));
                text
            }
        } else {
            let text = self.document.read(cx).text();
            self.cached_doc_text = Some((doc_revision, text.clone()));
            text
        };
        let preview_rev = self.preview.read(cx).source_revision;

        if doc_revision != preview_rev {
            let text = doc_text.clone();
            let preview = self.preview.clone();
            let target_rev = doc_revision;
            self.preview_debounce.schedule(cx, move |_, cx| {
                // Clone values inside FnMut so they can be moved into async
                let text = text.clone();
                let preview = preview.clone();
                // Spawn async task to parse markdown in background
                cx.spawn(async move |_, cx| {
                    // Run render_blocks on background thread to avoid blocking UI
                    let parsed = cx.background_executor().spawn(async move {
                        render_blocks(&text)
                    }).await;
                    
                    // Update preview state on main thread
                    let _ = preview.update(cx, |p, cx| {
                        if target_rev >= p.source_revision {
                            p.blocks = std::sync::Arc::new(parsed.blocks);
                            p.footnotes = std::sync::Arc::new(parsed.footnotes);
                            p.source_revision = target_rev;
                            cx.notify();
                        }
                    });
                }).detach();
            });
        }

        // Use cached word count from document
        let status_right = format!("{} words", word_count);
        // Use size_full() instead of explicit pixel dimensions to ensure proper layout

        let window_title = {
            let name = doc_path
                .as_ref()
                .and_then(|p| p.file_name())
                .unwrap_or("untitled.md");
            let dirty = if doc_dirty { " •" } else { "" };
            format!("{name}{dirty} — Aster")
        };
        window.set_window_title(&window_title);

        let make_view_button = |id: &'static str, icon: IconName, target: ViewMode| {
            let selected = self.view_mode == target;
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .w(px(34.))
                .h(px(28.))
                .rounded(px(6.))
                .text_sm()
                .cursor_pointer()
                .when(selected, |this| {
                    this.bg(Theme::panel_alt()).text_color(Theme::text())
                })
                .when(!selected, |this| {
                    this.text_color(Theme::muted())
                        .hover(|this| this.bg(Theme::panel_alt()).text_color(Theme::text()))
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        this.view_mode = target;
                        cx.notify();
                    }),
                )
                .child(
                    svg()
                        .path(icon.path())
                        .size_4()
                        .text_color(if selected { Theme::text() } else { Theme::muted() })
                )
        };

        let view_controls = div()
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .child(make_view_button(
                "view-editor",
                IconName::PanelLeft,
                ViewMode::Editor,
            ))
            .child(make_view_button(
                "view-split",
                IconName::LayoutDashboard,
                ViewMode::Split,
            ))
            .child(make_view_button(
                "view-preview",
                IconName::PanelRight,
                ViewMode::Preview,
            ));



        let split_view = div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .when(self.view_mode != ViewMode::Preview, |this| {
                this.child(self.editor_view.clone())
            })
            .when(self.view_mode == ViewMode::Split, |this| {
                this.child(div().w(px(1.)).bg(Theme::border()).flex_shrink_0().h_full())
            })
            .when(self.view_mode != ViewMode::Editor, |this| {
                this.child(self.preview_view.clone())
            });

        let bottom_bar = div()
            .flex()
            .items_center()
            .gap_3()
            .px(px(16.))
            .py(px(4.))
            .bg(Theme::panel())
            .border_t_1()
            .border_color(Theme::border())
            .flex_shrink_0()
            .child(view_controls)
            .child(div().flex_1())
            .child(
                div()
                    .text_sm()
                    .text_color(Theme::muted())
                    .truncate()
                    .max_w(px(520.))
                    .child(status_right),
            );

        div()
            .relative()
            .flex()
            .flex_col()
            .bg(Theme::bg())
            .text_color(Theme::text())
            .size_full()
            .on_action(cx.listener(|this, _: &NewFile, window, cx| {
                this.action_new_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFile, window, cx| {
                this.action_open_file(window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenFolder, window, cx| {
                this.action_open_folder(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFile, window, cx| {
                this.action_save(window, cx);
            }))
            .on_action(cx.listener(|this, _: &SaveFileAs, window, cx| {
                this.action_save_as(window, cx);
            }))
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| {
                this.action_close_window(window, cx);
            }))
            .on_action(cx.listener(|this, _: &FontSizeIncrease, _window, cx| {
                this.font_size = Settings::clamp_font_size(this.font_size + Settings::FONT_SIZE_STEP);
                settings::set_font_size(this.font_size);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FontSizeDecrease, _window, cx| {
                this.font_size = Settings::clamp_font_size(this.font_size - Settings::FONT_SIZE_STEP);
                settings::set_font_size(this.font_size);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FontSizeReset, _window, cx| {
                this.font_size = Settings::DEFAULT_FONT_SIZE;
                settings::set_font_size(this.font_size);
                cx.notify();
            }))
            // Handle sidebar resize drag at root level so we don't lose events
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if !this.resizing_sidebar {
                    return;
                }
                let new_width: f32 = event.position.x.into();
                let clamped = new_width.clamp(100.0, 400.0);
                this.sidebar_width = clamped;
                cx.notify();
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.resizing_sidebar {
                        this.resizing_sidebar = false;
                        cx.notify();
                    }
                }),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .flex()
                    .flex_row()
                    .child({
                        // Update the file explorer width to match our state
                        let fe = self.file_explorer_view.clone();
                        let width = self.sidebar_width;
                        let _ = fe.update(cx, |view, cx| {
                            view.set_width(width, cx);
                        });
                        fe
                    })
                    // Resize handle
                    .child(
                        div()
                            .id("sidebar-resize-handle")
                            .w(px(4.))
                            .h_full()
                            .cursor_col_resize()
                            .bg(gpui::transparent_black())
                            .hover(|s| s.bg(gpui::hsla(0.0, 0.0, 0.5, 0.3)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                                    this.resizing_sidebar = true;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .min_w(px(0.))
                            .flex()
                            .flex_col()
                            .child(split_view),
                    ),
            )
            .child(bottom_bar)
            .child(self.notifications.clone())
    }
}
