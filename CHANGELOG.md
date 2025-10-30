# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.18] - 2025-10-29

### Added
- Spinners for all long-running network operations (outdated, info, search, fetch, cask upgrade, deps, uses)

### Changed
- Comprehensive compact output style across all commands following modern CLI patterns (uv, cargo, gh)
- Removed leading newlines from all command output for cleaner, more professional appearance

### Fixed
- `bru upgrade` now correctly handles both tap/formula names (e.g., `nijaru/tap/bru`) and simple formula names

## [0.1.17] - 2025-10-29

### Fixed
- Release workflow asset naming now uses Darwin target triples (fixes download failures)
- Native tap formula support without Ruby runtime dependency
- Receipt parsing now handles missing `compatibility_version` and null `changed_files`
- `bru info` now displays tap formula information natively
- `bru upgrade` correctly detects and upgrades tap packages (e.g., nijaru/tap/bru)
- `bru reinstall` checks if package is from tap before uninstalling (prevents self-destruction)

## [0.1.16] - 2025-10-29

### Added
- Spinner during "Checking for outdated packages" operation for better UX
- Per-package spinner during upgrade phase (shows "Upgrading package-name...")

### Changed
- Removed unnecessary newlines for more compact output (following uv/cargo style)
- Combined redundant messages (e.g., "Checking..." + "X packages to upgrade" → "Found X outdated packages")
- Suppressed benign install_name_tool warnings about code signatures (still logs real errors)

## [0.1.15] - 2025-10-29

### Added
- Live progress display for parallel tap updates (shows results as they complete)
- 5 comprehensive parallel operations tests (tap update, upgrade, API fetch, services filtering)
- Total test count: 97 automated tests (76 unit + 21 integration)

### Changed
- Consistent, colored error formatting across all commands with --help hints
- Updated performance benchmarks with real-world measurements (M3 Max, October 2025)

### Fixed
- Services list now correctly filters cask-only plists (e.g., tailscale-app)
- Now has parity with `brew services list` behavior

## [0.1.14] - 2025-10-28

### Added
- Parallel Mach-O detection and relocation with rayon for faster bottle installation
- HashMap/Vec capacity hints to reduce allocations

### Performance
- Significant speedup in bottle relocation phase
- Reduced memory allocations during package operations

## Earlier Versions

See [GitHub Releases](https://github.com/nijaru/kombrucha/releases) for older release notes.
