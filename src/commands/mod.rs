// New modular command structure
pub mod alias;
pub mod analytics;
pub mod autoremove;
pub mod bundle;
pub mod cache;
pub mod cat;
pub mod cleanup;
pub mod commands;
pub mod config;
pub mod deps;
pub mod desc;
pub mod edit;
pub mod fetch;
pub mod gist_logs;
pub mod home;
pub mod info;
pub mod install;
pub mod leaves;
pub mod link;
pub mod list;
pub mod log_cmd;
pub mod missing;
pub mod options;
pub mod pin;
pub mod reinstall;
pub mod search;
pub mod services;
pub mod shellenv;
pub mod tap;
pub mod uninstall;
pub mod update;
pub mod upgrade;
pub mod uses;
pub mod which;

// Re-export old command functions that haven't been migrated yet
#[path = "../commands_old.rs"]
mod old_commands;
pub use old_commands::*;
