use crate::model::preview::PreviewState;
use crate::services::markdown::{Block, InlineRun, TableCell, TableRow};
use crate::services::settings;
use crate::ui::theme::Theme;
use gpui::{
    prelude::FluentBuilder, list, App, ClickEvent, Context, CursorStyle, Entity, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ListAlignment, ListState, MouseButton, MouseDownEvent, ObjectFit,
    ParentElement, Render, ScrollHandle, SharedString, SharedUri, StatefulInteractiveElement,
    Styled, StyledImage, Window, div, img, px,
};
use std::sync::Arc;

pub struct PreviewView {
    preview: Entity<PreviewState>,
    focus_handle: Option<FocusHandle>,
    scroll_handle: ScrollHandle,
    /// Virtualized list state for efficient rendering of large documents
    list_state: ListState,
    /// Cached grouped blocks to avoid O(n) clone and regrouping every frame
    cached_groups: Option<Arc<Vec<BlockGroup>>>,
    /// Pointer to the blocks Arc for cache invalidation
    cached_blocks_ptr: usize,
}

impl PreviewView {
    pub fn new(preview: Entity<PreviewState>) -> Self {
        Self {
            preview,
            focus_handle: None,
            scroll_handle: ScrollHandle::new(),
            // Virtualized list: 0 items initially, top alignment, 300px overdraw for smooth scrolling
            list_state: ListState::new(0, ListAlignment::Top, px(300.0)),
            cached_groups: None,
            cached_blocks_ptr: 0,
        }
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let blocks = self.preview.read(cx).blocks.clone(); // Arc clone - cheap!
        let footnotes = self.preview.read(cx).footnotes.clone(); // Arc clone - cheap!
        let focus_handle = self
            .focus_handle
            .get_or_insert_with(|| cx.focus_handle())
            .clone();

        // Clone scroll_handle for use in footnote closures
        let scroll_handle_for_footnotes = Some(self.scroll_handle.clone());

        // Build the footnotes section if there are any
        let has_footnotes = !footnotes.is_empty();

        // Cache grouped blocks - only recompute when blocks Arc changes
        let blocks_ptr = Arc::as_ptr(&blocks) as usize;
        let grouped = if self.cached_blocks_ptr == blocks_ptr && self.cached_groups.is_some() {
            self.cached_groups.clone().unwrap()
        } else {
            let groups = Arc::new(group_blocks(blocks.as_ref().clone()));
            self.cached_groups = Some(groups.clone());
            self.cached_blocks_ptr = blocks_ptr;
            // Reset list state when blocks change
            self.list_state.reset(groups.len());
            groups
        };

        div()
            .id("preview_container")
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .bg(Theme::panel_alt())
            .p(px(18.))
            .text_size(px(settings::get_font_size()))
            .text_color(Theme::text())
            .flex()
            .flex_col()
            .track_focus(&focus_handle)
            .on_mouse_down(MouseButton::Left, {
                let focus_handle = focus_handle.clone();
                move |_: &MouseDownEvent, window: &mut Window, _cx: &mut App| {
                    focus_handle.focus(window);
                }
            })
            // Virtualized list - takes up all available space and handles its own scrolling
            .child({
                let grouped_for_list = grouped.clone();
                list(self.list_state.clone(), move |ix, _window, _cx| {
                    if let Some(group) = grouped_for_list.get(ix) {
                        div()
                            .w_full()
                            .pb_3() // gap between blocks
                            .child(render_block_group(group.clone(), None))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }
                })
                .flex_1()
                .size_full()
            })
            // Add footnotes section if there are footnotes (outside virtualized list)
            .when(has_footnotes, |el| {
                el.child(
                    // Horizontal rule separator
                    div()
                        .w_full()
                        .h(px(1.))
                        .bg(Theme::border())
                        .my_3()
                )
                .child(
                    // Footnotes container
                    div()
                        .id("footnotes_section")
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children({
                            let handle = scroll_handle_for_footnotes.clone();
                            footnotes.iter().cloned().map(move |block| {
                                render_block(block, handle.clone())
                            })
                        })
                )
            })
    }
}

fn render_block(block: Block, scroll_handle: Option<ScrollHandle>) -> gpui::AnyElement {
    match block {
        Block::Heading(level, runs) => {
            let mut el = div().w_full().min_w(px(0.)).text_color(Theme::text());
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
        Block::Paragraph(runs) => div().w_full().min_w(px(0.)).child(render_inline_runs(runs)).into_any_element(),
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
        Block::TaskListItem { checked, content } => {
            let checkbox = if checked {
                div()
                    .text_lg()
                    .text_color(Theme::accent())
                    .child("☑")
            } else {
                div()
                    .text_lg()
                    .text_color(Theme::muted())
                    .child("☐")
            };
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(checkbox)
                .child(div().flex_1().min_w(px(0.)).child(render_inline_runs(content)))
                .into_any_element()
        }
        Block::OrderedListItem { number, content } => div()
            .flex()
            .items_start()
            .gap_2()
            .child(
                div()
                    .text_color(Theme::accent())
                    .child(SharedString::from(format!("{}.", number))),
            )
            .child(div().flex_1().min_w(px(0.)).child(render_inline_runs(content)))
            .into_any_element(),
        Block::FootnoteRef { label, index } => {
            // Render as superscript number that links to definition
            let scroll_handle_clone = scroll_handle.clone();
            let label_clone = label.clone();
            div()
                .id(SharedString::from(format!("footnote_ref_{}", label)))
                .text_xs()
                .text_color(Theme::accent())
                .cursor_pointer()
                .child(SharedString::from(format!("[{}]", index)))
                .when_some(scroll_handle_clone, move |el, _handle| {
                    el.on_click(move |_: &ClickEvent, _window: &mut Window, _cx: &mut App| {
                        // TODO: Scroll to footnote definition when GPUI supports scroll_to_item by ID
                        // For now, clicking will be a no-op until we can implement proper scrolling
                        let _ = &label_clone; // Keeps the label for future scroll-to implementation
                    })
                })
                .into_any_element()
        }
        Block::FootnoteDefinition { label, index, content } => {
            // Render footnote definition with number and backlink
            let scroll_handle_clone = scroll_handle.clone();
            let label_clone = label.clone();
            div()
                .id(SharedString::from(format!("footnote_def_{}", label)))
                .flex()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(Theme::accent())
                        .font_weight(FontWeight::BOLD)
                        .child(SharedString::from(format!("{}.", index)))
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .text_sm()
                        .child(render_inline_runs(content))
                )
                .child(
                    div()
                        .id(SharedString::from(format!("footnote_back_{}", label)))
                        .text_xs()
                        .text_color(Theme::accent())
                        .cursor_pointer()
                        .child("↩")
                        .when_some(scroll_handle_clone, move |el, _handle| {
                            el.on_click(move |_: &ClickEvent, _window: &mut Window, _cx: &mut App| {
                                // TODO: Scroll back to reference when GPUI supports scroll_to_item by ID
                                let _ = &label_clone;
                            })
                        })
                )
                .into_any_element()
        }
        Block::Table { alignments, rows } => render_table(alignments, rows),
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

/// Renders a GFM table with borders, header styling, and column alignment.
fn render_table(alignments: Vec<pulldown_cmark::Alignment>, rows: Vec<TableRow>) -> gpui::AnyElement {
    use pulldown_cmark::Alignment;
    
    div()
        .w_full()
        .overflow_x_hidden()
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(Theme::border())
                .rounded(px(4.))
                .overflow_hidden()
                .children(rows.into_iter().enumerate().map(|(row_idx, row)| {
                    let is_header = row.cells.first().map(|c| c.is_header).unwrap_or(false);
                    let alignments = alignments.clone();
                    
                    div()
                        .flex()
                        .flex_row()
                        .when(is_header, |el| {
                            el.bg(Theme::border())
                                .font_weight(FontWeight::BOLD)
                        })
                        .when(row_idx > 0, |el| {
                            el.border_t_1()
                                .border_color(Theme::border())
                        })
                        .children(row.cells.into_iter().enumerate().map({
                            let alignments = alignments.clone();
                            move |(col_idx, cell)| {
                                let alignment = alignments.get(col_idx).copied().unwrap_or(Alignment::None);
                                render_table_cell(cell, alignment, col_idx > 0)
                            }
                        }))
                }))
        )
        .into_any_element()
}

/// Renders a single table cell with alignment and border.
fn render_table_cell(cell: TableCell, alignment: pulldown_cmark::Alignment, has_left_border: bool) -> gpui::AnyElement {
    use pulldown_cmark::Alignment;
    
    let mut el = div()
        .flex_1()
        .min_w(px(60.))
        .px(px(8.))
        .py(px(6.));
    
    // Add left border for non-first columns
    if has_left_border {
        el = el.border_l_1().border_color(Theme::border());
    }
    
    // Apply text alignment
    el = match alignment {
        Alignment::Left | Alignment::None => el,
        Alignment::Center => el.flex().justify_center(),
        Alignment::Right => el.flex().justify_end(),
    };
    
    el.child(render_inline_runs(cell.content))
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
        let base = apply_base_styles(div().w_full().min_w(px(0.)).child(SharedString::from(text)));
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
    apply_base_styles(div().w_full().min_w(px(0.)).child(SharedString::from(text))).into_any_element()
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

/// Groups consecutive list items together for compact rendering
#[derive(Clone)]
enum BlockGroup {
    Single(Block),
    ListGroup(Vec<Block>),
}

/// Identifies the list type for grouping purposes
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListType {
    Unordered,
    Ordered,
    Task,
}

fn get_list_type(block: &Block) -> Option<ListType> {
    match block {
        Block::ListItem(_) => Some(ListType::Unordered),
        Block::OrderedListItem { .. } => Some(ListType::Ordered),
        Block::TaskListItem { .. } => Some(ListType::Task),
        _ => None,
    }
}

fn group_blocks(blocks: Vec<Block>) -> Vec<BlockGroup> {
    let mut groups: Vec<BlockGroup> = Vec::new();
    let mut current_list: Vec<Block> = Vec::new();
    let mut current_list_type: Option<ListType> = None;

    for block in blocks {
        let block_list_type = get_list_type(&block);
        
        if let Some(list_type) = block_list_type {
            // Check if this is the same type as the current list
            if current_list_type == Some(list_type) {
                current_list.push(block);
            } else {
                // Different list type - flush the current list first
                if !current_list.is_empty() {
                    groups.push(BlockGroup::ListGroup(std::mem::take(&mut current_list)));
                }
                current_list.push(block);
                current_list_type = Some(list_type);
            }
        } else {
            // Not a list block - flush any pending list items
            if !current_list.is_empty() {
                groups.push(BlockGroup::ListGroup(std::mem::take(&mut current_list)));
                current_list_type = None;
            }
            groups.push(BlockGroup::Single(block));
        }
    }

    // Don't forget the last group
    if !current_list.is_empty() {
        groups.push(BlockGroup::ListGroup(current_list));
    }

    groups
}

fn render_block_group(group: BlockGroup, scroll_handle: Option<ScrollHandle>) -> gpui::AnyElement {
    match group {
        BlockGroup::Single(block) => render_block(block, scroll_handle),
        BlockGroup::ListGroup(blocks) => {
            let handle = scroll_handle.clone();
            div()
                .flex()
                .flex_col()
                .gap_0()
                .children(blocks.into_iter().map(move |b| render_block(b, handle.clone())))
                .into_any_element()
        }
    }
}

