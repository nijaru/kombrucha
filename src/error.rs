use thiserror::Error;

#[derive(Error, Debug)]
pub enum BruError {
    #[error("API request failed: {0}")]
    ApiError(#[from] reqwest::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    #[error("Cask not found: {0}")]
    CaskNotFound(String),

    #[error("Network error: {0}")]
    #[allow(dead_code)]
    NetworkError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, BruError>;
