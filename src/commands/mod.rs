// New modular command structure
pub mod deps;
pub mod info;
pub mod search;
pub mod uses;

// Re-export old command functions that haven't been migrated yet
#[path = "../commands_old.rs"]
mod old_commands;
pub use old_commands::*;
