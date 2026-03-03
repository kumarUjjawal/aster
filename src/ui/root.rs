use crate::commands::{
    CloseWindow, FontSizeDecrease, FontSizeIncrease, FontSizeReset, NewFile, OpenFile, SaveFile,
    SaveFileAs,
};
use crate::model::document::DocumentState;
use crate::model::inline_markdown::InlineMarkdownState;
use crate::services::fs::{
    pick_open_markdown_path_async, pick_save_path_async, read_to_string, write_atomic,
};
use crate::services::inline_markdown::compute_inline_spans;
use crate::services::settings::{self, Settings};
use crate::services::tasks::Debouncer;
use crate::ui::editor::EditorView;
use crate::ui::file_explorer::FileExplorerView;
use crate::ui::theme::Theme;

use camino::Utf8PathBuf;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::notification::NotificationList;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::time::Duration;

const INLINE_SYNC_PARSE_MAX_BYTES: usize = 64 * 1024;

pub struct RootView {
    document: Entity<DocumentState>,
    inline_markdown: Entity<InlineMarkdownState>,
    editor_view: Entity<crate::ui::editor::EditorView>,
    file_explorer_view: Entity<crate::ui::file_explorer::FileExplorerView>,
    notifications: Entity<NotificationList>,
    inline_debounce: Debouncer<RootView>,
    /// Highest document revision for which an inline parse has been scheduled.
    scheduled_inline_revision: u64,
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
        inline_markdown: Entity<InlineMarkdownState>,
        editor_view: Entity<crate::ui::editor::EditorView>,
        file_explorer_view: Entity<crate::ui::file_explorer::FileExplorerView>,
        notifications: Entity<NotificationList>,
    ) -> Self {
        Self {
            document,
            inline_markdown,
            editor_view,
            file_explorer_view,
            notifications,
            inline_debounce: Debouncer::new(Duration::from_millis(35)),
            scheduled_inline_revision: 0,
            cached_doc_text: None,
            font_size: settings::get_font_size(),
            sidebar_width: 200.0,
            resizing_sidebar: false,
        }
    }

    pub fn new_document() -> DocumentState {
        DocumentState::new_empty()
    }

    pub fn new_inline_markdown() -> InlineMarkdownState {
        InlineMarkdownState::new()
    }

    pub fn build_editor(
        document: Entity<DocumentState>,
        inline_markdown: Entity<InlineMarkdownState>,
    ) -> crate::ui::editor::EditorView {
        EditorView::new(document, inline_markdown)
    }

    pub fn build_file_explorer(
        document: Entity<DocumentState>,
    ) -> crate::ui::file_explorer::FileExplorerView {
        FileExplorerView::new(document)
    }

    fn save_document(&mut self, cx: &mut Context<Self>, force_save_as: bool) {
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
                    let contents_result =
                        this.update(&mut *cx, |this, cx| this.document.read(cx).text());

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
        })
        .detach();
    }

    /// Synchronous save for when we have a path and window context
    fn do_save_to_path_sync(&mut self, mut path: Utf8PathBuf, cx: &mut Context<Self>) {
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
    fn open_path_internal(&mut self, path: &camino::Utf8PathBuf, cx: &mut Context<Self>) {
        match read_to_string(path) {
            Ok(text) => {
                let _ = self.document.update(cx, |d, cx| {
                    d.path = Some(path.clone());
                    d.set_text(&text);
                    d.clear_undo_history();
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
            d.clear_undo_history();
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

        let picker = pick_open_markdown_path_async();
        cx.spawn(async move |this, cx| {
            if let Some(utf8_path) = picker.await {
                let _ = this.update(&mut *cx, |this, cx| {
                    this.open_path_internal(&utf8_path, cx);
                });
            }
        })
        .detach();
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

    fn action_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.confirm_can_discard_changes(window, cx, "Save changes before closing?") {
            return;
        }
        window.remove_window();
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let inline_rev = self.inline_markdown.read(cx).source_revision;

        if doc_revision != inline_rev && self.scheduled_inline_revision < doc_revision {
            self.scheduled_inline_revision = doc_revision;
            let last_edit = self.document.read(cx).last_edit.clone();
            let target_rev = doc_revision;
            if doc_text.len() <= INLINE_SYNC_PARSE_MAX_BYTES {
                // Small/medium notes: parse inline to avoid style flicker between keystrokes.
                let parsed = compute_inline_spans(&doc_text, last_edit.as_ref());
                let _ = self.inline_markdown.update(cx, |state, cx| {
                    if target_rev >= state.source_revision {
                        state.spans = std::sync::Arc::new(parsed.spans);
                        state.source_revision = target_rev;
                        state.parse_millis = parsed.parse_millis;
                        cx.notify();
                    } else {
                        state.dropped_updates = state.dropped_updates.saturating_add(1);
                    }
                });
            } else {
                // Large notes: debounce and parse in background to protect typing latency.
                let text = doc_text.clone();
                let inline_markdown = self.inline_markdown.clone();
                self.inline_debounce.schedule(cx, move |_, cx| {
                    let text = text.clone();
                    let last_edit = last_edit.clone();
                    let inline_markdown = inline_markdown.clone();
                    cx.spawn(async move |_, cx| {
                        let parsed = cx
                            .background_executor()
                            .spawn(async move { compute_inline_spans(&text, last_edit.as_ref()) })
                            .await;
                        let _ = inline_markdown.update(cx, |state, cx| {
                            if target_rev >= state.source_revision {
                                state.spans = std::sync::Arc::new(parsed.spans);
                                state.source_revision = target_rev;
                                state.parse_millis = parsed.parse_millis;
                                cx.notify();
                            } else {
                                state.dropped_updates = state.dropped_updates.saturating_add(1);
                            }
                        });
                    })
                    .detach();
                });
            }
        }

        let (inline_parse_millis, inline_dropped_updates) = {
            let inline = self.inline_markdown.read(cx);
            (inline.parse_millis, inline.dropped_updates)
        };
        // Expose inline parser timing in status for quick perf monitoring.
        let status_right = if inline_dropped_updates > 0 {
            format!(
                "{} words · inline {:.1} ms · dropped {}",
                word_count, inline_parse_millis, inline_dropped_updates
            )
        } else {
            format!(
                "{} words · inline {:.1} ms",
                word_count, inline_parse_millis
            )
        };
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

        let top_chrome = div()
            .id("window-chrome")
            .h(px(38.))
            .w_full()
            .bg(Theme::panel())
            .border_b_1()
            .border_color(Theme::border())
            .flex_shrink_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, window, _| {
                    window.start_window_move();
                }),
            );

        let resize_line_color = if self.resizing_sidebar {
            gpui::rgba(0x2d7fd299)
        } else {
            Theme::border()
        };

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
            .child(
                div().w_full().flex().justify_end().child(
                    div()
                        .text_sm()
                        .text_color(Theme::muted())
                        .overflow_hidden()
                        .max_w(px(640.))
                        .child(status_right),
                ),
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
                this.font_size =
                    Settings::clamp_font_size(this.font_size + Settings::FONT_SIZE_STEP);
                settings::set_font_size(this.font_size);
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &FontSizeDecrease, _window, cx| {
                this.font_size =
                    Settings::clamp_font_size(this.font_size - Settings::FONT_SIZE_STEP);
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
            .child(top_chrome)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .min_w(px(0.))
                    .flex()
                    .flex_row()
                    .child({
                        // Keep the sidebar width in sync with the resize state
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
                            .w(px(1.))
                            .h_full()
                            .cursor_col_resize()
                            .bg(resize_line_color)
                            .hover(|s| s.bg(gpui::rgba(0x2d7fd24d)))
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
                            .child(self.editor_view.clone()),
                    ),
            )
            .child(bottom_bar)
            .child(self.notifications.clone())
    }
}
