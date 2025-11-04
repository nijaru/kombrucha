//! Library interface for kombrucha (bru) package manager
//!
//! This library exposes core functionality for testing and potential future use.

pub mod cellar;
pub mod symlink;

// Re-export commonly used functions
pub use symlink::{link_formula, normalize_path, unlink_formula};
