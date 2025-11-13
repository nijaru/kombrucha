//! Platform detection for selecting the correct bottle type.
//!
//! This module detects the current system's OS and architecture to determine which
//! precompiled bottle to download. Homebrew maintains separate bottles for:
//! - **macOS versions**: Sequoia (15), Sonoma (14), Ventura (13), etc.
//! - **CPU architectures**: ARM64 (Apple Silicon), x86_64 (Intel)
//! - **Linux variants**: x86_64, ARM64 variants
//!
//! # Bottle Tag Format
//!
//! Bottles are named with platform tags like `arm64_sequoia` or `x86_64_sonoma`:
//! - `<arch>_<os_version>`
//! - Examples: `arm64_sequoia`, `x86_64_ventura`, `arm64_linux`
//!
//! If an exact platform bottle isn't available, Homebrew falls back to universal
//! bottles tagged as `all`.
//!
//! # Examples
//!
//! ```no_run
//! use kombrucha::platform;
//!
//! fn main() -> anyhow::Result<()> {
//!     let bottle_tag = platform::detect_bottle_tag()?;
//!     println!("This system needs: {}", bottle_tag);
//!     // Output: "arm64_sequoia" on M3 Mac with macOS 15
//!     // Output: "x86_64_ventura" on Intel Mac with macOS 13
//!
//!     Ok(())
//! }
//! ```

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;
#[cfg(target_os = "macos")]
use std::process::Command;

/// Detect the current system platform for bottle selection.
///
/// Returns a platform tag that identifies which precompiled bottle variant to download.
/// Homebrew maintains different bottles for different macOS versions and CPU architectures.
///
/// # Platform Tags
///
/// Examples of returned tags:
/// - `arm64_sequoia` - Apple Silicon (M1+) on macOS 15
/// - `x86_64_ventura` - Intel on macOS 13
/// - `arm64_linux` - ARM64 Linux
/// - `x86_64_linux` - x86_64 Linux
///
/// # Returns
///
/// A string identifying the platform (e.g., `"arm64_sequoia"`).
///
/// # Errors
///
/// Returns an error if:
/// - On macOS: `sw_vers` command is unavailable
/// - On unsupported platforms: Not macOS or Linux
///
/// # Examples
///
/// ```no_run
/// use kombrucha::platform;
///
/// fn main() -> anyhow::Result<()> {
///     let tag = platform::detect_bottle_tag()?;
///     println!("Bottle tag: {}", tag);
///     // Output: "arm64_sequoia" (on M3 Mac)
///     // Output: "x86_64_ventura" (on Intel Mac)
///
///     Ok(())
/// }
/// ```
///
/// # Fallback Behavior
///
/// If exact platform bottles aren't available, Homebrew falls back to universal bottles
/// tagged as `all`. This function returns the preferred tag, not the fallback.
pub fn detect_bottle_tag() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        // Homebrew uses "arm64" not "aarch64"
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            other => other,
        };
        let os_version = macos_version()?;
        let os_name = macos_name(&os_version);

        Ok(format!("{}_{}", arch, os_name))
    }

    #[cfg(target_os = "linux")]
    {
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            other => other,
        };
        Ok(format!("{}_linux", arch))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        anyhow::bail!("Unsupported platform")
    }
}

#[cfg(target_os = "macos")]
fn macos_version() -> Result<String> {
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .context("Failed to run sw_vers")?;

    let version = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in sw_vers output")?
        .trim()
        .to_string();

    Ok(version)
}

#[cfg(target_os = "macos")]
fn macos_name(version: &str) -> &'static str {
    // Parse major version
    let major: u32 = version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    match major {
        26 => "tahoe",    // macOS 26 (Tahoe) - year-based versioning
        16 => "tahoe",    // macOS 16 (Tahoe) - compatibility version number
        15 => "sequoia",  // macOS 15
        14 => "sonoma",   // macOS 14
        13 => "ventura",  // macOS 13
        12 => "monterey", // macOS 12
        11 => "big_sur",  // macOS 11
        _ => "sonoma",    // Default to recent compatible version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bottle_tag() {
        let tag = detect_bottle_tag().unwrap();
        assert!(!tag.is_empty());
        #[cfg(target_arch = "aarch64")]
        assert!(tag.starts_with("arm64_"));
        #[cfg(target_arch = "x86_64")]
        assert!(tag.starts_with("x86_64_"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_names() {
        assert_eq!(macos_name("15.1"), "sequoia");
        assert_eq!(macos_name("14.0"), "sonoma");
        assert_eq!(macos_name("13.0"), "ventura");
        assert_eq!(macos_name("12.0"), "monterey");
    }
}
