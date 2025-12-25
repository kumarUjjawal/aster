use crate::model::document::DocumentState;
use crate::ui::theme::Theme;
use gpui::{
    div, px, App, ClipboardItem, Context, Entity, FocusHandle, Focusable, HighlightStyle,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement, Pixels, Point, Render, Styled, StyledText, Window,
};
use std::ops::Range;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

pub struct EditorView {
    document: Entity<DocumentState>,
    focus_handle: Option<FocusHandle>,
    caret_visible: bool,
    blink_task: Option<gpui::Task<()>>,
    text_layout: Option<gpui::TextLayout>,
    layout_ready: bool,
    last_revision: u64,
}

impl EditorView {
    pub fn new(document: Entity<DocumentState>) -> Self {
        Self {
            document,
            focus_handle: None,
            caret_visible: true,
            blink_task: None,
            text_layout: None,
            layout_ready: false,
            last_revision: 0,
        }
    }

    fn start_cursor_blink(&mut self, cx: &mut Context<Self>) {
        if self.blink_task.is_some() {
            return;
        }
        let entity = cx.entity();
        self.blink_task = Some(cx.spawn(async move |_editor, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let _ = entity.update(cx, |view, cx| {
                view.caret_visible = !view.caret_visible;
                cx.notify();
            });
        }));
    }

    fn selection_highlights(&self, doc: &DocumentState) -> Vec<(Range<usize>, HighlightStyle)> {
        doc.selection_bytes().map_or_else(Vec::new, |range| {
            vec![(
                range,
                HighlightStyle {
                    background_color: Some(hsla_from_rgba(Theme::selection_bg())),
                    ..Default::default()
                },
            )]
        })
    }

    fn caret_position(&self, byte_idx: usize) -> Option<Point<Pixels>> {
        let layout = self.text_layout.as_ref()?;
        std::panic::catch_unwind(AssertUnwindSafe(|| layout.position_for_index(byte_idx)))
            .ok()
            .flatten()
    }

    fn line_height(&self) -> Option<Pixels> {
        let layout = self.text_layout.as_ref()?;
        std::panic::catch_unwind(AssertUnwindSafe(|| layout.line_height())).ok()
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
        self.start_cursor_blink(cx);
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let doc = self.document.read(cx);
        let text_owned = doc.text();
        let cursor_byte = doc.char_to_byte(doc.cursor);
        let highlights = self.selection_highlights(&doc);
        let mut styled = StyledText::new(text_owned.clone());
        if !highlights.is_empty() {
            styled = styled.with_highlights(highlights);
        }
        let text_layout = styled.layout().clone();
        self.text_layout = Some(text_layout.clone());
        let layout_ready = self.layout_ready && self.last_revision == doc.revision;
        self.last_revision = doc.revision;
        if !layout_ready {
            // Mark ready for the next render; avoids touching the layout before gpui measures it.
            self.layout_ready = true;
            cx.notify();
        }

        div()
            .relative()
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
                let doc_handle = self.document.clone();
                let layout_for_event = text_layout.clone();
                let layout_ready = layout_ready;
                move |event: &MouseDownEvent, window: &mut Window, cx_app: &mut App| {
                    focus_handle.focus(window);
                    if !layout_ready {
                        return;
                    }
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        let byte_idx = std::panic::catch_unwind(AssertUnwindSafe(|| {
                            layout_for_event.index_for_position(event.position)
                        }))
                        .ok()
                        .map(|res| match res {
                            Ok(ix) => ix,
                            Err(ix) => ix,
                        });
                        if let Some(byte_idx) = byte_idx.map(|b| doc.byte_to_char(b))
                        {
                            if event.modifiers.shift {
                                let anchor = doc.selection_anchor.unwrap_or(doc.cursor);
                                doc.set_selection(anchor, byte_idx);
                            } else {
                                doc.set_cursor(byte_idx);
                            }
                            cx.notify();
                        }
                    });
                }
            })
            .on_mouse_move({
                let doc_handle = self.document.clone();
                let layout_for_event = text_layout.clone();
                let layout_ready = layout_ready;
                move |event: &MouseMoveEvent, _window: &mut Window, cx_app: &mut App| {
                    if !event.dragging() {
                        return;
                    }
                    if !layout_ready {
                        return;
                    }
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        let byte_idx = std::panic::catch_unwind(AssertUnwindSafe(|| {
                            layout_for_event.index_for_position(event.position)
                        }))
                        .ok()
                        .map(|res| match res {
                            Ok(ix) => ix,
                            Err(ix) => ix,
                        });
                        if let Some(byte_idx) = byte_idx.map(|b| doc.byte_to_char(b))
                        {
                            let anchor = doc.selection_anchor.unwrap_or(doc.cursor);
                            doc.set_selection(anchor, byte_idx);
                            cx.notify();
                        }
                    });
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
                    let modifiers = event.keystroke.modifiers;
                    let is_cmd = modifiers.platform || modifiers.control;
                    let shift = modifiers.shift;

                    if is_cmd {
                        match key.as_str() {
                            "a" => {
                                let _ = doc_handle.update(cx_app, |doc, cx| {
                                    doc.select_all();
                                    cx.notify();
                                });
                            }
                            "c" => {
                                if let Some(selection) =
                                    doc_handle.read_with(cx_app, |d, _| d.selection_range())
                                {
                                    let text = doc_handle
                                        .read_with(cx_app, |d, _| d.slice_chars(selection));
                                    cx_app.write_to_clipboard(ClipboardItem::new_string(text));
                                }
                            }
                            "x" => {
                                let selection = doc_handle
                                    .read_with(cx_app, |d, _| d.selection_range())
                                    .unwrap_or_else(|| 0..0);
                                if selection.start != selection.end {
                                    let text = doc_handle
                                        .read_with(cx_app, |d, _| d.slice_chars(selection.clone()));
                                    cx_app.write_to_clipboard(ClipboardItem::new_string(text));
                                    let _ = doc_handle.update(cx_app, |doc, cx| {
                                        doc.delete_selection();
                                        cx.notify();
                                    });
                                }
                            }
                            "v" => {
                                if let Some(item) = cx_app.read_from_clipboard() {
                                    if let Some(text) = item.text() {
                                        let _ = doc_handle.update(cx_app, |doc, cx| {
                                            doc.delete_selection();
                                            let insert_at = doc.cursor;
                                            doc.insert(insert_at, &text);
                                            doc.cursor =
                                                insert_at.saturating_add(text.chars().count());
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                        return;
                    }
                    let _ = doc_handle.update(cx_app, |doc, cx_doc| {
                        let len = doc.rope.len_chars();
                        match key.as_str() {
                            "backspace" => {
                                if doc.delete_selection().is_some() {
                                    cx_doc.notify();
                                    return;
                                }
                                if doc.cursor > 0 && len > 0 {
                                    let start = doc.cursor.saturating_sub(1);
                                    doc.delete_range(start..doc.cursor);
                                    doc.cursor = start;
                                    cx_doc.notify();
                                }
                            }
                            "delete" => {
                                if doc.delete_selection().is_some() {
                                    cx_doc.notify();
                                    return;
                                }
                                if doc.cursor < len {
                                    let end = (doc.cursor + 1).min(len);
                                    doc.delete_range(doc.cursor..end);
                                    cx_doc.notify();
                                }
                            }
                            "enter" | "return" => {
                                doc.delete_selection();
                                doc.insert(doc.cursor, "\n");
                                doc.cursor += 1;
                                cx_doc.notify();
                            }
                            "left" | "arrowleft" => {
                                if shift {
                                    let anchor = doc.selection_anchor.unwrap_or(doc.cursor);
                                    if doc.cursor > 0 {
                                        doc.set_selection(anchor, doc.cursor - 1);
                                    }
                                } else if doc.cursor > 0 {
                                    doc.cursor -= 1;
                                    doc.clear_selection();
                                }
                            }
                            "right" | "arrowright" => {
                                if shift {
                                    let anchor = doc.selection_anchor.unwrap_or(doc.cursor);
                                    if doc.cursor < len {
                                        doc.set_selection(anchor, doc.cursor + 1);
                                    }
                                } else if doc.cursor < len {
                                    doc.cursor += 1;
                                    doc.clear_selection();
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
                                    doc.delete_selection();
                                    doc.insert(doc.cursor, &insert);
                                    doc.cursor =
                                        (doc.cursor).saturating_add(insert.chars().count());
                                    cx_doc.notify();
                                } else if let Some(raw) = &event.keystroke.key_char {
                                    if raw == "\n" {
                                        doc.delete_selection();
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
                    .whitespace_normal()
                    .child(styled)
                    .child(if layout_ready {
                        std::panic::catch_unwind(AssertUnwindSafe(|| {
                            let maybe_pos = self
                                .caret_visible
                                .then(|| self.caret_position(cursor_byte))
                                .flatten();
                            if let (Some(pos), Some(height)) = (maybe_pos, self.line_height()) {
                        div()
                            .absolute()
                            .left(pos.x)
                            .top(pos.y)
                            .w(px(1.))
                                    .h(height)
                                    .bg(Theme::accent())
                            } else {
                                div().hidden()
                            }
                        }))
                        .unwrap_or_else(|_| div().hidden())
                    } else {
                        div().hidden()
                    }),
            )
    }
}

fn hsla_from_rgba(color: gpui::Rgba) -> gpui::Hsla {
    let mut hsla: gpui::Hsla = color.into();
    hsla.a = 0.18;
    hsla
}
