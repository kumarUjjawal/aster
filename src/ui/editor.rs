use crate::model::document::DocumentState;
use crate::ui::theme::Theme;
use gpui::{
    div, px, App, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, SharedString, Styled, Window,
};

pub struct EditorView {
    document: Entity<DocumentState>,
    focus_handle: Option<FocusHandle>,
}

impl EditorView {
    pub fn new(document: Entity<DocumentState>) -> Self {
        Self {
            document,
            focus_handle: None,
        }
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle
            .clone()
            .expect("focus handle should be initialized during render")
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let doc = self.document.read(cx);
        let text_owned = doc.text();
        let cursor_pos = doc.cursor.min(text_owned.len());
        let mut offset = 0usize;
        let mut lines: Vec<String> = Vec::new();
        for line in text_owned.split('\n') {
            let len = line.chars().count();
            let mut rendered = line.to_string();
            if cursor_pos >= offset && cursor_pos <= offset + len {
                let rel = cursor_pos - offset;
                rendered.insert(rel, '|');
            }
            lines.push(rendered);
            offset += len + 1;
        }
        let _ = doc;

        div()
            .flex_grow()
            .min_w(px(360.))
            .bg(Theme::panel())
            .border_1()
            .border_color(Theme::border())
            .p(px(18.))
            .text_sm()
            .text_color(Theme::text())
            .font_family("Menlo")
            .track_focus(&focus_handle)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                move |_: &MouseDownEvent, window: &mut Window, _app: &mut App| {
                    focus_handle.focus(window);
                }
            })
            .on_key_down({
                let focus = focus_handle.clone();
                let doc_handle = self.document.clone();
                move |event: &KeyDownEvent, window: &mut Window, cx_app: &mut App| {
                    if !focus.is_focused(window) {
                        return;
                    }
                    let key = event.keystroke.key.to_lowercase();
                    let is_cmd = event.keystroke.modifiers.platform || event.keystroke.modifiers.control;
                    if is_cmd {
                        return;
                    }
                    let _ = doc_handle.update(cx_app, |doc, cx_doc| {
                        let len = doc.rope.len_chars();
                        match key.as_str() {
                            "backspace" => {
                                if doc.cursor > 0 && len > 0 {
                                    let start = doc.cursor.saturating_sub(1);
                                    doc.delete_range(start..doc.cursor);
                                    doc.cursor = start;
                                    cx_doc.notify();
                                }
                            }
                            "delete" => {
                                if doc.cursor < len {
                                    let end = (doc.cursor + 1).min(len);
                                    doc.delete_range(doc.cursor..end);
                                    cx_doc.notify();
                                }
                            }
                            "enter" | "return" => {
                                doc.insert(doc.cursor, "\n");
                                doc.cursor += 1;
                                cx_doc.notify();
                            }
                            "left" | "arrowleft" => {
                                if doc.cursor > 0 {
                                    doc.cursor -= 1;
                                }
                            }
                            "right" | "arrowright" => {
                                if doc.cursor < len {
                                    doc.cursor += 1;
                                }
                            }
                            _ => {
                                if let Some(ch) = event
                                    .keystroke
                                    .key_char
                                    .as_ref()
                                    .and_then(|s| s.chars().next())
                                {
                                    let insert = ch.to_string();
                                    doc.insert(doc.cursor, &insert);
                                    doc.cursor =
                                        (doc.cursor).saturating_add(insert.chars().count());
                                    cx_doc.notify();
                                } else if let Some(raw) = &event.keystroke.key_char {
                                    if raw == "\n" {
                                        doc.insert(doc.cursor, "\n");
                                        doc.cursor += 1;
                                        cx_doc.notify();
                                    }
                                }
                            }
                        }
                    });
                }
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(lines.into_iter().map(|l| div().child(SharedString::from(l)))),
            )
    }
}
