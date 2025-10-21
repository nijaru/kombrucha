/// Color support with NO_COLOR and CLICOLOR environment variable handling
///
/// Implements the NO_COLOR standard (https://no-color.org/) and traditional
/// CLICOLOR conventions for disabling terminal colors.
///
/// **Environment Variables**:
/// - `NO_COLOR`: If set (to any value), disable colors
/// - `CLICOLOR`: If set to 0, disable colors
/// - `CLICOLOR_FORCE`: If set to non-zero, force colors even when not a TTY
///
/// **TTY Detection**: Colors are automatically disabled if stdout is not a terminal,
/// unless CLICOLOR_FORCE is set.
use colored::control;

/// Initialize color support by checking environment variables and TTY status
///
/// This function implements the NO_COLOR standard and CLICOLOR conventions.
/// Call this early in main() to configure color output for the entire program.
pub fn init_colors() {
    // NO_COLOR takes precedence over everything (https://no-color.org/)
    if std::env::var("NO_COLOR").is_ok() {
        control::set_override(false);
        return;
    }

    // CLICOLOR_FORCE overrides both CLICOLOR and TTY detection
    if std::env::var("CLICOLOR_FORCE")
        .map(|v| v != "0")
        .unwrap_or(false)
    {
        control::set_override(true);
        return;
    }

    // CLICOLOR=0 disables colors
    if std::env::var("CLICOLOR").map(|v| v == "0").unwrap_or(false) {
        control::set_override(false);
        return;
    }

    // Default: use colors only if stdout is a TTY
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());
    control::set_override(is_tty);
}
