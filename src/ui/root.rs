use crate::model::document::DocumentState;
use crate::model::preview::PreviewState;
use crate::services::fs::{pick_open_path, pick_save_path, read_to_string, write_atomic};
use crate::services::markdown::render_blocks;
use crate::services::tasks::Debouncer;
use crate::ui::editor::EditorView;
use crate::ui::preview::PreviewView;
use crate::ui::theme::Theme;
use crate::ui::widgets::tag;
use gpui::{div, px, Context, Entity, InteractiveElement, KeyDownEvent, ParentElement, Render, Styled, Window};
use std::time::Duration;

pub struct RootView {
    document: Entity<DocumentState>,
    preview: Entity<PreviewState>,
    editor_view: Entity<crate::ui::editor::EditorView>,
    preview_view: Entity<crate::ui::preview::PreviewView>,
    preview_debounce: Debouncer<RootView>,
}

impl RootView {
    pub fn new(
        document: Entity<DocumentState>,
        preview: Entity<PreviewState>,
        editor_view: Entity<crate::ui::editor::EditorView>,
        preview_view: Entity<crate::ui::preview::PreviewView>,
    ) -> Self {
        Self {
            document,
            preview,
            editor_view,
            preview_view,
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
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let doc_info = {
            let doc = self.document.read(cx);
            (
                doc.path.clone(),
                doc.dirty,
                doc.revision,
                doc.text(), // cheap enough for now
            )
        };
        let preview_rev = self.preview.read(cx).source_revision;

        if doc_info.2 != preview_rev {
            let text = doc_info.3.clone();
            let blocks = render_blocks(&text);
            let preview = self.preview.clone();
            let target_rev = doc_info.2;
            self.preview_debounce.schedule(cx, move |_, cx| {
                preview.update(cx, |p, cx| {
                    if target_rev >= p.source_revision {
                        p.blocks = blocks.clone();
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

        div()
            .flex()
            .flex_col()
            .bg(Theme::bg())
            .text_color(Theme::text())
            .on_key_down({
                let doc = self.document.clone();
                move |event: &KeyDownEvent, _window: &mut Window, cx_app: &mut gpui::App| {
                    let ks = &event.keystroke;
                    let key = ks.key.to_lowercase();
                    let is_cmd = ks.modifiers.platform || ks.modifiers.control;
                    if !is_cmd {
                        return;
                    }
                    match key.as_str() {
                        "o" => {
                            if let Some(path) = pick_open_path() {
                                if let Ok(text) = read_to_string(&path) {
                                    let _ = doc.update(cx_app, |d, cx| {
                                        d.path = Some(path.clone());
                                        d.set_text(&text);
                                        d.save_snapshot();
                                        cx.notify();
                                    });
                                }
                            }
                        }
                        "s" => {
                            let maybe_path = doc.read_with(cx_app, |d, _| d.path.clone());
                            let target = maybe_path.or_else(|| pick_save_path(None));
                            if let Some(path) = target {
                                let contents = doc.read_with(cx_app, |d, _| d.text());
                                if write_atomic(&path, &contents).is_ok() {
                                    let _ = doc.update(cx_app, |d, cx| {
                                        d.path = Some(path.clone());
                                        d.save_snapshot();
                                        cx.notify();
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            })
            .child(
                div()
                    .p(px(16.))
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(format!("File: {}", path_display))
                    .child(status_tag),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .p(px(16.))
                    .child(self.editor_view.clone())
                    .child(self.preview_view.clone()),
            )
    }
}
