mod api;
mod cache;
mod cask;
mod cellar;
mod colors;
mod commands;
mod download;
mod error;
mod extract;
mod platform;
mod receipt;
mod relocate;
mod services;
mod symlink;
mod tap;

use clap::Parser;
use colored::Colorize;
use commands::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
    Ok(())
}

async fn run() -> anyhow::Result<()> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        // SAFETY: Setting RUST_LOG before any threads spawn is safe. This is the entry point
        // and no other threads have been created yet. Environment variable is only read by
        // tracing_subscriber initialization immediately after this.
        unsafe {
            std::env::set_var("RUST_LOG", "warn");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    // Handle broken pipe errors gracefully (e.g., when piping to `head`)
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Check if this is a broken pipe error
        if let Some(s) = panic_info.payload().downcast_ref::<String>()
            && s.contains("Broken pipe")
        {
            std::process::exit(0);
        }
        if let Some(s) = panic_info.payload().downcast_ref::<&str>()
            && s.contains("Broken pipe")
        {
            std::process::exit(0);
        }
        // Otherwise, use the default panic handler
        default_panic(panic_info);
    }));

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // Check if this is an unrecognized command error
            let err_str = err.to_string();
            if err_str.contains("unrecognized subcommand") {
                // Extract the command and arguments to pass to brew
                let args: Vec<String> = std::env::args().skip(1).collect();

                // Fall back to brew
                let status = std::process::Command::new("brew").args(&args).status();

                match status {
                    Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                    Err(_) => {
                        eprintln!("{} brew not available for fallback", "Error:".red().bold());
                        std::process::exit(1);
                    }
                }
            } else {
                // Let clap handle the error (print help, etc.)
                err.exit();
            }
        }
    };

    // Set NO_COLOR if --no-color flag is set
    if cli.no_color {
        // SAFETY: Setting NO_COLOR early in main before colors are initialized is safe.
        // This happens before any concurrent access and is only read by colors::init_colors().
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
    }

    // Initialize color support (respects NO_COLOR, CLICOLOR, TTY)
    colors::init_colors();

    // Set quiet mode environment variable for commands to check
    if cli.quiet {
        // SAFETY: Setting BRU_QUIET early in main is safe. This happens in the main thread
        // before command execution and is only read (not modified) by child operations.
        unsafe {
            std::env::set_var("BRU_QUIET", "1");
        }
    }

    // Dispatch to commands module
    // SAFETY: Ruby VM is a singleton and must be initialized once.
    let ruby = unsafe { magnus::embed::init() };
    commands::run(cli, &ruby).await?;

    Ok(())
}