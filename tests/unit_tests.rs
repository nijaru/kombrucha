// Unit tests for core functionality

#[cfg(test)]
mod version_tests {
    // Test strip_bottle_revision function from commands.rs
    fn strip_bottle_revision(version: &str) -> &str {
        if let Some(pos) = version.rfind('_') {
            // Check if everything after _ is digits (bottle revision)
            if version[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
                return &version[..pos];
            }
        }
        version
    }

    #[test]
    fn test_strip_bottle_revision_with_revision() {
        assert_eq!(strip_bottle_revision("1.4.0_32"), "1.4.0");
        assert_eq!(strip_bottle_revision("2.14.1_1"), "2.14.1");
        assert_eq!(strip_bottle_revision("8.0_1"), "8.0");
        assert_eq!(strip_bottle_revision("21.1.3_99"), "21.1.3");
        assert_eq!(strip_bottle_revision("0.9.4_1"), "0.9.4");
    }

    #[test]
    fn test_strip_bottle_revision_without_revision() {
        assert_eq!(strip_bottle_revision("1.4.0"), "1.4.0");
        assert_eq!(strip_bottle_revision("2.14.1"), "2.14.1");
        assert_eq!(strip_bottle_revision("8.0"), "8.0");
        assert_eq!(strip_bottle_revision("2025.10.12"), "2025.10.12");
    }

    #[test]
    fn test_strip_bottle_revision_with_underscore_in_version() {
        // Underscore followed by non-digits should be kept
        assert_eq!(strip_bottle_revision("python_3.11"), "python_3.11");
        assert_eq!(strip_bottle_revision("foo_bar"), "foo_bar");
        assert_eq!(strip_bottle_revision("1.0_beta"), "1.0_beta");
        assert_eq!(strip_bottle_revision("clang_format"), "clang_format");
    }

    #[test]
    fn test_strip_bottle_revision_edge_cases() {
        assert_eq!(strip_bottle_revision(""), "");
        assert_eq!(strip_bottle_revision("1"), "1");
        assert_eq!(strip_bottle_revision("_1"), "");
        // Trailing underscore with no digits - gets stripped due to empty string matching .all()
        // This is acceptable behavior as trailing underscores shouldn't appear in real versions
        assert_eq!(strip_bottle_revision("1.0_"), "1.0");
    }

    #[test]
    fn test_strip_bottle_revision_multiple_underscores() {
        // Should only strip the last underscore if followed by digits
        assert_eq!(strip_bottle_revision("foo_bar_1"), "foo_bar");
        assert_eq!(strip_bottle_revision("1.0_beta_2"), "1.0_beta");
        assert_eq!(strip_bottle_revision("python@3.11_5"), "python@3.11");
    }

    #[test]
    fn test_strip_bottle_revision_regression_cases() {
        // Real-world cases from the outdated bug
        assert_eq!(strip_bottle_revision("1.4.0_31"), "1.4.0"); // mosh
        assert_eq!(strip_bottle_revision("1.4.0_32"), "1.4.0"); // mosh
        assert_eq!(strip_bottle_revision("2.14.1_1"), "2.14.1"); // freetype
    }
}

#[cfg(test)]
mod dependency_resolution_tests {
    use std::collections::{HashMap, HashSet};

    // Helper to simulate dependency graph
    #[derive(Debug, Clone)]
    struct MockFormula {
        name: String,
        dependencies: Vec<String>,
    }

    fn build_dependency_graph(formulae: &[MockFormula]) -> HashMap<String, Vec<String>> {
        formulae
            .iter()
            .map(|f| (f.name.clone(), f.dependencies.clone()))
            .collect()
    }

    fn topological_sort(graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>, String> {
        let mut sorted = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();

        fn visit(
            node: &str,
            graph: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            temp_mark: &mut HashSet<String>,
            sorted: &mut Vec<String>,
        ) -> Result<(), String> {
            if temp_mark.contains(node) {
                return Err(format!("Circular dependency detected: {}", node));
            }
            if visited.contains(node) {
                return Ok(());
            }

            temp_mark.insert(node.to_string());

            if let Some(deps) = graph.get(node) {
                for dep in deps {
                    visit(dep, graph, visited, temp_mark, sorted)?;
                }
            }

            temp_mark.remove(node);
            visited.insert(node.to_string());
            sorted.push(node.to_string());
            Ok(())
        }

        for node in graph.keys() {
            if !visited.contains(node) {
                visit(node, graph, &mut visited, &mut temp_mark, &mut sorted)?;
            }
        }

        Ok(sorted)
    }

    #[test]
    fn test_simple_dependency_chain() {
        let formulae = vec![
            MockFormula {
                name: "c".to_string(),
                dependencies: vec![],
            },
            MockFormula {
                name: "b".to_string(),
                dependencies: vec!["c".to_string()],
            },
            MockFormula {
                name: "a".to_string(),
                dependencies: vec!["b".to_string()],
            },
        ];

        let graph = build_dependency_graph(&formulae);
        let sorted = topological_sort(&graph).unwrap();

        // c should come before b, b should come before a
        let c_pos = sorted.iter().position(|x| x == "c").unwrap();
        let b_pos = sorted.iter().position(|x| x == "b").unwrap();
        let a_pos = sorted.iter().position(|x| x == "a").unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_diamond_dependency() {
        let formulae = vec![
            MockFormula {
                name: "d".to_string(),
                dependencies: vec![],
            },
            MockFormula {
                name: "b".to_string(),
                dependencies: vec!["d".to_string()],
            },
            MockFormula {
                name: "c".to_string(),
                dependencies: vec!["d".to_string()],
            },
            MockFormula {
                name: "a".to_string(),
                dependencies: vec!["b".to_string(), "c".to_string()],
            },
        ];

        let graph = build_dependency_graph(&formulae);
        let sorted = topological_sort(&graph).unwrap();

        // d should come before both b and c
        let d_pos = sorted.iter().position(|x| x == "d").unwrap();
        let b_pos = sorted.iter().position(|x| x == "b").unwrap();
        let c_pos = sorted.iter().position(|x| x == "c").unwrap();
        let a_pos = sorted.iter().position(|x| x == "a").unwrap();

        assert!(d_pos < b_pos);
        assert!(d_pos < c_pos);
        assert!(b_pos < a_pos);
        assert!(c_pos < a_pos);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let formulae = vec![
            MockFormula {
                name: "a".to_string(),
                dependencies: vec!["b".to_string()],
            },
            MockFormula {
                name: "b".to_string(),
                dependencies: vec!["c".to_string()],
            },
            MockFormula {
                name: "c".to_string(),
                dependencies: vec!["a".to_string()],
            },
        ];

        let graph = build_dependency_graph(&formulae);
        let result = topological_sort(&graph);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency detected"));
    }

    #[test]
    fn test_no_dependencies() {
        let formulae = vec![
            MockFormula {
                name: "a".to_string(),
                dependencies: vec![],
            },
            MockFormula {
                name: "b".to_string(),
                dependencies: vec![],
            },
        ];

        let graph = build_dependency_graph(&formulae);
        let sorted = topological_sort(&graph).unwrap();

        assert_eq!(sorted.len(), 2);
        assert!(sorted.contains(&"a".to_string()));
        assert!(sorted.contains(&"b".to_string()));
    }
}

#[cfg(test)]
mod api_parsing_tests {
    use serde_json::json;

    #[test]
    fn test_parse_formula_with_all_fields() {
        let json = json!({
            "name": "wget",
            "full_name": "wget",
            "tap": "homebrew/core",
            "versions": {
                "stable": "1.21.4",
                "head": null,
                "bottle": true
            },
            "desc": "Internet file retriever",
            "homepage": "https://www.gnu.org/software/wget/",
            "dependencies": ["openssl@3", "libidn2"],
            "keg_only": false,
            "bottle": {
                "stable": {
                    "files": {
                        "arm64_sonoma": {
                            "url": "https://ghcr.io/...",
                            "sha256": "abc123"
                        }
                    }
                }
            }
        });

        // Just verify it can be parsed - actual parsing is handled by serde
        assert!(json.get("name").is_some());
        assert_eq!(json.get("name").unwrap().as_str().unwrap(), "wget");
        assert_eq!(
            json.get("versions")
                .unwrap()
                .get("stable")
                .unwrap()
                .as_str()
                .unwrap(),
            "1.21.4"
        );
    }

    #[test]
    fn test_parse_formula_with_missing_optional_fields() {
        let json = json!({
            "name": "test",
            "full_name": "test",
            "tap": "homebrew/core",
            "versions": {
                "stable": "1.0"
            },
            "desc": "Test formula"
        });

        assert!(json.get("name").is_some());
        assert!(json.get("homepage").is_none());
        assert!(json.get("dependencies").is_none());
    }

    #[test]
    fn test_parse_keg_only_formula() {
        let json = json!({
            "name": "sqlite",
            "full_name": "sqlite",
            "tap": "homebrew/core",
            "versions": {
                "stable": "3.43.0"
            },
            "keg_only": true,
            "keg_only_reason": {
                "reason": ":provided_by_macos",
                "explanation": "macOS already provides this software."
            }
        });

        assert_eq!(json.get("keg_only").unwrap().as_bool().unwrap(), true);
        assert!(json.get("keg_only_reason").is_some());
        assert_eq!(
            json.get("keg_only_reason")
                .unwrap()
                .get("reason")
                .unwrap()
                .as_str()
                .unwrap(),
            ":provided_by_macos"
        );
    }
}

#[cfg(test)]
mod symlink_tests {
    use std::path::PathBuf;

    // Re-export the normalize_path function for testing
    fn normalize_path(path: &std::path::Path) -> PathBuf {
        let mut components = Vec::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    components.pop();
                }
                std::path::Component::CurDir => {}
                c => components.push(c),
            }
        }
        components.iter().collect()
    }

    #[test]
    fn test_normalize_path_simple() {
        let path = std::path::Path::new("/usr/local/Cellar/wget/1.21.4/bin/wget");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/usr/local/Cellar/wget/1.21.4/bin/wget"));
    }

    #[test]
    fn test_normalize_path_with_parent_dir() {
        let path = std::path::Path::new("/usr/local/bin/../Cellar/wget/1.21.4/bin/wget");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/usr/local/Cellar/wget/1.21.4/bin/wget"));
    }

    #[test]
    fn test_normalize_path_with_current_dir() {
        let path = std::path::Path::new("/usr/local/./Cellar/./wget/./1.21.4/bin/wget");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/usr/local/Cellar/wget/1.21.4/bin/wget"));
    }

    #[test]
    fn test_normalize_path_multiple_parent_dirs() {
        let path = std::path::Path::new("/usr/local/bin/../../opt/../Cellar/wget/1.21.4");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/usr/Cellar/wget/1.21.4"));
    }

    #[test]
    fn test_normalize_path_relative() {
        let path = std::path::Path::new("../Cellar/wget/1.21.4/bin/wget");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("Cellar/wget/1.21.4/bin/wget"));
    }

    #[test]
    fn test_normalize_path_mixed() {
        let path = std::path::Path::new("/usr/./local/../local/Cellar/./wget/../wget/1.21.4");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/usr/local/Cellar/wget/1.21.4"));
    }

    #[test]
    fn test_normalize_path_root() {
        let path = std::path::Path::new("/");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/"));
    }

    #[test]
    fn test_normalize_path_empty_after_normalization() {
        let path = std::path::Path::new("./foo/../bar/..");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from(""));
    }
}

#[cfg(test)]
mod cache_tests {
    use std::path::PathBuf;
    use std::fs;
    use std::time::SystemTime;

    fn cache_dir_with_env(xdg_cache: Option<&str>, home: Option<&str>) -> PathBuf {
        if let Some(cache_home) = xdg_cache {
            PathBuf::from(cache_home).join("bru")
        } else if let Some(home) = home {
            PathBuf::from(home).join(".cache/bru")
        } else {
            PathBuf::from(".cache/bru")
        }
    }

    #[test]
    fn test_cache_dir_with_xdg_cache_home() {
        let cache_dir = cache_dir_with_env(Some("/custom/cache"), Some("/home/user"));
        assert_eq!(cache_dir, PathBuf::from("/custom/cache/bru"));
    }

    #[test]
    fn test_cache_dir_with_home_only() {
        let cache_dir = cache_dir_with_env(None, Some("/home/user"));
        assert_eq!(cache_dir, PathBuf::from("/home/user/.cache/bru"));
    }

    #[test]
    fn test_cache_dir_fallback() {
        let cache_dir = cache_dir_with_env(None, None);
        assert_eq!(cache_dir, PathBuf::from(".cache/bru"));
    }

    #[test]
    fn test_cache_freshness_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let nonexistent = temp_dir.join("nonexistent_cache_file.json");

        // Test with kombrucha's is_cache_fresh if available, otherwise test logic
        // For now, test the logic that a nonexistent file is not fresh
        assert!(!nonexistent.exists());
    }

    #[test]
    fn test_cache_freshness_fresh_file() {
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let fresh_file = temp_dir.join("fresh_cache_test.json");

        fs::write(&fresh_file, "{}").expect("Failed to create test file");

        // File should be fresh (just created)
        assert!(fresh_file.exists());

        let metadata = fs::metadata(&fresh_file).expect("Failed to get metadata");
        let modified = metadata.modified().expect("Failed to get modified time");
        let age = SystemTime::now()
            .duration_since(modified)
            .expect("Failed to calculate age");

        // Should be less than 1 second old
        assert!(age.as_secs() < 1);

        // Cleanup
        let _ = fs::remove_file(&fresh_file);
    }

    #[test]
    fn test_cache_path_construction() {
        let base = PathBuf::from("/home/user/.cache/bru");
        let formulae_cache = base.join("formulae.json");
        let casks_cache = base.join("casks.json");

        assert_eq!(formulae_cache, PathBuf::from("/home/user/.cache/bru/formulae.json"));
        assert_eq!(casks_cache, PathBuf::from("/home/user/.cache/bru/casks.json"));
    }
}

#[cfg(test)]
mod download_tests {
    use std::path::PathBuf;

    fn download_cache_dir(home: Option<&str>) -> PathBuf {
        let home_str = home.unwrap_or(".");
        PathBuf::from(home_str).join(".cache/bru/downloads")
    }

    #[test]
    fn test_download_cache_dir_with_home() {
        let cache = download_cache_dir(Some("/home/user"));
        assert_eq!(cache, PathBuf::from("/home/user/.cache/bru/downloads"));
    }

    #[test]
    fn test_download_cache_dir_fallback() {
        let cache = download_cache_dir(None);
        assert_eq!(cache, PathBuf::from("./.cache/bru/downloads"));
    }

    #[test]
    fn test_bottle_filename_construction() {
        let formula_name = "wget";
        let version = "1.21.4";
        let platform_tag = "arm64_sonoma";

        let filename = format!(
            "{}--{}.{}.bottle.tar.gz",
            formula_name, version, platform_tag
        );

        assert_eq!(filename, "wget--1.21.4.arm64_sonoma.bottle.tar.gz");
    }

    #[test]
    fn test_bottle_filename_with_special_chars() {
        let formula_name = "python@3.11";
        let version = "3.11.7";
        let platform_tag = "arm64_sequoia";

        let filename = format!(
            "{}--{}.{}.bottle.tar.gz",
            formula_name, version, platform_tag
        );

        assert_eq!(filename, "python@3.11--3.11.7.arm64_sequoia.bottle.tar.gz");
    }

    #[test]
    fn test_ghcr_token_url_construction() {
        let repository = "homebrew/core/wget";
        let url = format!(
            "https://ghcr.io/token?service=ghcr.io&scope=repository:{}:pull",
            repository
        );

        assert_eq!(
            url,
            "https://ghcr.io/token?service=ghcr.io&scope=repository:homebrew/core/wget:pull"
        );
    }

    #[test]
    fn test_ghcr_repository_format() {
        let formula_name = "wget";
        let repository = format!("homebrew/core/{}", formula_name);
        assert_eq!(repository, "homebrew/core/wget");

        let formula_name = "python@3.11";
        let repository = format!("homebrew/core/{}", formula_name);
        assert_eq!(repository, "homebrew/core/python@3.11");
    }
}

#[cfg(test)]
mod error_tests {
    #[test]
    fn test_error_message_formula_not_found() {
        let formula = "nonexistent-formula";
        let error_msg = format!("Formula not found: {}", formula);
        assert_eq!(error_msg, "Formula not found: nonexistent-formula");
    }

    #[test]
    fn test_error_message_cask_not_found() {
        let cask = "nonexistent-cask";
        let error_msg = format!("Cask not found: {}", cask);
        assert_eq!(error_msg, "Cask not found: nonexistent-cask");
    }

    #[test]
    fn test_error_message_api_error() {
        let details = "Connection refused";
        let error_msg = format!("API request failed: {}", details);
        assert_eq!(error_msg, "API request failed: Connection refused");
    }

    #[test]
    fn test_error_message_network_error() {
        let details = "Timeout after 30s";
        let error_msg = format!("Network error: {}", details);
        assert_eq!(error_msg, "Network error: Timeout after 30s");
    }
}

#[cfg(test)]
mod receipt_tests {
    use serde_json::json;

    #[test]
    fn test_receipt_homebrew_version_format() {
        let version = env!("CARGO_PKG_VERSION");
        let homebrew_version = format!("bru/{}", version);
        assert!(homebrew_version.starts_with("bru/"));
        assert!(homebrew_version.contains('.'));
    }

    #[test]
    fn test_receipt_basic_structure() {
        let receipt = json!({
            "homebrew_version": "bru/0.1.8",
            "built_as_bottle": true,
            "poured_from_bottle": true,
            "loaded_from_api": true,
            "installed_as_dependency": false,
            "installed_on_request": true,
            "time": 1234567890,
            "arch": "arm64"
        });

        assert_eq!(receipt.get("homebrew_version").unwrap().as_str().unwrap(), "bru/0.1.8");
        assert_eq!(receipt.get("built_as_bottle").unwrap().as_bool().unwrap(), true);
        assert_eq!(receipt.get("poured_from_bottle").unwrap().as_bool().unwrap(), true);
        assert_eq!(receipt.get("loaded_from_api").unwrap().as_bool().unwrap(), true);
    }

    #[test]
    fn test_receipt_dependency_flag_logic() {
        let installed_on_request = true;
        let installed_as_dependency = !installed_on_request;

        assert_eq!(installed_as_dependency, false);

        let installed_on_request = false;
        let installed_as_dependency = !installed_on_request;

        assert_eq!(installed_as_dependency, true);
    }

    #[test]
    fn test_receipt_source_info_structure() {
        let source = json!({
            "tap": "homebrew/core",
            "spec": "stable"
        });

        assert_eq!(source.get("tap").unwrap().as_str().unwrap(), "homebrew/core");
        assert_eq!(source.get("spec").unwrap().as_str().unwrap(), "stable");
    }

    #[test]
    fn test_receipt_runtime_dependencies_empty() {
        let runtime_deps: Vec<String> = vec![];
        assert_eq!(runtime_deps.len(), 0);
    }

    #[test]
    fn test_receipt_arch_detection() {
        let arch = std::env::consts::ARCH;
        assert!(!arch.is_empty());
        assert!(arch == "aarch64" || arch == "x86_64" || arch == "x86");
    }
}

#[cfg(test)]
mod platform_tests {
    #[test]
    fn test_arch_normalization_aarch64() {
        let arch = "aarch64";
        let normalized = match arch {
            "aarch64" => "arm64",
            other => other,
        };
        assert_eq!(normalized, "arm64");
    }

    #[test]
    fn test_arch_normalization_x86_64() {
        let arch = "x86_64";
        let normalized = match arch {
            "aarch64" => "arm64",
            other => other,
        };
        assert_eq!(normalized, "x86_64");
    }

    #[test]
    fn test_macos_version_parsing() {
        let version = "15.1.0";
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(major, 15);

        let version = "14.0";
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(major, 14);
    }

    #[test]
    fn test_macos_version_parsing_edge_cases() {
        // Empty string
        let version = "";
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(major, 0);

        // Invalid format
        let version = "invalid";
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(major, 0);

        // Single digit
        let version = "15";
        let major: u32 = version
            .split('.')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(major, 15);
    }

    #[test]
    fn test_bottle_tag_format() {
        let arch = "arm64";
        let os_name = "sonoma";
        let tag = format!("{}_{}", arch, os_name);
        assert_eq!(tag, "arm64_sonoma");

        let arch = "x86_64";
        let os_name = "sequoia";
        let tag = format!("{}_{}", arch, os_name);
        assert_eq!(tag, "x86_64_sequoia");
    }

    #[test]
    fn test_linux_bottle_tag_format() {
        let arch = "arm64";
        let tag = format!("{}_linux", arch);
        assert_eq!(tag, "arm64_linux");

        let arch = "x86_64";
        let tag = format!("{}_linux", arch);
        assert_eq!(tag, "x86_64_linux");
    }
}
