use camino::Utf8PathBuf;
use gpui::Context;

/// Represents a single entry (file or directory) in the file tree.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: Utf8PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub expanded: bool,
}

impl FileEntry {
    pub fn new(path: Utf8PathBuf, is_dir: bool, depth: usize) -> Self {
        let name = path
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.to_string());
        Self {
            path,
            name,
            is_dir,
            depth,
            expanded: depth == 0, // Root level expanded by default
        }
    }
}

/// State for the file explorer tree.
pub struct FileTreeState {
    pub root_path: Option<Utf8PathBuf>,
    pub entries: Vec<FileEntry>,
    pub selected_path: Option<Utf8PathBuf>,
    /// File path that should be opened next (consumed after use)
    pub pending_open: Option<Utf8PathBuf>,
}

impl FileTreeState {
    pub fn new() -> Self {
        Self {
            root_path: None,
            entries: Vec::new(),
            selected_path: None,
            pending_open: None,
        }
    }

    pub fn set_root(&mut self, path: Utf8PathBuf, cx: &mut Context<Self>) {
        self.root_path = Some(path.clone());
        self.entries = scan_markdown_tree(&path, 0);
        cx.notify();
    }

    pub fn toggle_expanded(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.entries.get_mut(index) {
            if entry.is_dir {
                entry.expanded = !entry.expanded;
                cx.notify();
            }
        }
    }

    pub fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.entries.get(index) {
            self.selected_path = Some(entry.path.clone());
            self.pending_open = Some(entry.path.clone());
            cx.notify();
        }
    }

    /// Take the pending file to open (clears it after taking)
    pub fn take_pending_open(&mut self) -> Option<Utf8PathBuf> {
        self.pending_open.take()
    }

    /// Returns visible entries (respecting expanded/collapsed state).
    pub fn visible_entries(&self) -> Vec<(usize, &FileEntry)> {
        let mut result = Vec::new();
        let mut skip_depth: Option<usize> = None;

        for (idx, entry) in self.entries.iter().enumerate() {
            // If we're skipping entries under a collapsed folder
            if let Some(depth) = skip_depth {
                if entry.depth > depth {
                    continue;
                } else {
                    skip_depth = None;
                }
            }

            result.push((idx, entry));

            // If this is a collapsed directory, skip its children
            if entry.is_dir && !entry.expanded {
                skip_depth = Some(entry.depth);
            }
        }

        result
    }
}

/// Recursively scan a directory for markdown files and subdirectories.
fn scan_markdown_tree(root: &Utf8PathBuf, depth: usize) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = std::fs::read_dir(root) else {
        return entries;
    };

    let mut items: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    // Sort: directories first, then alphabetically
    items.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for item in items {
        let path = item.path();
        let Ok(utf8_path) = Utf8PathBuf::try_from(path.clone()) else {
            continue;
        };

        // Skip hidden files/directories
        if utf8_path
            .file_name()
            .map(|n| n.starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }

        let is_dir = path.is_dir();

        if is_dir {
            // Check if directory contains any markdown files (recursively)
            let children = scan_markdown_tree(&utf8_path, depth + 1);
            if !children.is_empty() {
                entries.push(FileEntry::new(utf8_path, true, depth));
                entries.extend(children);
            }
        } else {
            // Only include markdown files
            let ext = utf8_path.extension().unwrap_or("");
            if ext == "md" || ext == "markdown" || ext == "mdown" {
                entries.push(FileEntry::new(utf8_path, false, depth));
            }
        }
    }

    entries
}
