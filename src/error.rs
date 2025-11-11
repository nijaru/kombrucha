//! Error types for Kombrucha library operations.
//!
//! This module defines the error types returned by library functions. All errors are
//! variants of [`BruError`], which provides context-aware error messages suitable for
//! both programmatic handling and user-facing display.
//!
//! # Error Handling Strategy
//!
//! The library uses `thiserror` for ergonomic error construction and conversion. Most
//! errors automatically convert from underlying libraries (reqwest, serde_json, std::io).
//! For custom error handling, use the [`Other`](BruError::Other) variant with `anyhow`
//! for rich context.
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::{BrewApi, BruError};
//!
//! async fn fetch_safe(name: &str) -> Result<(), String> {
//!     let api = BrewApi::new().map_err(|e| format!("API init failed: {}", e))?;
//!     match api.fetch_formula(name).await {
//!         Ok(formula) => {
//!             println!("Found: {}", formula.name);
//!             Ok(())
//!         }
//!         Err(BruError::FormulaNotFound(name)) => Err(format!("'{}' not in Homebrew", name)),
//!         Err(e) => Err(format!("Error: {}", e)),
//!     }
//! }
//! ```

use thiserror::Error;

/// Error type for all Kombrucha library operations.
///
/// This enum represents all error conditions that can occur when using the Kombrucha
/// library. Each variant includes specific context about what failed.
///
/// # Variants
///
/// - [`ApiError`](BruError::ApiError): Network or HTTP request failure when communicating
///   with the Homebrew JSON API
/// - [`JsonError`](BruError::JsonError): Failed to parse JSON from API or local cache
/// - [`FormulaNotFound`](BruError::FormulaNotFound): The requested formula doesn't exist
///   in Homebrew
/// - [`CaskNotFound`](BruError::CaskNotFound): The requested cask doesn't exist in Homebrew
/// - [`NetworkError`](BruError::NetworkError): Generic network connectivity error
/// - [`IoError`](BruError::IoError): File system operation failed (Cellar access, cache, etc.)
/// - [`Other`](BruError::Other): Miscellaneous error with rich context from `anyhow`
///
/// # Converting from Other Error Types
///
/// Most common errors automatically convert to `BruError`:
///
/// ```no_run
/// use kombrucha::Result;
/// use std::fs;
///
/// fn read_file() -> Result<String> {
///     // std::io::Error automatically converts to BruError::IoError
///     Ok(fs::read_to_string("/path/to/file")?)
/// }
/// ```
#[derive(Error, Debug)]
pub enum BruError {
    /// Network or HTTP request failed when communicating with Homebrew API.
    ///
    /// This typically indicates connectivity issues, timeout, or API unavailability.
    /// The wrapped `reqwest::Error` contains details about the specific failure.
    #[error("API request failed: {0}")]
    ApiError(#[from] reqwest::Error),

    /// Failed to parse JSON from API response or local cache file.
    ///
    /// This usually indicates either corrupted cache files or an unexpected API
    /// response format. The wrapped error contains the specific parsing failure.
    #[error("Failed to parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    /// A formula with the given name was not found in Homebrew.
    ///
    /// This doesn't necessarily indicate an error in your code - the formula may have
    /// been removed from Homebrew, or the name may be misspelled.
    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    /// A cask with the given token was not found in Homebrew.
    ///
    /// Like [`FormulaNotFound`](BruError::FormulaNotFound), this may indicate the cask
    /// was removed or the token is incorrect.
    #[error("Cask not found: {0}")]
    CaskNotFound(String),

    /// Generic network error with custom message.
    ///
    /// Used for network-related errors that don't fit other categories. Prefer
    /// [`ApiError`](BruError::ApiError) for HTTP-specific failures.
    #[error("Network error: {0}")]
    #[allow(dead_code)]
    NetworkError(String),

    /// File system operation failed (reading Cellar, cache, taps, etc.).
    ///
    /// This wraps `std::io::Error` and typically indicates permission issues, missing
    /// directories, or corrupted file state.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Miscellaneous error with rich context.
    ///
    /// Used for errors that don't fit other categories. The wrapped `anyhow::Error`
    /// preserves error context chains for debugging.
    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Convenience type alias for library operations.
///
/// All fallible functions in the Kombrucha library return `Result<T>`, which is
/// equivalent to `std::result::Result<T, BruError>`.
///
/// # Examples
///
/// ```no_run
/// use kombrucha::Result;
///
/// async fn list_packages() -> Result<Vec<String>> {
///     // Return type is Result<Vec<String>> = std::result::Result<Vec<String>, BruError>
///     Ok(vec![])
/// }
/// ```
pub type Result<T> = std::result::Result<T, BruError>;
