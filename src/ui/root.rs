use crate::model::document::DocumentState;
use crate::model::preview::PreviewState;
use crate::services::fs::{pick_open_path, pick_save_path, read_to_string, write_atomic};
use crate::services::markdown::render_blocks;
use crate::services::tasks::Debouncer;
use crate::ui::editor::EditorView;
use crate::ui::preview::PreviewView;
use crate::ui::theme::Theme;
use crate::ui::widgets::tag;
use gpui::{
    App, Context, Entity, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    Styled, Window, div, px,
};
use gpui_component::notification::{Notification, NotificationList};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::time::Duration;

pub struct RootView {
    document: Entity<DocumentState>,
    preview: Entity<PreviewState>,
    editor_view: Entity<crate::ui::editor::EditorView>,
    preview_view: Entity<crate::ui::preview::PreviewView>,
    notifications: Entity<NotificationList>,
    preview_debounce: Debouncer<RootView>,
}

impl RootView {
    pub fn new(
        document: Entity<DocumentState>,
        preview: Entity<PreviewState>,
        editor_view: Entity<crate::ui::editor::EditorView>,
        preview_view: Entity<crate::ui::preview::PreviewView>,
        notifications: Entity<NotificationList>,
    ) -> Self {
        Self {
            document,
            preview,
            editor_view,
            preview_view,
            notifications,
            preview_debounce: Debouncer::new(Duration::from_millis(120)),
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

    fn save_document(
        doc: &Entity<DocumentState>,
        window: &mut Window,
        cx_app: &mut App,
        notifications: &Entity<NotificationList>,
        force_save_as: bool,
    ) -> bool {
        let current_path = doc.read_with(cx_app, |d, _| d.path.clone());
        let target = if force_save_as {
            pick_save_path(current_path.as_ref())
        } else {
            current_path.or_else(|| pick_save_path(None))
        };

        let Some(path) = target else {
            return false;
        };

        let contents = doc.read_with(cx_app, |d, _| d.text());
        match write_atomic(&path, &contents) {
            Ok(()) => {
                let _ = doc.update(cx_app, |d, cx| {
                    d.path = Some(path.clone());
                    d.save_snapshot();
                    cx.notify();
                });
                let _ = notifications.update(cx_app, |list, cx| {
                    list.push(
                        Notification::success(format!(
                            "Saved {}",
                            path.file_name().unwrap_or("file")
                        )),
                        window,
                        cx,
                    );
                });
                true
            }
            Err(err) => {
                let _ = notifications.update(cx_app, |list, cx| {
                    list.push(
                        Notification::error(format!("Failed to save {}: {}", path, err)),
                        window,
                        cx,
                    );
                });
                false
            }
        }
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let doc_info = {
            let doc = self.document.read(cx);
            let cursor = doc.cursor.min(doc.rope.len_chars());
            let selection = doc.selection_range();
            let line_idx = doc.rope.char_to_line(cursor);
            let col = cursor.saturating_sub(doc.rope.line_to_char(line_idx));
            let line_count = doc.rope.len_lines();
            let char_count = doc.rope.len_chars();
            (
                doc.path.clone(),
                doc.dirty,
                doc.revision,
                doc.text(), // cheap enough for now
                cursor,
                selection,
                line_idx,
                col,
                line_count,
                char_count,
            )
        };
        let preview_rev = self.preview.read(cx).source_revision;

        if doc_info.2 != preview_rev {
            let text = doc_info.3.clone();
            let preview = self.preview.clone();
            let target_rev = doc_info.2;
            self.preview_debounce.schedule(cx, move |_, cx| {
                let blocks = render_blocks(&text);
                preview.update(cx, |p, cx| {
                    if target_rev >= p.source_revision {
                        p.blocks = blocks;
                        p.source_revision = target_rev;
                        cx.notify();
                    }
                });
            });
        }

        // Wire global shortcuts for open/save.
        let path_display = doc_info
            .0
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "untitled.md".to_string());
        let status_tag = if doc_info.1 {
            tag("dirty", Theme::warn())
        } else {
            tag("saved", Theme::muted())
        };
        let word_count = doc_info.3.split_whitespace().count();
        let selection_stats = doc_info.5.as_ref().map(|range| range.end - range.start);
        let status_right = match selection_stats {
            Some(selected_chars) => format!(
                "Ln {}, Col {} | {} lines | {} words | {} chars | Sel {}",
                doc_info.6 + 1,
                doc_info.7 + 1,
                doc_info.8,
                word_count,
                doc_info.9,
                selected_chars
            ),
            None => format!(
                "Ln {}, Col {} | {} lines | {} words | {} chars",
                doc_info.6 + 1,
                doc_info.7 + 1,
                doc_info.8,
                word_count,
                doc_info.9
            ),
        };
        let bounds = window.bounds();
        let w: f32 = bounds.size.width.into();
        let h: f32 = bounds.size.height.into();

        div()
            .relative()
            .flex()
            .flex_col()
            .bg(Theme::bg())
            .text_color(Theme::text())
            .w(px(w))
            .h(px(h))
            .on_key_down({
                let doc = self.document.clone();
                let notifications = self.notifications.clone();
                move |event: &KeyDownEvent, window: &mut Window, cx_app: &mut App| {
                    let ks = &event.keystroke;
                    let key = ks.key.to_lowercase();
                    let is_cmd = ks.modifiers.platform || ks.modifiers.control;
                    let shift = ks.modifiers.shift;
                    if !is_cmd {
                        return;
                    }
                    match key.as_str() {
                        "o" => {
                            let is_dirty = doc.read_with(cx_app, |d, _| d.dirty);
                            if is_dirty {
                                let result = MessageDialog::new()
                                    .set_level(MessageLevel::Warning)
                                    .set_title("Unsaved changes")
                                    .set_description("Save changes before opening another file?")
                                    .set_buttons(MessageButtons::YesNoCancelCustom(
                                        "Save".to_string(),
                                        "Don't Save".to_string(),
                                        "Cancel".to_string(),
                                    ))
                                    .show();

                                match result {
                                    MessageDialogResult::Ok | MessageDialogResult::Yes => {
                                        if !RootView::save_document(
                                            &doc,
                                            window,
                                            cx_app,
                                            &notifications,
                                            false,
                                        ) {
                                            return;
                                        }
                                    }
                                    MessageDialogResult::No => {}
                                    MessageDialogResult::Custom(label) => match label.as_str() {
                                        "Save" => {
                                            if !RootView::save_document(
                                                &doc,
                                                window,
                                                cx_app,
                                                &notifications,
                                                false,
                                            ) {
                                                return;
                                            }
                                        }
                                        "Don't Save" => {}
                                        _ => return,
                                    },
                                    _ => return,
                                }
                            }
                            if let Some(path) = pick_open_path() {
                                match read_to_string(&path) {
                                    Ok(text) => {
                                        let _ = doc.update(cx_app, |d, cx| {
                                            d.path = Some(path.clone());
                                            d.set_text(&text);
                                            d.save_snapshot();
                                            cx.notify();
                                        });
                                        let _ = notifications.update(cx_app, |list, cx| {
                                            list.push(
                                                Notification::success(format!(
                                                    "Opened {}",
                                                    path.file_name().unwrap_or("file")
                                                )),
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                    Err(err) => {
                                        let _ = notifications.update(cx_app, |list, cx| {
                                            list.push(
                                                Notification::error(format!(
                                                    "Failed to open {}: {}",
                                                    path, err
                                                )),
                                                window,
                                                cx,
                                            );
                                        });
                                    }
                                }
                            }
                        }
                        "s" => {
                            let _ = RootView::save_document(
                                &doc,
                                window,
                                cx_app,
                                &notifications,
                                shift,
                            );
                        }
                        _ => {}
                    }
                }
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(16.))
                    .py(px(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(format!("File: {}", path_display))
                            .child(status_tag),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(Theme::muted())
                            .child(status_right),
                    ),
            )
            .child(
                div()
                    .flex_grow()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .p(px(16.))
                    .child(self.editor_view.clone())
                    .child(self.preview_view.clone()),
            )
            .child(self.notifications.clone())
    }
}
