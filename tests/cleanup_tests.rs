// Tests for cleanup functionality - version sorting and removal logic
use std::cmp::Ordering;

/// Mock package structure for testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MockPackage {
    name: String,
    version: String,
}

/// Version comparison logic extracted from cleanup command
/// This is the CORRECT implementation that should be used
fn compare_versions(a: &MockPackage, b: &MockPackage) -> Ordering {
    // Try to parse as semantic version numbers
    let a_parts: Vec<u32> = a
        .version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    let b_parts: Vec<u32> = b
        .version
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();

    // Compare version parts numerically
    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_part = a_parts.get(i).unwrap_or(&0);
        let b_part = b_parts.get(i).unwrap_or(&0);
        match a_part.cmp(b_part) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    // If numeric comparison fails, fall back to lexicographic
    a.version.cmp(&b.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison_basic() {
        let v1 = MockPackage {
            name: "test".to_string(),
            version: "1.8.1".to_string(),
        };
        let v2 = MockPackage {
            name: "test".to_string(),
            version: "1.7.0".to_string(),
        };

        // 1.8.1 > 1.7.0
        assert_eq!(compare_versions(&v1, &v2), Ordering::Greater);
        assert_eq!(compare_versions(&v2, &v1), Ordering::Less);
    }

    #[test]
    fn test_version_comparison_edge_case() {
        let v1 = MockPackage {
            name: "test".to_string(),
            version: "1.10.0".to_string(),
        };
        let v2 = MockPackage {
            name: "test".to_string(),
            version: "1.9.0".to_string(),
        };

        // 1.10.0 > 1.9.0 (NOT lexicographic!)
        // Lexicographically "1.9.0" > "1.10.0" would be WRONG
        assert_eq!(compare_versions(&v1, &v2), Ordering::Greater);
    }

    #[test]
    fn test_cleanup_keeps_newest_version() {
        // Simulate cleanup scenario
        let mut versions = [
            MockPackage {
                name: "jq".to_string(),
                version: "1.7.0".to_string(),
            },
            MockPackage {
                name: "jq".to_string(),
                version: "1.8.1".to_string(),
            },
            MockPackage {
                name: "jq".to_string(),
                version: "1.6.0".to_string(),
            },
        ];

        // Sort and reverse (highest first)
        versions.sort_by(compare_versions);
        versions.reverse();

        // Cleanup should keep first (newest) and remove rest (oldest)
        let latest = &versions[0];
        let to_remove = &versions[1..];

        assert_eq!(latest.version, "1.8.1", "Should keep newest version");
        assert_eq!(to_remove.len(), 2, "Should remove 2 old versions");
        assert_eq!(to_remove[0].version, "1.7.0");
        assert_eq!(to_remove[1].version, "1.6.0");
    }

    #[test]
    fn test_cleanup_with_complex_versions() {
        let mut versions = [
            MockPackage {
                name: "llvm".to_string(),
                version: "21.1.3".to_string(),
            },
            MockPackage {
                name: "llvm".to_string(),
                version: "21.1.4".to_string(),
            },
            MockPackage {
                name: "llvm".to_string(),
                version: "20.0.1".to_string(),
            },
        ];

        versions.sort_by(compare_versions);
        versions.reverse();

        assert_eq!(versions[0].version, "21.1.4");
        assert_eq!(versions[1].version, "21.1.3");
        assert_eq!(versions[2].version, "20.0.1");
    }

    #[test]
    fn test_version_with_suffixes() {
        // Versions with non-numeric suffixes
        let v1 = MockPackage {
            name: "test".to_string(),
            version: "2.0.0-beta".to_string(),
        };
        let v2 = MockPackage {
            name: "test".to_string(),
            version: "2.0.0-alpha".to_string(),
        };

        // Should fall back to lexicographic
        assert_eq!(compare_versions(&v1, &v2), Ordering::Greater);
    }

    #[test]
    fn test_equal_versions() {
        let v1 = MockPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
        };
        let v2 = MockPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
        };

        assert_eq!(compare_versions(&v1, &v2), Ordering::Equal);
    }
}
