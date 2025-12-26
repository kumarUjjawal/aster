use crate::model::preview::PreviewState;
use crate::services::markdown::{Block, InlineRun};
use crate::ui::theme::Theme;
use gpui::{
    prelude::FluentBuilder, App, ClickEvent, Context, CursorStyle, Entity, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ObjectFit,
    ParentElement, Render, ScrollHandle, SharedString, SharedUri, StatefulInteractiveElement,
    Styled, StyledImage, Window, div, img, point, px,
};

pub struct PreviewView {
    preview: Entity<PreviewState>,
    focus_handle: Option<FocusHandle>,
    scroll_handle: ScrollHandle,
}

impl PreviewView {
    pub fn new(preview: Entity<PreviewState>) -> Self {
        Self {
            preview,
            focus_handle: None,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks = self.preview.read(cx).blocks.clone(); // Arc clone - cheap!
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();
        let scroll_handle = self.scroll_handle.clone();

        div()
            .id("preview_scroll")
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(Theme::panel_alt())
            .p(px(18.))
            .text_sm()
            .text_color(Theme::text())
            .overflow_y_scroll()
            .overflow_x_hidden()
            .scrollbar_width(px(10.))
            .track_scroll(&self.scroll_handle)
            .track_focus(&focus_handle)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                move |_: &MouseDownEvent, window: &mut Window, _cx: &mut App| {
                    focus_handle.focus(window);
                }
            })
            .on_key_down({
                let focus_handle = focus_handle.clone();
                let scroll_handle = scroll_handle.clone();
                move |event: &KeyDownEvent, window: &mut Window, _cx: &mut App| {
                    if !focus_handle.is_focused(window) {
                        return;
                    }

                    let key = event.keystroke.key.to_lowercase();
                    let modifiers = event.keystroke.modifiers;
                    let is_cmd = modifiers.platform || modifiers.control;

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
                    }
                }
            })
            .child(
                div()
                    .w_full()
                    .min_w(px(0.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(blocks.iter().cloned().map(render_block)),
            )
    }
}

fn render_block(block: Block) -> gpui::AnyElement {
    match block {
        Block::Heading(level, runs) => {
            let mut el = div().text_color(Theme::text());
            el = match level {
                1 => el
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
                2 => el
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
                _ => el
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Theme::accent()),
            };
            el.child(render_inline_runs(runs)).into_any_element()
        }
        Block::Paragraph(runs) => div().child(render_inline_runs(runs)).into_any_element(),
        Block::ListItem(runs) => div()
            .flex()
            .items_start()
            .gap_2()
            .child(div().text_color(Theme::accent()).text_lg().child("•"))
            .child(div().flex_1().min_w(px(0.)).child(render_inline_runs(runs)))
            .into_any_element(),
        Block::CodeBlock(text) => div()
            .font_family("Menlo")
            .bg(Theme::border())
            .p(px(10.))
            .rounded(px(4.))
            .child(SharedString::from(text))
            .into_any_element(),
        Block::Quote(runs) => div()
            .flex()
            .gap_2()
            .child(div().w(px(4.)).bg(Theme::strong()))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_color(Theme::muted())
                    .italic()
                    .child(render_inline_runs(runs)),
            )
            .into_any_element(),
        Block::Image { alt, src } => render_image_block(alt, src),
    }
}

/// Renders an image block with the given alt text and source.
/// Supports HTTP/HTTPS URLs and local file paths.
/// Images are scaled to fit the preview pane width while maintaining aspect ratio.
fn render_image_block(alt: String, src: String) -> gpui::AnyElement {
    use std::path::PathBuf;

    // Create the image element with appropriate source
    // GPUI's img() accepts various source types via From implementations:
    // - SharedUri for HTTP/HTTPS URLs
    // - PathBuf for local file paths
    let img_element = if src.starts_with("http://") || src.starts_with("https://") {
        // Remote image - use SharedUri
        let uri: SharedUri = src.clone().into();
        img(uri).w_full().object_fit(ObjectFit::ScaleDown)
    } else {
        // Local file path
        let path = PathBuf::from(&src);
        img(path).w_full().object_fit(ObjectFit::ScaleDown)
    };

    // Create container with image and optional alt text caption
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .child(img_element)
        .when(!alt.is_empty(), |el| {
            el.child(
                div()
                    .text_xs()
                    .text_color(Theme::muted())
                    .italic()
                    .child(SharedString::from(alt))
            )
        })
        .into_any_element()
}
fn render_inline_runs(runs: Vec<InlineRun>) -> impl IntoElement {
    let lines = split_runs(runs);
    div()
        .w_full()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .children(lines.into_iter().map(|line| {
            div()
                .w_full()
                .min_w(px(0.))
                .flex()
                .flex_row()
                .flex_wrap()
                .items_baseline()
                .children(line.into_iter().map(render_inline_run))
        }))
}

fn render_inline_run(r: InlineRun) -> impl IntoElement {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static LINK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

    let text = r.text.clone();

    // Base styling that applies to all runs
    let apply_base_styles = |mut el: gpui::Div| -> gpui::Div {
        if r.bold {
            el = el.font_weight(FontWeight::BOLD);
        }
        if r.italic {
            el = el.italic();
        }
        if r.code {
            el = el
                .font_family("Menlo")
                .bg(Theme::border())
                .rounded(px(4.))
                .px(px(4.))
                .py(px(2.));
        }
        el
    };

    // For links, we need to add interactivity which changes the type to Stateful<Div>
    if let Some(ref url) = r.link {
        let url_for_click = url.clone();
        let link_id = LINK_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = apply_base_styles(div().child(SharedString::from(text)));
        return base
            .id(SharedString::from(format!("link_{}", link_id)))
            .text_color(Theme::accent())
            .underline()
            .cursor(CursorStyle::PointingHand)
            .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                open_link(&url_for_click, cx);
            })
            .into_any_element();
    }

    // Non-link runs
    apply_base_styles(div().child(SharedString::from(text))).into_any_element()
}

/// Opens a URL in the system's default browser.
/// Only http://, https://, and mailto: schemes are supported.
/// Unsupported or malformed URLs are silently ignored.
fn open_link(url: &str, cx: &mut App) {
    let url_trimmed = url.trim();
    if url_trimmed.is_empty() {
        return;
    }

    // Only allow safe URL schemes
    if url_trimmed.starts_with("http://")
        || url_trimmed.starts_with("https://")
        || url_trimmed.starts_with("mailto:")
    {
        cx.open_url(url_trimmed);
    }
    // Silently ignore unsupported schemes (file://, javascript:, etc.)
}

fn split_runs(runs: Vec<InlineRun>) -> Vec<Vec<InlineRun>> {
    let mut lines: Vec<Vec<InlineRun>> = vec![Vec::new()];
    for run in runs {
        let parts: Vec<&str> = run.text.split('\n').collect();
        for (idx, part) in parts.iter().enumerate() {
            if idx > 0 {
                lines.push(Vec::new());
            }
            if !part.is_empty() {
                lines.last_mut().unwrap().push(InlineRun {
                    text: part.to_string(),
                    bold: run.bold,
                    italic: run.italic,
                    code: run.code,
                    link: run.link.clone(),
                });
            }
        }
    }
    lines
}
