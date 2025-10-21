//! Progress tracking and terminal integration
//!
//! Provides progress bars, spinners, and native terminal progress indicators
//! for long-running operations.

use std::env;
use std::io::{self, Write};
use std::time::Instant;

/// Terminal capabilities for native progress integration
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TerminalCapabilities {
    /// OSC 9;4 support (Ghostty, Windows Terminal, ConEmu)
    OSC94,
    /// No native progress support
    #[allow(dead_code)]
    None,
}

/// Progress state for terminal indicators
#[derive(Debug, Clone, Copy)]
pub enum ProgressState {
    Off = 0,
    Indeterminate = 1,
    Normal = 2,
    Error = 3,
    #[allow(dead_code)]
    Warning = 4,
}

/// Detects terminal capabilities
pub fn detect_terminal_capabilities() -> TerminalCapabilities {
    // Check TERM_PROGRAM for known terminals
    match env::var("TERM_PROGRAM").ok().as_deref() {
        Some("ghostty") => return TerminalCapabilities::OSC94,
        Some("WezTerm") => return TerminalCapabilities::OSC94,
        _ => {}
    }

    // Check for Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return TerminalCapabilities::OSC94;
    }

    // Check for ConEmu
    if env::var("ConEmuPID").is_ok() {
        return TerminalCapabilities::OSC94;
    }

    // Default to OSC 9;4 anyway - it's gracefully ignored by unsupported terminals
    // This gives us the widest compatibility
    TerminalCapabilities::OSC94
}

/// Set terminal progress using OSC 9;4
///
/// This works in Ghostty, Windows Terminal, ConEmu, and is gracefully ignored
/// by terminals that don't support it.
pub fn set_terminal_progress(progress: u8, state: ProgressState) {
    let progress = progress.min(100);
    print!("\x1b]9;4;{};{}\x1b\\", state as u8, progress);
    let _ = io::stdout().flush();
}

/// Clear terminal progress
pub fn clear_terminal_progress() {
    set_terminal_progress(0, ProgressState::Off);
}

/// Progress manager with terminal integration
pub struct ProgressManager {
    capabilities: TerminalCapabilities,
    start_time: Instant,
    total_items: Option<usize>,
    current_item: usize,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        Self {
            capabilities: detect_terminal_capabilities(),
            start_time: Instant::now(),
            total_items: None,
            current_item: 0,
        }
    }

    /// Start tracking progress with a known total
    pub fn start_with_total(&mut self, total: usize) {
        self.total_items = Some(total);
        self.current_item = 0;
        self.start_time = Instant::now();
        self.update_terminal_progress();
    }

    /// Start tracking progress without a known total (indeterminate)
    pub fn start_indeterminate(&mut self) {
        self.total_items = None;
        self.current_item = 0;
        self.start_time = Instant::now();
        if self.capabilities == TerminalCapabilities::OSC94 {
            set_terminal_progress(0, ProgressState::Indeterminate);
        }
    }

    /// Update progress to the next item
    pub fn inc(&mut self) {
        self.current_item += 1;
        self.update_terminal_progress();
    }

    #[allow(dead_code)]
    /// Set progress to a specific item
    pub fn set(&mut self, current: usize) {
        self.current_item = current;
        self.update_terminal_progress();
    }

    /// Finish progress tracking
    pub fn finish(&mut self) {
        if self.capabilities == TerminalCapabilities::OSC94 {
            clear_terminal_progress();
        }
    }

    /// Finish with success state
    pub fn finish_success(&mut self) {
        if self.capabilities == TerminalCapabilities::OSC94 {
            set_terminal_progress(100, ProgressState::Normal);
            // Brief delay to show 100%, then clear
            std::thread::sleep(std::time::Duration::from_millis(100));
            clear_terminal_progress();
        }
    }

    /// Finish with error state
    pub fn finish_error(&mut self) {
        if self.capabilities == TerminalCapabilities::OSC94 {
            set_terminal_progress(100, ProgressState::Error);
            // Keep error state visible
            std::thread::sleep(std::time::Duration::from_millis(200));
            clear_terminal_progress();
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Format elapsed time as string
    pub fn format_elapsed(&self) -> String {
        let elapsed = self.elapsed();
        let ms = elapsed.as_millis();
        if ms < 1000 {
            format!("{}ms", ms)
        } else {
            format!("{:.1}s", elapsed.as_secs_f64())
        }
    }

    /// Get current progress percentage (0-100)
    pub fn percent(&self) -> u8 {
        if let Some(total) = self.total_items {
            if total > 0 {
                ((self.current_item as f64 / total as f64) * 100.0).min(100.0) as u8
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Update terminal progress indicator
    fn update_terminal_progress(&self) {
        if self.capabilities != TerminalCapabilities::OSC94 {
            return;
        }

        if let Some(_total) = self.total_items {
            let percent = self.percent();
            set_terminal_progress(percent, ProgressState::Normal);
        }
        // For indeterminate, we set it once in start_indeterminate()
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProgressManager {
    fn drop(&mut self) {
        // Always clean up terminal progress on drop
        if self.capabilities == TerminalCapabilities::OSC94 {
            clear_terminal_progress();
        }
    }
}
#[allow(dead_code)]

/// Helper to check if we should show progress indicators
pub fn should_show_progress() -> bool {
    use atty::Stream;

    // Don't show if output is not a TTY (piped, redirected)
    if !atty::is(Stream::Stdout) {
        return false;
    }

    // Don't show if NO_COLOR is set
    if env::var("NO_COLOR").is_ok() {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_percent() {
        let mut pm = ProgressManager::new();
        pm.start_with_total(100);

        assert_eq!(pm.percent(), 0);

        pm.set(50);
        assert_eq!(pm.percent(), 50);

        pm.set(100);
        assert_eq!(pm.percent(), 100);
    }

    #[test]
    fn test_progress_percent_edge_cases() {
        let mut pm = ProgressManager::new();

        // No total set
        assert_eq!(pm.percent(), 0);

        // Zero total
        pm.start_with_total(0);
        assert_eq!(pm.percent(), 0);

        // Exceeds total
        pm.start_with_total(10);
        pm.set(15);
        assert_eq!(pm.percent(), 100); // Capped at 100
    }

    #[test]
    fn test_elapsed_formatting() {
        let pm = ProgressManager::new();
        let formatted = pm.format_elapsed();
        // Should be very short since just created
        assert!(formatted.ends_with("ms") || formatted.ends_with("s"));
    }
}
