//! Platform detection for selecting the correct bottle

use anyhow::{Context, Result};
use std::process::Command;

/// Detect the current platform for bottle selection
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
        16 => "sequoia",  // macOS 16
        15 => "sequoia",  // macOS 15
        14 => "sonoma",   // macOS 14
        13 => "ventura",  // macOS 13
        12 => "monterey", // macOS 12
        11 => "big_sur",  // macOS 11
        _ => "sonoma",    // Default to recent
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
