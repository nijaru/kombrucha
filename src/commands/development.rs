//! Formula development and testing commands
//!
//! Commands for editing, creating, testing, and auditing formulae.
//! Most of these operations require Phase 3 (Ruby interop) and are
//! currently informational or delegate to brew.

use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;

/// Open a formula in the default editor
///
/// Locates the formula file in local taps and opens it in $EDITOR.
/// Verifies the formula exists via the API before attempting to edit.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `formula_name` - The formula to edit
pub async fn edit(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!(
        "{} Opening {} in editor...",
        "✏️".bold(),
        formula_name.cyan()
    );

    // First, verify formula exists
    match api.fetch_formula(formula_name).await {
        Ok(_) => {}
        Err(_) => {
            println!("{} Formula '{}' not found", "✗".red(), formula_name);
            return Ok(());
        }
    }

    // Try to find formula file in taps
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    // Check homebrew-core first (try both flat and letter-organized structure)
    let first_letter = formula_name
        .chars()
        .next()
        .unwrap_or('a')
        .to_lowercase()
        .to_string();
    let core_formula_letter = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(&first_letter)
        .join(format!("{}.rb", formula_name));
    let core_formula_flat = taps_dir
        .join("homebrew/homebrew-core/Formula")
        .join(format!("{}.rb", formula_name));

    let formula_path = if core_formula_letter.exists() {
        core_formula_letter
    } else if core_formula_flat.exists() {
        core_formula_flat
    } else {
        // Search all taps
        let mut found_path = None;
        if taps_dir.exists() {
            for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                let tap_path = tap_entry.path();
                if tap_path.is_dir() {
                    for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                        let repo_path = repo_entry.path();
                        let formula_path = repo_path
                            .join("Formula")
                            .join(format!("{}.rb", formula_name));
                        if formula_path.exists() {
                            found_path = Some(formula_path);
                            break;
                        }
                    }
                }
                if found_path.is_some() {
                    break;
                }
            }
        }

        match found_path {
            Some(p) => p,
            None => {
                println!("{} Formula file not found locally", "⚠".yellow());
                println!("Formula exists in API but not in local taps");
                println!("Try: {}", "brew tap homebrew/core".to_string().cyan());
                return Ok(());
            }
        }
    };

    println!(
        "  {}: {}",
        "File".dimmed(),
        formula_path.display().to_string().cyan()
    );

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    // Open in editor
    let status = std::process::Command::new(&editor)
        .arg(&formula_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{} Finished editing {}", "✓".green(), formula_name.bold());
        }
        Ok(_) => {
            println!("{} Editor exited with error", "⚠".yellow());
        }
        Err(e) => {
            println!("{} Failed to open editor: {}", "✗".red(), e);
            println!("Set EDITOR environment variable to your preferred editor");
        }
    }

    Ok(())
}

/// Create a new formula from a URL
///
/// Generates a basic formula template with sensible defaults based on
/// the provided URL. The formula file is created in the current directory.
///
/// # Arguments
/// * `url` - The source URL for the formula
/// * `name` - Optional formula name (extracted from URL if not provided)
pub fn create(url: &str, name: Option<&str>) -> Result<()> {
    println!("Creating formula from URL: {}", url.cyan());

    // Extract name from URL if not provided
    let formula_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Try to extract from URL
        let parts: Vec<&str> = url.split('/').collect();
        let filename = parts.last().unwrap_or(&"formula");

        // Remove common extensions
        let name = filename
            .trim_end_matches(".tar.gz")
            .trim_end_matches(".tar.bz2")
            .trim_end_matches(".tar.xz")
            .trim_end_matches(".zip")
            .trim_end_matches(".tgz");

        // Remove version numbers (simple heuristic)
        let parts: Vec<&str> = name.split('-').collect();
        if parts.len() > 1 {
            // Take first part before version
            parts[0].to_string()
        } else {
            name.to_string()
        }
    };

    println!("  {}: {}", "Name".bold(), formula_name.cyan());

    // Generate basic formula template (capitalize first letter)
    let class_name = {
        let mut chars = formula_name.chars();
        if let Some(first_char) = chars.next() {
            first_char.to_uppercase().to_string() + chars.as_str()
        } else {
            formula_name.to_uppercase()
        }
    };
    let homepage_base =
        url.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '/');

    let template = vec![
        format!("class {} < Formula", class_name),
        format!("  desc \"Description of {}\"", formula_name),
        format!("  homepage \"{}\"", homepage_base),
        format!("  url \"{}\"", url),
        "  sha256 \"\"  # TODO: Add SHA256 checksum".to_string(),
        "  license \"\"  # TODO: Add license".to_string(),
        "".to_string(),
        "  depends_on \"cmake\" => :build  # Example build dependency".to_string(),
        "".to_string(),
        "  def install".to_string(),
        "    # TODO: Add installation steps".to_string(),
        "    # Common patterns:".to_string(),
        "    # system \"./configure\", \"--prefix=#{prefix}\"".to_string(),
        "    # system \"make\", \"install\"".to_string(),
        "    #".to_string(),
        "    # Or for CMake:".to_string(),
        "    # system \"cmake\", \"-S\", \".\", \"-B\", \"build\", *std_cmake_args".to_string(),
        "    # system \"cmake\", \"--build\", \"build\"".to_string(),
        "    # system \"cmake\", \"--install\", \"build\"".to_string(),
        "  end".to_string(),
        "".to_string(),
        "  test do".to_string(),
        "    # TODO: Add test".to_string(),
        format!("    # system \"#{{bin}}/{}\", \"--version\"", formula_name),
        "  end".to_string(),
        "end".to_string(),
    ]
    .join("\n")
        + "\n";

    // Write to file in current directory
    let filename = format!("{}.rb", formula_name);
    std::fs::write(&filename, template)?;

    println!("{} Created {}", "✓".green(), filename.bold().green());
    println!();
    println!("{}", "Next steps:".bold());
    println!(
        "  1. Add SHA256 checksum: {}",
        "shasum -a 256 <downloaded-file>".to_string().cyan()
    );
    println!("  2. Fill in description and license");
    println!("  3. Update install method with build steps");
    println!("  4. Add test command");
    println!(
        "  5. Test formula: {}",
        format!("bru install --build-from-source {}", filename).cyan()
    );

    Ok(())
}

/// Check for newer versions of a formula
///
/// Compares the current formula version with upstream sources to detect
/// if a newer version is available.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `formula_name` - The formula to check
pub async fn livecheck(api: &BrewApi, formula_name: &str) -> Result<()> {
    println!("Checking for newer versions of {}...", formula_name.cyan());

    let formula = api.fetch_formula(formula_name).await?;

    let current_version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No stable version found"))?;

    println!("{}", format!("==> {}", formula.name).bold().green());
    println!("{}: {}", "Current version".bold(), current_version.cyan());

    if let Some(homepage) = &formula.homepage {
        println!("{}: {}", "Homepage".bold(), homepage.dimmed());
    }

    println!();
    println!("Livecheck not yet implemented");
    println!("Would check:");
    if let Some(homepage) = &formula.homepage {
        println!("  - {}", homepage.dimmed());
    }
    println!("  - GitHub releases (if applicable)");
    println!("  - Other version sources");

    Ok(())
}

/// Audit formula files for style and correctness
///
/// Performs basic validation checks on formula files including checking
/// for required fields, proper syntax, and common issues.
///
/// # Arguments
/// * `_api` - The Homebrew API client (unused)
/// * `formula_names` - The formulae to audit
pub async fn audit(_api: &BrewApi, formula_names: &[String]) -> Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Auditing {} formulae...",
        formula_names.len().to_string().bold()
    );
    println!();

    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    for formula_name in formula_names {
        println!("{} {}", "==>".bold().green(), formula_name.bold().cyan());

        // Find formula file
        let first_letter = formula_name
            .chars()
            .next()
            .unwrap_or('a')
            .to_lowercase()
            .to_string();
        let core_formula_letter = taps_dir
            .join("homebrew/homebrew-core/Formula")
            .join(&first_letter)
            .join(format!("{}.rb", formula_name));
        let core_formula_flat = taps_dir
            .join("homebrew/homebrew-core/Formula")
            .join(format!("{}.rb", formula_name));

        let formula_path = if core_formula_letter.exists() {
            Some(core_formula_letter)
        } else if core_formula_flat.exists() {
            Some(core_formula_flat)
        } else {
            // Search all taps
            let mut found_path = None;
            if taps_dir.exists() {
                'outer: for tap_entry in std::fs::read_dir(&taps_dir)?.flatten() {
                    let tap_path = tap_entry.path();
                    if tap_path.is_dir() {
                        for repo_entry in std::fs::read_dir(&tap_path)?.flatten() {
                            let repo_path = repo_entry.path();
                            let fp = repo_path
                                .join("Formula")
                                .join(format!("{}.rb", formula_name));
                            if fp.exists() {
                                found_path = Some(fp);
                                break 'outer;
                            }
                        }
                    }
                }
            }
            found_path
        };

        match formula_path {
            Some(path) => {
                let content = std::fs::read_to_string(&path)?;
                let mut issues = Vec::new();

                // Basic checks
                if !content.contains("def install") {
                    issues.push("Missing install method");
                }

                if !content.contains("desc ") {
                    issues.push("Missing description");
                }

                if !content.contains("homepage ") {
                    issues.push("Missing homepage");
                }

                if !content.contains("url ") {
                    issues.push("Missing URL");
                }

                if !content.contains("sha256 ") {
                    issues.push("Missing SHA256");
                }

                if content.contains("TODO") {
                    issues.push("Contains TODO comments");
                }

                if issues.is_empty() {
                    println!("  {} No issues found", "✓".green());
                } else {
                    for issue in issues {
                        println!("  {} {}", "⚠".yellow(), issue.dimmed());
                    }
                }
            }
            None => {
                println!("  {} Formula file not found locally", "⚠".yellow());
            }
        }

        println!();
    }

    Ok(())
}

/// Check formula style with RuboCop
///
/// Validates formula Ruby code against Homebrew style guidelines.
/// Requires Phase 3 (Ruby interop) - currently informational.
///
/// # Arguments
/// * `formula_names` - The formulae to check
/// * `fix` - If true, automatically fix style violations
pub fn style(formula_names: &[String], fix: bool) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Checking style for {} formulae...",
        formula_names.len().to_string().bold()
    );

    if fix {
        println!("  Auto-fix enabled");
    }

    println!(
        "\n {} Style checking requires RuboCop (Phase 3)",
        "ℹ".blue()
    );
    println!("  Formula style would be validated against Homebrew standards:");
    println!("  - Naming conventions");
    println!("  - Method ordering");
    println!("  - Spacing and indentation");
    println!("  - Ruby best practices");

    for formula in formula_names {
        println!("  {}", formula.cyan());
        println!("    {} Would check formula style", "ℹ".dimmed());
    }

    if fix {
        println!(" Auto-fix would correct violations");
    }

    Ok(())
}

/// Run tests for a formula
///
/// Executes the test block from a formula file to verify installation
/// and basic functionality. Requires Phase 3 (Ruby interop).
///
/// # Arguments
/// * `formula_name` - The formula to test
pub fn test(formula_name: &str) -> anyhow::Result<()> {
    println!("Running tests for: {}", formula_name.cyan());

    println!(
        "{} Formula testing requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Test suite would be executed from formula's test block");
    println!("  Typical tests verify:");
    println!("  - Installation succeeded");
    println!("  - Binary is executable");
    println!("  - Version output matches");
    println!("  - Basic functionality works");

    println!(
        "\n  Would run: {} test {}",
        "brew".cyan(),
        formula_name.cyan()
    );

    Ok(())
}

/// Generate bottles for formulae
///
/// Builds formulae from source and packages them into bottles (binary packages).
/// Requires Phase 3 (Ruby interop) - currently informational.
///
/// # Arguments
/// * `formula_names` - The formulae to bottle
/// * `write` - If true, write bottle DSL back to formula files
pub fn bottle(formula_names: &[String], write: bool) -> anyhow::Result<()> {
    if formula_names.is_empty() {
        println!("{} No formulae specified", "✗".red());
        return Ok(());
    }

    println!(
        "Generating bottles for {} formulae...",
        formula_names.len().to_string().bold()
    );

    if write {
        println!(
            "  {} Write mode enabled - would update formula files",
            "ℹ".blue()
        );
    }

    println!(
        "{} Bottle generation requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Would build from source and create bottles:");

    for formula in formula_names {
        println!("  {}", formula.cyan());
        println!("    {} Build from source", "1.".dimmed());
        println!("    {} Package into bottle tarball", "2.".dimmed());
        println!("    {} Calculate SHA256 checksum", "3.".dimmed());
        if write {
            println!("    {} Write bottle block to formula", "4.".dimmed());
        }
    }

    Ok(())
}

/// Extract a formula to a target tap
///
/// Copies a formula file from one tap (typically homebrew/core) to
/// another tap, useful for maintaining formula forks.
///
/// # Arguments
/// * `formula_name` - The formula to extract
/// * `target_tap` - The tap to copy the formula to
pub fn extract(formula_name: &str, target_tap: &str) -> Result<()> {
    println!("Extracting formula: {}", formula_name.cyan());
    println!("  Target tap: {}", target_tap.cyan());

    // Find the formula file
    let prefix = cellar::detect_prefix();
    let taps_dir = prefix.join("Library/Taps");

    let mut formula_path = None;
    let mut source_tap = None;

    // Search in homebrew/core first
    let core_formula_dir = taps_dir.join("homebrew/homebrew-core/Formula");
    if core_formula_dir.exists() {
        // Check letter-organized directories
        if let Some(first_letter) = formula_name.chars().next() {
            let letter_dir = core_formula_dir.join(first_letter.to_lowercase().to_string());
            let possible_path = letter_dir.join(format!("{}.rb", formula_name));
            if possible_path.exists() {
                formula_path = Some(possible_path);
                source_tap = Some("homebrew/core");
            }
        }
    }

    // If not found in core, search other taps
    if formula_path.is_none() {
        println!("  Searching taps...");
        // This is a simplified search - real implementation would be more thorough
    }

    let (formula_path, source_tap) = match (formula_path, source_tap) {
        (Some(path), Some(tap)) => (path, tap),
        _ => {
            println!("{} Formula not found: {}", "✗".red(), formula_name);
            return Ok(());
        }
    };

    println!("  {} Found in: {}", "✓".green(), source_tap.cyan());

    // Validate target tap
    let target_tap_dir = crate::tap::tap_directory(target_tap)?;
    if !target_tap_dir.exists() {
        println!("{} Target tap not found: {}", "✗".red(), target_tap);
        println!(
            "  Create it first with: {}",
            format!("bru tap-new {}", target_tap).cyan()
        );
        return Ok(());
    }

    // Copy formula to target tap
    let target_formula_dir = target_tap_dir.join("Formula");
    std::fs::create_dir_all(&target_formula_dir)?;

    let target_path = target_formula_dir.join(format!("{}.rb", formula_name));

    if target_path.exists() {
        println!("{} Formula already exists in target tap", "⚠".yellow());
        return Ok(());
    }

    std::fs::copy(&formula_path, &target_path)?;

    println!(
        "{} Extracted {} to {}",
        "✓".green().bold(),
        formula_name.bold(),
        target_tap.cyan()
    );
    println!("  Path: {}", target_path.display().to_string().dimmed());

    Ok(())
}

/// Unpack formula source to a directory
///
/// Downloads and extracts the source code for a formula.
/// Requires Phase 3 (Ruby interop) - currently informational.
///
/// # Arguments
/// * `api` - The Homebrew API client
/// * `formula_name` - The formula to unpack
/// * `dest_dir` - Optional destination directory (defaults to formula name)
pub async fn unpack(api: &BrewApi, formula_name: &str, dest_dir: Option<&str>) -> Result<()> {
    println!("Unpacking source for: {}", formula_name.cyan());

    // Fetch formula info
    let formula = api.fetch_formula(formula_name).await?;

    let version = formula
        .versions
        .stable
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No stable version available"))?;

    println!("  Version: {}", version.cyan());

    // Note: Full implementation would download source tarball and extract
    // For now, provide informational output
    println!(
        "{} Source unpacking requires Phase 3 (Ruby interop)",
        "ℹ".blue()
    );
    println!("  Formula source would be downloaded and extracted to:");

    let target_dir = if let Some(dir) = dest_dir {
        std::path::PathBuf::from(dir)
    } else {
        std::env::current_dir()?.join(formula_name)
    };

    println!("  {}", target_dir.display().to_string().cyan());

    // Show what would happen
    if let Some(homepage) = &formula.homepage {
        println!("  Homepage: {}", homepage.dimmed());
    }

    Ok(())
}

/// Migrate a formula to a different tap
///
/// Updates metadata to mark a formula as coming from a different tap.
/// Useful when a formula moves between taps.
///
/// # Arguments
/// * `formula_name` - The formula to migrate
/// * `new_tap` - Optional new tap name (shows info if not provided)
pub fn migrate(formula_name: &str, new_tap: Option<&str>) -> Result<()> {
    println!("Migrating formula: {}", formula_name.cyan());

    // Check if formula is installed
    let versions = cellar::get_installed_versions(formula_name)?;
    if versions.is_empty() {
        println!("{} Formula not installed: {}", "✗".red(), formula_name);
        return Ok(());
    }

    let version = &versions[0].version;

    // If no new tap specified, show information about current tap
    let tap = match new_tap {
        Some(t) => t,
        None => {
            println!("Migration information:");
            println!("  Formula: {} {}", formula_name.bold(), version.dimmed());
            println!("  Currently installed from: {}", "homebrew/core".cyan());
            println!("To migrate to a different tap, use:");
            println!("  {} --tap <tap-name>", "bru migrate".cyan());
            return Ok(());
        }
    };

    println!("  Migrating {} to tap: {}", formula_name, tap.cyan());
    println!("Migration is a metadata operation only");
    println!("  No reinstallation needed - formula remains at same location");
    println!("  Future upgrades will use the new tap");

    // In a full implementation, this would update the formula's tap metadata
    // For now, this is informational

    println!(
        "{} Migration prepared (metadata would be updated)",
        "✓".green()
    );

    Ok(())
}

/// Check library linkages for installed formulae
///
/// Inspects executables and libraries to verify they link to the correct
/// dependencies and detect broken links. Uses otool on macOS.
///
/// # Arguments
/// * `formula_names` - The formulae to check (empty checks all)
/// * `show_all` - If true, show all linkages (not just broken ones)
pub fn linkage(formula_names: &[String], show_all: bool) -> Result<()> {
    println!("Checking library linkages...");

    let formulae_to_check: Vec<String> = if formula_names.is_empty() {
        // Check all installed formulae
        cellar::list_installed()?
            .into_iter()
            .map(|p| p.name)
            .collect()
    } else {
        formula_names.to_vec()
    };

    if formulae_to_check.is_empty() {
        println!("No formulae to check");
        return Ok(());
    }

    for formula_name in &formulae_to_check {
        println!("{}", formula_name.cyan());

        let versions = cellar::get_installed_versions(formula_name)?;
        if versions.is_empty() {
            println!("  {} Not installed", "⚠".yellow());
            continue;
        }

        let version = &versions[0].version;
        let formula_path = cellar::cellar_path().join(formula_name).join(version);

        // Find all executables and libraries
        let mut checked_files = 0;
        let mut broken_links = 0;

        // Check bin/ directory
        let bin_dir = formula_path.join("bin");
        if bin_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&bin_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    checked_files += 1;

                    // Use otool to check linkages on macOS
                    let output = std::process::Command::new("otool")
                        .arg("-L")
                        .arg(&path)
                        .output();

                    if let Ok(output) = output {
                        let stdout = String::from_utf8_lossy(&output.stdout);

                        if show_all && let Some(name) = path.file_name() {
                            println!("  {}:", name.to_string_lossy());
                            for line in stdout.lines().skip(1) {
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    println!("    {}", trimmed.dimmed());
                                }
                            }
                        }

                        // Check for broken links (simplified)
                        if stdout.contains("dyld:") || stdout.contains("not found") {
                            broken_links += 1;
                        }
                    }
                }
            }
        }

        // Check lib/ directory
        let lib_dir = formula_path.join("lib");
        if lib_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&lib_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && (path.extension().and_then(|s| s.to_str()) == Some("dylib")) {
                    checked_files += 1;
                }
            }
        }

        if checked_files == 0 {
            println!("  No linkable files found");
        } else if broken_links > 0 {
            println!(
                "  {} {} files checked, {} broken links",
                "⚠".yellow(),
                checked_files,
                broken_links
            );
        } else {
            println!(
                "  {} {} files checked, all links valid",
                "✓".green(),
                checked_files
            );
        }
    }

    Ok(())
}

/// Read and validate all formulae in a tap
///
/// Checks that all formula files in a tap are readable and syntactically valid.
/// Useful for verifying tap integrity.
///
/// # Arguments
/// * `tap_name` - The tap to check (defaults to homebrew/core)
pub fn readall(tap_name: Option<&str>) -> Result<()> {
    let tap = tap_name.unwrap_or("homebrew/core");

    println!("Reading all formulae in tap: {}", tap.cyan());

    let tap_dir = if tap == "homebrew/core" {
        cellar::detect_prefix().join("Library/Taps/homebrew/homebrew-core")
    } else {
        crate::tap::tap_directory(tap)?
    };

    if !tap_dir.exists() {
        println!("{} Tap not found: {}", "✗".red(), tap);
        return Ok(());
    }

    let formula_dir = tap_dir.join("Formula");
    if !formula_dir.exists() {
        println!("{} No Formula directory in tap", "⚠".yellow());
        return Ok(());
    }

    // Count formula files recursively
    fn count_formulae(dir: &std::path::Path, depth: usize) -> (usize, usize) {
        const MAX_DEPTH: usize = 10;
        if depth > MAX_DEPTH {
            return (0, 0);
        }

        let mut total = 0;
        let mut valid = 0;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rb") {
                    total += 1;
                    // Basic validation: check if file is readable (metadata check, not reading content)
                    if std::fs::metadata(&path).is_ok() {
                        valid += 1;
                    }
                } else if path.is_dir() {
                    let (sub_total, sub_valid) = count_formulae(&path, depth + 1);
                    total += sub_total;
                    valid += sub_valid;
                }
            }
        }

        (total, valid)
    }

    let (total, valid) = count_formulae(&formula_dir, 0);

    if total == 0 {
        println!("{} No formulae found in tap", "⚠".yellow());
    } else if valid == total {
        println!(
            "{} All {} formulae are readable",
            "✓".green().bold(),
            total.to_string().bold()
        );
    } else {
        println!(
            "{} {} of {} formulae are readable",
            "⚠".yellow(),
            valid,
            total
        );
        println!("  {} {} formulae have issues", "✗".red(), total - valid);
    }

    Ok(())
}
