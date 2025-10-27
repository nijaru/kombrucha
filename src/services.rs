use crate::cellar;
use std::path::PathBuf;
use std::process::Command;

type Result<T> = anyhow::Result<T>;
type ServiceTuple = (String, Option<i32>, Option<i32>);

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub status: ServiceStatus,
    pub user: Option<String>,
    pub plist_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceStatus {
    None,
    Started,
    Error(i32),
}

/// Get LaunchAgents directory for current user
pub fn launch_agents_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join("Library/LaunchAgents")
    } else {
        PathBuf::from("~/Library/LaunchAgents")
    }
}

/// Get the plist filename for a formula
pub fn plist_filename(formula: &str) -> String {
    format!("homebrew.mxcl.{}.plist", formula)
}

/// Get the plist path for a formula
pub fn plist_path(formula: &str) -> PathBuf {
    launch_agents_dir().join(plist_filename(formula))
}

/// Get the launchd label for a formula
pub fn service_label(formula: &str) -> String {
    format!("homebrew.mxcl.{}", formula)
}

/// Check if a service plist exists
pub fn service_exists(formula: &str) -> bool {
    plist_path(formula).exists()
}

/// List all running launchd services
pub fn list_running_services() -> Result<Vec<ServiceTuple>> {
    let output = Command::new("launchctl").arg("list").output()?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut services = Vec::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let label = parts[2];
            if label.starts_with("homebrew.mxcl.") {
                let pid = if parts[0] == "-" {
                    None
                } else {
                    parts[0].parse().ok()
                };
                let exit_code = if parts[1] == "0" {
                    None
                } else {
                    parts[1].parse().ok()
                };
                services.push((label.to_string(), pid, exit_code));
            }
        }
    }

    Ok(services)
}

/// Get service status for a formula
pub fn get_service_status(formula: &str) -> Result<ServiceInfo> {
    let plist = plist_path(formula);
    let label = service_label(formula);

    let user = std::env::var("USER").ok();

    if !plist.exists() {
        return Ok(ServiceInfo {
            name: formula.to_string(),
            status: ServiceStatus::None,
            user,
            plist_path: None,
        });
    }

    // Check if service is loaded
    let running_services = list_running_services()?;
    let service_entry = running_services.iter().find(|(l, _, _)| l == &label);

    let status = match service_entry {
        Some((_, Some(_pid), _)) => ServiceStatus::Started,
        Some((_, None, Some(code))) => ServiceStatus::Error(*code),
        _ => ServiceStatus::None,
    };

    Ok(ServiceInfo {
        name: formula.to_string(),
        status,
        user,
        plist_path: Some(plist),
    })
}

/// List all services (both running and available)
pub fn list_all_services() -> Result<Vec<ServiceInfo>> {
    let mut services = Vec::new();
    let agents_dir = launch_agents_dir();

    if !agents_dir.exists() {
        return Ok(services);
    }

    // Get all homebrew plist files
    for entry in std::fs::read_dir(&agents_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|f| f.to_str())
            && filename.starts_with("homebrew.mxcl.")
            && filename.ends_with(".plist")
        {
            // Extract formula name from filename
            let formula_name = filename
                .strip_prefix("homebrew.mxcl.")
                .and_then(|s| s.strip_suffix(".plist"))
                .unwrap_or("");

            if !formula_name.is_empty() {
                let info = get_service_status(formula_name)?;
                services.push(info);
            }
        }
    }

    // Also check installed formulae that might have services
    let installed = cellar::list_installed()?;
    for pkg in installed {
        if !services.iter().any(|s| s.name == pkg.name) {
            // Check if this formula could have a service
            let info = get_service_status(&pkg.name)?;
            if info.plist_path.is_some() || service_could_exist(&pkg.name) {
                services.push(info);
            }
        }
    }

    services.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(services)
}

/// Check if a formula could potentially have a service
/// (Common service formulae)
fn service_could_exist(formula: &str) -> bool {
    const COMMON_SERVICES: &[&str] = &[
        "postgresql",
        "mysql",
        "redis",
        "mongodb",
        "nginx",
        "apache",
        "memcached",
        "rabbitmq",
        "elasticsearch",
        "dnsmasq",
        "unbound",
        "mariadb",
        "cassandra",
        "dbus",
        "atuin",
    ];

    COMMON_SERVICES.iter().any(|&s| formula.starts_with(s))
}

/// Start a service
pub fn start_service(formula: &str) -> Result<()> {
    let plist = plist_path(formula);

    if !plist.exists() {
        anyhow::bail!("Service file not found for {}", formula);
    }

    let output = Command::new("launchctl")
        .arg("load")
        .arg("-w")
        .arg(&plist)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start service: {}", stderr);
    }

    Ok(())
}

/// Stop a service
pub fn stop_service(formula: &str) -> Result<()> {
    let plist = plist_path(formula);

    if !plist.exists() {
        anyhow::bail!("Service file not found for {}", formula);
    }

    let output = Command::new("launchctl")
        .arg("unload")
        .arg("-w")
        .arg(&plist)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stop service: {}", stderr);
    }

    Ok(())
}

/// Restart a service
pub fn restart_service(formula: &str) -> Result<()> {
    stop_service(formula)?;

    // Small delay to ensure clean stop
    std::thread::sleep(std::time::Duration::from_millis(500));

    start_service(formula)?;
    Ok(())
}
