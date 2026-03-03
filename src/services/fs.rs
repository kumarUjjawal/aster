use crate::error::{AppError, AppResult};
use camino::Utf8PathBuf;
use futures::channel::oneshot;
use gpui::App;
use rfd::AsyncFileDialog;
use std::fs;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Opens a native file picker and returns a single selected markdown file.
///
/// The dialog is filtered to markdown extensions so users only see selectable
/// markdown files while browsing.
pub fn pick_open_markdown_path_async() -> impl Future<Output = Option<Utf8PathBuf>> + Send {
    let home_dir = directories::UserDirs::new().map(|d| d.home_dir().to_path_buf());
    let mut dialog = AsyncFileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "mdown"])
        .set_title("Open Markdown File");

    if let Some(dir) = home_dir {
        dialog = dialog.set_directory(dir);
    }

    async move {
        dialog
            .pick_file()
            .await
            .map(|handle| handle.path().to_path_buf())
            .and_then(|path| Utf8PathBuf::try_from(path).ok())
            .filter(is_markdown_path)
    }
}

/// Returns true if the path has a supported markdown extension.
pub fn is_markdown_path(path: &Utf8PathBuf) -> bool {
    path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("md")
            || ext.eq_ignore_ascii_case("markdown")
            || ext.eq_ignore_ascii_case("mdown")
    })
}

/// Async version of pick_save_path using GPUI's native dialog
/// Returns a receiver that will contain the selected path
pub fn pick_save_path_async(
    cx: &App,
    default: Option<&Utf8PathBuf>,
) -> oneshot::Receiver<Result<Option<PathBuf>>> {
    let home_dir = directories::UserDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let (directory, suggested_name) = if let Some(path) = default {
        let dir = path
            .parent()
            .map(|p| p.as_std_path().to_path_buf())
            .unwrap_or_else(|| home_dir.clone());
        let name = path.file_name().unwrap_or("untitled.md");
        (dir, Some(name))
    } else {
        (home_dir, Some("untitled.md"))
    };

    cx.prompt_for_new_path(&directory, suggested_name)
}

pub fn read_to_string(path: &Utf8PathBuf) -> AppResult<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn write_atomic(path: &Utf8PathBuf, contents: &str) -> AppResult<()> {
    let mut tmp = NamedTempFile::new_in(
        path.parent()
            .and_then(|p| Utf8PathBuf::try_from(p.to_path_buf()).ok())
            .unwrap_or_else(|| {
                Utf8PathBuf::try_from(std::env::temp_dir())
                    .unwrap_or_else(|_| Utf8PathBuf::from("tmp"))
            }),
    )?;
    tmp.write_all(contents.as_bytes())?;
    tmp.flush()?;
    tmp.persist(path).map_err(|e| AppError::Io(e.error))?;
    Ok(())
}

// Re-export Result for use with oneshot receivers
pub use anyhow::Result;

#[cfg(test)]
mod tests {
    use super::is_markdown_path;
    use camino::Utf8PathBuf;

    #[test]
    fn detects_supported_markdown_extensions() {
        assert!(is_markdown_path(&Utf8PathBuf::from("/tmp/doc.md")));
        assert!(is_markdown_path(&Utf8PathBuf::from("/tmp/doc.MD")));
        assert!(is_markdown_path(&Utf8PathBuf::from("/tmp/doc.markdown")));
        assert!(is_markdown_path(&Utf8PathBuf::from("/tmp/doc.mdown")));
    }

    #[test]
    fn rejects_non_markdown_extensions() {
        assert!(!is_markdown_path(&Utf8PathBuf::from("/tmp/doc.txt")));
        assert!(!is_markdown_path(&Utf8PathBuf::from("/tmp/doc")));
    }
}
