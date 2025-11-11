# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.35] - 2025-11-10

### Added
- `PackageManager` high-level library API for programmatic package management
  - Single unified interface for common package operations
  - Automatic resource management (HTTP client, connection pooling)
  - Rich result types with operation details (timing, dependencies, paths)
- Core package management operations:
  - `install(name)` - Install packages from bottles with dependency resolution
  - `uninstall(name)` - Remove packages with proper symlink cleanup
  - `upgrade(name)` - Full upgrade workflow with receipt generation
  - `reinstall(name)` - Fresh installation via uninstall + install
  - `cleanup(dry_run)` - Remove old versions, preserve linked and newest
- Discovery operations:
  - `search(query)` - Find packages (returns name, description)
  - `info(name)` - Get package metadata (version, dependencies, description)
  - `list()` - List all installed packages
  - `outdated()` - Find packages needing upgrade
  - `dependencies(name)` - Resolve package dependencies (runtime and build)
  - `uses(name)` - Find packages that depend on this one
- Maintenance operations:
  - `cleanup(dry_run)` - Remove old versions with space tracking
  - `check()` - System health verification
- Library documentation with usage examples and performance characteristics

### Documentation
- Added `docs/library-api.md` with complete API reference and usage patterns
- Updated README with library usage section
- Added 5 example test programs demonstrating integration testing patterns
- Comprehensive integration test report validating production-readiness

### Verified
- ✅ Production-ready on real system state (340+ packages tested)
- ✅ Full Homebrew compatibility (INSTALL_RECEIPT.json, symlinks, binary execution)
- ✅ Error handling robustness (zero panics, proper Result types)
- ✅ Performance acceptable for interactive use
- ✅ Type safety across all operations and result types

### Performance
- Non-destructive operations: <300ms (except outdated which queries all packages)
- Install/upgrade: Fast when bottles cached (100-200ms typical)
- Cleanup: <50ms on typical systems
- API caching reduces redundant network requests

### Breaking Changes
None - Library API is purely additive on top of existing CLI

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
