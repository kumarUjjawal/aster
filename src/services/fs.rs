use crate::error::{AppError, AppResult};
use camino::Utf8PathBuf;
use rfd::FileDialog;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

pub fn pick_open_path() -> Option<Utf8PathBuf> {
    FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "mdown"])
        .pick_file()
        .and_then(|p| Utf8PathBuf::try_from(p).ok())
}

pub fn pick_save_path(default: Option<&Utf8PathBuf>) -> Option<Utf8PathBuf> {
    let mut dialog = FileDialog::new().add_filter("Markdown", &["md", "markdown", "mdown"]);
    if let Some(path) = default {
        if let Some(parent) = path.parent() {
            let parent_path = parent.as_std_path();
            let mut dir: Option<PathBuf> = None;
            if parent_path.is_absolute() {
                dir = Some(parent_path.to_path_buf());
            } else if let Ok(cwd) = std::env::current_dir() {
                dir = Some(cwd.join(parent_path));
            }
            if let Some(dir) = dir.filter(|p| p.is_dir()) {
                dialog = dialog.set_directory(dir);
            }
        }
        dialog = dialog.set_file_name(path.file_name().unwrap_or("untitled.md"));
    } else {
        dialog = dialog.set_file_name("untitled.md");
    }
    dialog
        .save_file()
        .and_then(|p| Utf8PathBuf::try_from(p).ok())
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
