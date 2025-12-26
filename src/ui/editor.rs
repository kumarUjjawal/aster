use crate::commands::{Copy, Cut, Paste, SelectAll};
use crate::model::document::DocumentState;
use crate::ui::theme::Theme;
use gpui::{
    App, Bounds, ClipboardItem, Context, Entity, FocusHandle, Focusable, HighlightStyle,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    ParentElement, Render, ScrollHandle, StatefulInteractiveElement, Styled, StyledText, Window,
    canvas, div, fill, point, px, size,
};
use std::ops::Range;
use std::panic::AssertUnwindSafe;
use std::time::Duration;

pub struct EditorView {
    document: Entity<DocumentState>,
    focus_handle: Option<FocusHandle>,
    caret_visible: bool,
    blink_task: Option<gpui::Task<()>>,
    scroll_handle: ScrollHandle,
    /// Cached text with revision to avoid repeated rope-to-string conversions
    cached_text: Option<(u64, String)>,
}

impl EditorView {
    pub fn new(document: Entity<DocumentState>) -> Self {
        Self {
            document,
            focus_handle: None,
            caret_visible: true,
            blink_task: None,
            scroll_handle: ScrollHandle::new(),
            cached_text: None,
        }
    }

    fn start_cursor_blink(&mut self, cx: &mut Context<Self>) {
        if self.blink_task.is_some() {
            return;
        }
        let entity = cx.entity();
        self.blink_task = Some(cx.spawn(async move |_editor, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let _ = entity.update(cx, |view, cx| {
                    view.caret_visible = !view.caret_visible;
                    cx.notify();
                });
            }
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
}

impl Focusable for EditorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle
            .clone()
            .expect("focus handle should be initialized during render")
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.start_cursor_blink(cx);
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| {
                let handle = cx.focus_handle();
                handle.focus(window);
                handle
            })
            .clone();
        let is_focused = focus_handle.is_focused(window);
        // Use cached text if revision hasn't changed to avoid O(n) rope conversion
        let (text_owned, doc_revision) = {
            let doc = self.document.read(cx);
            let rev = doc.revision;
            if let Some((cached_rev, ref text)) = self.cached_text {
                if cached_rev == rev {
                    (text.clone(), rev)
                } else {
                    (doc.text(), rev)
                }
            } else {
                (doc.text(), rev)
            }
        };
        // Update cache if needed (after releasing the read borrow)
        if self.cached_text.as_ref().map(|(r, _)| *r) != Some(doc_revision) {
            self.cached_text = Some((doc_revision, text_owned.clone()));
        }
        let doc = self.document.read(cx);
        let cursor_byte = doc.char_to_byte(doc.cursor);
        let show_caret = doc.selection.is_none();
        let draw_caret = show_caret && is_focused && self.caret_visible;

        let highlights = self.selection_highlights(&doc);
        let mut styled = StyledText::new(text_owned);
        if !highlights.is_empty() {
            styled = styled.with_highlights(highlights);
        }
        let text_layout = styled.layout().clone();
        let scroll_handle = self.scroll_handle.clone();

        div()
            .id("editor_scroll")
            .relative()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(Theme::panel())
            .p(px(18.))
            .text_sm()
            .text_color(Theme::text())
            .font_family("Menlo")
            .overflow_y_scroll()
            .overflow_x_hidden()
            .scrollbar_width(px(10.))
            .track_scroll(&self.scroll_handle)
            .track_focus(&focus_handle)
            .on_action({
                let doc_handle = self.document.clone();
                move |_: &SelectAll, _window: &mut Window, cx_app: &mut App| {
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        doc.select_all();
                        cx.notify();
                    });
                }
            })
            .on_action({
                let doc_handle = self.document.clone();
                move |_: &Copy, _window: &mut Window, cx_app: &mut App| {
                    if let Some(selection) =
                        doc_handle.read_with(cx_app, |d, _| d.selection_range())
                    {
                        let text = doc_handle.read_with(cx_app, |d, _| d.slice_chars(selection));
                        cx_app.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
            })
            .on_action({
                let doc_handle = self.document.clone();
                move |_: &Cut, _window: &mut Window, cx_app: &mut App| {
                    let selection = doc_handle
                        .read_with(cx_app, |d, _| d.selection_range())
                        .unwrap_or_else(|| 0..0);
                    if selection.start == selection.end {
                        return;
                    }

                    let text =
                        doc_handle.read_with(cx_app, |d, _| d.slice_chars(selection.clone()));
                    cx_app.write_to_clipboard(ClipboardItem::new_string(text));
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        doc.delete_selection();
                        cx.notify();
                    });
                }
            })
            .on_action({
                let doc_handle = self.document.clone();
                move |_: &Paste, _window: &mut Window, cx_app: &mut App| {
                    let Some(item) = cx_app.read_from_clipboard() else {
                        return;
                    };
                    let Some(text) = item.text() else {
                        return;
                    };
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        doc.delete_selection();
                        let insert_at = doc.cursor;
                        doc.insert(insert_at, &text);
                        doc.cursor = insert_at.saturating_add(text.chars().count());
                        cx.notify();
                    });
                }
            })
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                let doc_handle = self.document.clone();
                let layout_for_event = text_layout.clone();
                move |event: &MouseDownEvent, window: &mut Window, cx_app: &mut App| {
                    focus_handle.focus(window);
                    let _ = doc_handle.update(cx_app, |doc, cx| {
                        let byte_idx = std::panic::catch_unwind(AssertUnwindSafe(|| {
                            layout_for_event.index_for_position(event.position)
                        }))
                        .ok()
                        .map(|res| match res {
                            Ok(ix) => ix,
                            Err(ix) => ix,
                        });
                        if let Some(byte_idx) = byte_idx.map(|b| doc.byte_to_char(b)) {
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
                move |event: &MouseMoveEvent, _window: &mut Window, cx_app: &mut App| {
                    if !event.dragging() {
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
                        if let Some(byte_idx) = byte_idx.map(|b| doc.byte_to_char(b)) {
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
                let scroll_handle = scroll_handle.clone();
                move |event: &KeyDownEvent, window: &mut Window, cx_app: &mut App| {
                    if !focus.is_focused(window) {
                        return;
                    }
                    let key = event.keystroke.key.to_lowercase();
                    let modifiers = event.keystroke.modifiers;
                    let is_cmd = modifiers.platform || modifiers.control;
                    let shift = modifiers.shift;

                    if is_cmd {
                        return;
                    }

                    if key == "pageup" || key == "pagedown" {
                        let max = scroll_handle.max_offset();
                        let offset = scroll_handle.offset();
                        let bounds = scroll_handle.bounds();
                        let page = bounds.size.height;
                        if page > px(0.) {
                            let amount = page * 0.9;
                            let delta = if key == "pagedown" { -amount } else { amount };
                            let mut new_offset = offset;
                            new_offset.y = (new_offset.y + delta).clamp(-max.height, px(0.));
                            scroll_handle.set_offset(point(new_offset.x, new_offset.y));
                            window.refresh();
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
                                        cx_doc.notify();
                                    }
                                } else if doc.cursor > 0 {
                                    doc.cursor -= 1;
                                    doc.clear_selection();
                                    cx_doc.notify();
                                }
                            }
                            "right" | "arrowright" => {
                                if shift {
                                    let anchor = doc.selection_anchor.unwrap_or(doc.cursor);
                                    if doc.cursor < len {
                                        doc.set_selection(anchor, doc.cursor + 1);
                                        cx_doc.notify();
                                    }
                                } else if doc.cursor < len {
                                    doc.cursor += 1;
                                    doc.clear_selection();
                                    cx_doc.notify();
                                }
                            }
                            "up" | "arrowup" => {
                                let cursor = doc.cursor.min(len);
                                let line_idx = doc.rope.char_to_line(cursor);
                                if line_idx == 0 {
                                    return;
                                }
                                let line_start = doc.rope.line_to_char(line_idx);
                                let col = cursor.saturating_sub(line_start);
                                let target_line = line_idx - 1;
                                let target_start = doc.rope.line_to_char(target_line);
                                let target_len = doc.rope.line(target_line).len_chars();
                                let max_col = if target_line + 1 < doc.rope.len_lines() {
                                    target_len.saturating_sub(1)
                                } else {
                                    target_len
                                };
                                let new_cursor = target_start + col.min(max_col);

                                if shift {
                                    let anchor = doc.selection_anchor.unwrap_or(cursor);
                                    doc.set_selection(anchor, new_cursor);
                                } else {
                                    doc.cursor = new_cursor;
                                    doc.clear_selection();
                                }
                                cx_doc.notify();
                            }
                            "down" | "arrowdown" => {
                                let cursor = doc.cursor.min(len);
                                let line_idx = doc.rope.char_to_line(cursor);
                                if line_idx + 1 >= doc.rope.len_lines() {
                                    return;
                                }
                                let line_start = doc.rope.line_to_char(line_idx);
                                let col = cursor.saturating_sub(line_start);
                                let target_line = line_idx + 1;
                                let target_start = doc.rope.line_to_char(target_line);
                                let target_len = doc.rope.line(target_line).len_chars();
                                let max_col = if target_line + 1 < doc.rope.len_lines() {
                                    target_len.saturating_sub(1)
                                } else {
                                    target_len
                                };
                                let new_cursor = target_start + col.min(max_col);

                                if shift {
                                    let anchor = doc.selection_anchor.unwrap_or(cursor);
                                    doc.set_selection(anchor, new_cursor);
                                } else {
                                    doc.cursor = new_cursor;
                                    doc.clear_selection();
                                }
                                cx_doc.notify();
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
                div().relative().child(styled).child(
                    canvas(
                        move |_, _, _| {},
                        move |_bounds: Bounds<_>, (), window: &mut Window, _cx: &mut App| {
                            if !draw_caret {
                                return;
                            }

                            let caret_pos = std::panic::catch_unwind(AssertUnwindSafe(|| {
                                text_layout.position_for_index(cursor_byte)
                            }))
                            .ok()
                            .flatten();
                            let Some(caret_pos) = caret_pos else {
                                return;
                            };

                            let line_height = std::panic::catch_unwind(AssertUnwindSafe(|| {
                                text_layout.line_height()
                            }))
                            .ok()
                            .unwrap_or(px(0.));
                            if line_height <= px(0.) {
                                return;
                            }

                            window.paint_quad(fill(
                                Bounds {
                                    origin: point(caret_pos.x, caret_pos.y),
                                    size: size(px(1.), line_height),
                                },
                                Theme::accent(),
                            ));
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                ),
            )
    }
}

fn hsla_from_rgba(color: gpui::Rgba) -> gpui::Hsla {
    let mut hsla: gpui::Hsla = color.into();
    hsla.a = 0.18;
    hsla
}
