use crate::api::BrewApi;
use crate::cellar;
use crate::error::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;

fn check_brew_available() -> bool {
    Command::new("brew")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub async fn info(api: &BrewApi, formula: &str, json: bool) -> Result<()> {
    let is_tty = std::io::IsTerminal::is_terminal(&std::io::stdout());

    let spinner = if !json && is_tty {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Fetching info for {}...", formula));
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    } else {
        ProgressBar::hidden()
    };

    if let Ok(versions) = cellar::get_installed_versions(formula)
        && let Some(installed_version) = versions.first()
        && let Ok(Some((tap_name, formula_path, _))) =
            crate::tap::get_package_tap_info(&installed_version.path)
    {
        match crate::tap::parse_formula_info(&formula_path, formula) {
            Ok(tap_info) => {
                println!(
                    "{}",
                    format!("==> {}/{}", tap_name, tap_info.name).bold().green()
                );
                if let Some(desc) = &tap_info.desc {
                    println!("{}", desc);
                }
                if let Some(homepage) = &tap_info.homepage {
                    println!("{}: {}", "Homepage".bold(), homepage);
                }
                if let Some(version) = &tap_info.version {
                    println!("{}: {}", "Version".bold(), version);
                }

                println!(
                    "{}: {} versions installed",
                    "Installed".bold(),
                    versions.len()
                );
                for v in &versions {
                    let marker = if v.version == installed_version.version {
                        "*"
                    } else {
                        ""
                    };
                    println!("  {} {}", v.version.dimmed(), marker);
                }

                println!(
                    "{}: {}",
                    "From".bold(),
                    formula_path.display().to_string().dimmed()
                );
                spinner.finish_and_clear();
                return Ok(());
            }
            Err(e) => {
                spinner.finish_and_clear();
                eprintln!(
                    "Warning: Failed to parse tap formula ({}), falling back to brew",
                    e
                );
                let full_name = format!("{}/{}", tap_name, formula);
                if check_brew_available() {
                    let _ = Command::new("brew").arg("info").arg(&full_name).status();
                }
                return Err(e.into());
            }
        }
    }

    match api.fetch_formula(formula).await {
        Ok(formula_data) => {
            spinner.finish_and_clear();
            if json {
                let json_str = serde_json::to_string_pretty(&formula_data)?;
                println!("{}", json_str);
            } else {
                println!("{}", format!("==> {}", formula_data.name).bold().green());
                if let Some(desc) = &formula_data.desc {
                    println!("{}", desc);
                }
                if let Some(homepage) = &formula_data.homepage {
                    println!("{}: {}", "Homepage".bold(), homepage);
                }
                if let Some(version) = &formula_data.versions.stable {
                    println!("{}: {}", "Version".bold(), version);
                }

                if formula_data.keg_only {
                    if let Some(reason) = &formula_data.keg_only_reason {
                        let reason_display = match reason.reason.as_str() {
                            ":provided_by_macos" => "provided by macOS",
                            ":shadowed_by_macos" => "shadowed by macOS",
                            ":versioned_formula" => "versioned formula",
                            _ => &reason.reason,
                        };
                        println!("{}: {}", "Keg-only".bold().yellow(), reason_display);
                        if !reason.explanation.is_empty() {
                            println!("  {}", reason.explanation.dimmed());
                        }
                    } else {
                        println!("{}: yes", "Keg-only".bold().yellow());
                    }
                }

                if !formula_data.dependencies.is_empty() {
                    println!(
                        "{}: {}",
                        "Dependencies".bold(),
                        formula_data.dependencies.join(", ")
                    );
                }

                if !formula_data.build_dependencies.is_empty() {
                    println!(
                        "{}: {}",
                        "Build dependencies".bold(),
                        formula_data.build_dependencies.join(", ")
                    );
                }
            }
        }
        Err(_) => match api.fetch_cask(formula).await {
            Ok(cask) => {
                spinner.finish_and_clear();
                if json {
                    let json_str = serde_json::to_string_pretty(&cask)?;
                    println!("{}", json_str);
                } else {
                    println!("{}", format!("==> {}", cask.token).bold().cyan());
                    if !cask.name.is_empty() {
                        println!("{}: {}", "Name".bold(), cask.name.join(", "));
                    }
                    if let Some(desc) = &cask.desc {
                        println!("{}", desc);
                    }
                    if let Some(homepage) = &cask.homepage {
                        println!("{}: {}", "Homepage".bold(), homepage);
                    }
                    if let Some(version) = &cask.version {
                        println!("{}: {}", "Version".bold(), version);
                    }
                }
            }
            Err(_) => {
                spinner.finish_and_clear();
                if json {
                    println!(
                        "{{\"error\": \"No formula or cask found for '{}'\"}}",
                        formula
                    );
                } else {
                    println!(
                        "\n {} No formula or cask found for '{}'",
                        "✗".red(),
                        formula
                    );
                }
            }
        },
    }

    Ok(())
}
