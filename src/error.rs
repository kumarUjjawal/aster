use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("markdown parse failed: {0}")]
    Markdown(String),
}

pub type AppResult<T> = Result<T, AppError>;
