use crate::model::file_tree::FileTreeState;
use crate::ui::theme::Theme;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    Context, Entity, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, div, px, svg,
};
use gpui_component::{IconName, IconNamed};

pub struct FileExplorerView {
    file_tree: Entity<FileTreeState>,
    scroll_handle: ScrollHandle,
}

impl FileExplorerView {
    pub fn new(file_tree: Entity<FileTreeState>) -> Self {
        Self {
            file_tree,
            scroll_handle: ScrollHandle::new(),
        }
    }
}

impl Render for FileExplorerView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Clone the data we need to avoid borrow issues
        let (visible_entries, selected_path) = {
            let tree = self.file_tree.read(cx);
            (
                tree.visible_entries()
                    .into_iter()
                    .map(|(idx, entry)| {
                        (
                            idx,
                            entry.path.clone(),
                            entry.name.clone(),
                            entry.is_dir,
                            entry.depth,
                            entry.expanded,
                        )
                    })
                    .collect::<Vec<_>>(),
                tree.selected_path.clone(),
            )
        };

        let has_entries = !visible_entries.is_empty();
        let file_tree = self.file_tree.clone();

        // Build entry elements inline
        let entry_elements: Vec<_> = visible_entries
            .into_iter()
            .map(|(index, path, name, is_dir, depth, expanded)| {
                let is_selected = selected_path
                    .as_ref()
                    .map(|p| p == &path)
                    .unwrap_or(false);
                let file_tree_clone = file_tree.clone();

                // For folders, we show: chevron + folder icon + name
                // For files, we show: file icon + name
                let folder_color = gpui::rgb(0x7eb4ea); // Blue folder color matching the reference image

                div()
                    .id(("file-entry", index))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .pl(px(8. + (depth as f32) * 16.))
                    .pr(px(8.))
                    .py(px(4.))
                    .cursor_pointer()
                    .when(is_selected, |this| this.bg(Theme::selection_bg()))
                    .hover(|this| this.bg(Theme::panel_alt()))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |_this, _: &MouseDownEvent, _, cx| {
                            if is_dir {
                                let _ = file_tree_clone.update(cx, |tree, cx| {
                                    tree.toggle_expanded(index, cx);
                                });
                            } else {
                                let _ = file_tree_clone.update(cx, |tree, cx| {
                                    tree.select(index, cx);
                                });
                            }
                        }),
                    )
                    .when(is_dir, |this| {
                        // Folder: chevron + folder icon + name
                        let chevron_icon = if expanded {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        };
                        this.child(
                            svg()
                                .path(chevron_icon.path())
                                .size(px(12.))
                                .text_color(Theme::muted())
                                .flex_shrink_0(),
                        )
                        .child(
                            svg()
                                .path(IconName::Folder.path())
                                .size(px(14.))
                                .text_color(folder_color)
                                .flex_shrink_0(),
                        )
                    })
                    .when(!is_dir, |this| {
                        // File: file icon + name
                        this.child(
                            svg()
                                .path(IconName::File.path())
                                .size(px(14.))
                                .text_color(Theme::muted())
                                .flex_shrink_0(),
                        )
                    })
                    .child(
                        div()
                            .text_sm()
                            .truncate()
                            .flex_1()
                            .text_color(Theme::text())
                            .child(name),
                    )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(px(200.))
            .bg(Theme::sidebar())
            .border_r_1()
            .border_color(Theme::border())
            .flex_shrink_0()
            .child(
                // Header
                div()
                    .flex()
                    .items_center()
                    .px(px(12.))
                    .py(px(10.))
                    .border_b_1()
                    .border_color(Theme::border())
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(Theme::muted())
                            .child("FILES"),
                    ),
            )
            .child(
                // File list
                div()
                    .id("file-explorer-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .when(!has_entries, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .p(px(16.))
                                .text_sm()
                                .text_color(Theme::muted())
                                .child("No markdown files"),
                        )
                    })
                    .when(has_entries, |this| this.children(entry_elements)),
            )
    }
}
