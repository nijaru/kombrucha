use crate::cellar;
use crate::error::Result;
use colored::Colorize;

pub fn shellenv(shell: Option<&str>) -> Result<()> {
    let prefix = cellar::detect_prefix();

    // Detect shell if not provided
    let shell_type = match shell {
        Some(s) => s.to_string(),
        None => {
            // Try to detect from SHELL environment variable
            std::env::var("SHELL")
                .ok()
                .and_then(|s| {
                    let path = std::path::PathBuf::from(s);
                    path.file_name()
                        .and_then(|f| f.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "bash".to_string())
        }
    };

    match shell_type.as_str() {
        "bash" | "sh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin:$PATH\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man:$MANPATH\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:$INFOPATH\";",
                prefix.display()
            );
        }
        "zsh" => {
            println!("export HOMEBREW_PREFIX=\"{}\";", prefix.display());
            println!("export HOMEBREW_CELLAR=\"{}/Cellar\";", prefix.display());
            println!("export HOMEBREW_REPOSITORY=\"{}\";", prefix.display());
            println!(
                "export PATH=\"{}/bin:{}/sbin${{PATH+:$PATH}}\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "export MANPATH=\"{}/share/man${{MANPATH+:$MANPATH}}:\";",
                prefix.display()
            );
            println!(
                "export INFOPATH=\"{}/share/info:${{INFOPATH:-}}\";",
                prefix.display()
            );
        }
        "fish" => {
            println!("set -gx HOMEBREW_PREFIX \"{}\";", prefix.display());
            println!("set -gx HOMEBREW_CELLAR \"{}/Cellar\";", prefix.display());
            println!("set -gx HOMEBREW_REPOSITORY \"{}\";", prefix.display());
            println!(
                "fish_add_path -gP \"{}/bin\" \"{}/sbin\";",
                prefix.display(),
                prefix.display()
            );
            println!(
                "set -gx MANPATH \"{}/share/man\" $MANPATH;",
                prefix.display()
            );
            println!(
                "set -gx INFOPATH \"{}/share/info\" $INFOPATH;",
                prefix.display()
            );
        }
        other => {
            println!("{} Unsupported shell: {}", "✗".red(), other);
            println!("Supported shells: bash, zsh, fish");
            return Ok(());
        }
    }

    Ok(())
}
