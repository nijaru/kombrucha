//! Homebrew tap management - adding and removing third-party repositories

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Get the Taps directory path
pub fn taps_path() -> PathBuf {
    crate::cellar::detect_prefix().join("Library/Taps")
}

/// Parse a tap name into (user, repo) components
/// Input: "user/repo" → Output: ("user", "homebrew-repo")
fn parse_tap_name(tap: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = tap.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid tap name format. Expected 'user/repo', got '{}'",
            tap
        ));
    }

    let user = parts[0].to_string();
    let mut repo = parts[1].to_string();

    // Add homebrew- prefix if not present
    if !repo.starts_with("homebrew-") {
        repo = format!("homebrew-{}", repo);
    }

    Ok((user, repo))
}

/// Get the directory path for a tap
pub fn tap_directory(tap: &str) -> Result<PathBuf> {
    let (user, repo) = parse_tap_name(tap)?;
    Ok(taps_path().join(user).join(repo))
}

/// List all installed taps
pub fn list_taps() -> Result<Vec<String>> {
    let taps_dir = taps_path();

    if !taps_dir.exists() {
        return Ok(vec![]);
    }

    let mut taps = Vec::new();

    for user_entry in fs::read_dir(&taps_dir)? {
        let user_entry = user_entry?;
        let user = user_entry.file_name().to_string_lossy().to_string();

        if user.starts_with('.') {
            continue;
        }

        let user_path = user_entry.path();
        if !user_path.is_dir() {
            continue;
        }

        for repo_entry in fs::read_dir(user_path)? {
            let repo_entry = repo_entry?;
            let repo = repo_entry.file_name().to_string_lossy().to_string();

            if repo.starts_with('.') {
                continue;
            }

            // Remove "homebrew-" prefix for display
            let display_repo = repo.strip_prefix("homebrew-").unwrap_or(&repo);
            taps.push(format!("{}/{}", user, display_repo));
        }
    }

    taps.sort();
    Ok(taps)
}

/// Check if a tap is installed
pub fn is_tapped(tap: &str) -> Result<bool> {
    let tap_dir = tap_directory(tap)?;
    Ok(tap_dir.exists() && tap_dir.join(".git").exists())
}

/// Add a tap (clone the git repository)
pub fn tap(tap_name: &str) -> Result<()> {
    let (user, repo) = parse_tap_name(tap_name)?;
    let tap_dir = tap_directory(tap_name)?;

    if tap_dir.exists() {
        return Err(anyhow!("Tap {}/{} already exists", user, repo));
    }

    // Create user directory if it doesn't exist
    let user_dir = taps_path().join(&user);
    if !user_dir.exists() {
        fs::create_dir_all(&user_dir)
            .with_context(|| format!("Failed to create directory: {}", user_dir.display()))?;
    }

    // Clone the repository
    let git_url = format!("https://github.com/{}/{}.git", user, repo);

    let output = Command::new("git")
        .args(["clone", "--depth", "1", &git_url, tap_dir.to_str().unwrap()])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to clone tap: {}", stderr));
    }

    Ok(())
}

/// Remove a tap (delete the git repository)
pub fn untap(tap_name: &str) -> Result<()> {
    let tap_dir = tap_directory(tap_name)?;

    if !tap_dir.exists() {
        return Err(anyhow!("Tap {} is not installed", tap_name));
    }

    fs::remove_dir_all(&tap_dir)
        .with_context(|| format!("Failed to remove tap directory: {}", tap_dir.display()))?;

    // Remove user directory if empty
    if let Some(user_dir) = tap_dir.parent()
        && user_dir.exists() && fs::read_dir(user_dir)?.next().is_none() {
            fs::remove_dir(user_dir)?;
        }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tap_name() {
        let (user, repo) = parse_tap_name("user/repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");

        let (user, repo) = parse_tap_name("user/homebrew-repo").unwrap();
        assert_eq!(user, "user");
        assert_eq!(repo, "homebrew-repo");
    }

    #[test]
    fn test_parse_tap_name_invalid() {
        assert!(parse_tap_name("invalid").is_err());
        assert!(parse_tap_name("too/many/slashes").is_err());
    }

    #[test]
    fn test_tap_directory() {
        let dir = tap_directory("user/repo").unwrap();
        assert!(dir.to_string_lossy().contains("Taps/user/homebrew-repo"));
    }
}
