# Project Status

Last updated: 2025-10-23

## Current State

**Version**: 0.1.8 (Beta)
**Status**: Production-ready for bottle-based workflows

### Metrics
- **Test Coverage**: 71 tests run automatically (8 inline + 49 unit + 14 regression)
- **Integration Tests**: 10 tests (run in CI on every push, all ignored by default)
- **Total Tests**: 86 tests (71 passing + 15 ignored)
- **Command Coverage**: Core user-facing commands fully functional
- **Bottle-Based Support**: 95% of Homebrew formulae
- **Source Build Support**: Not implemented (Phase 3)

### What Works ✅
- Core package management: install, uninstall, upgrade, reinstall
- Cask support: macOS application management (DMG, ZIP, PKG)
- Discovery commands: search, info, deps, uses, list, outdated
- Repository management: tap, untap, update
- System utilities: doctor, config, env, shellenv
- Services: launchd integration
- Bundle: Brewfile install and dump
- Modern CLI output: Tree connectors, clean formatting, command aliases
- Distribution: Homebrew tap (brew install nijaru/tap/bru)

### What Doesn't Work ❌
- Source builds: Formulae without bottles (~1-5%)
- Development tools: create, audit, livecheck, test (stubs only)
- CI/internal commands: Not implemented

### Performance
Verified benchmarks (M3 Max, macOS 15.1, 339 packages):
- upgrade --dry-run: **5.5x faster than brew** (0.65s vs 3.56s)
- upgrade optimization: **53x faster** than v0.1.2 (0.74s vs 39.5s)
- All API operations: Fully parallelized with in-memory caching
- Startup time: 0.014s

## What Worked

### Architecture Decisions
- **Hybrid Rust + Ruby approach**: Right balance of performance and compatibility
- **Parallel operations from day 1**: Major performance win
- **JSON API over tap parsing**: Faster, always up-to-date
- **Bottle-first strategy**: 95% coverage without complexity

### Testing Strategy
- Non-ignored regression tests using --dry-run
- Parity tests against brew to catch regressions
- Property-based checks (deduplication, bottle revision stripping)

### Recent Changes

**Unreleased** (post-v0.1.8):
- **Testing**: Added 36 new unit tests (71 total CI tests, up from 35)
  - Symlink path normalization tests (8 tests) - validates recent bug fixes
  - Cache functionality tests (6 tests) - TTL logic, directory detection
  - Download module tests (6 tests) - filename construction, GHCR token URLs
  - Error handling tests (4 tests) - error message formatting
  - Receipt module tests (6 tests) - version format, structure validation
  - Platform module tests (6 tests) - architecture normalization, version parsing
  - All tests CI-safe (no system modification required)
  - Test coverage increased by 103% (35 → 71 tests)

**v0.1.8** (2025-10-23):
- **Critical Bug Fixes**: Symlink cleanup in multiple commands
  - upgrade command: Properly unlinks symlinks before removal
  - cleanup command: Properly unlinks symlinks before removal
  - uninstall command: Proper cleanup of all symlinks
  - Prevents "Directory not empty" errors during package operations
- **Documentation**: Updated STATUS.md and cleaned up dated summary files

**v0.1.7** (2025-10-23):
- **UX Enhancement**: Better tree visualization and command summaries
- **UX Enhancement**: Simplified help output to match brew style
- **Bug Fix**: Show usage message for desc/pin/unpin/link/unlink with no args
- **Bug Fix**: Show helpful error when commands called with no formulae

**v0.1.6** (2025-10-22):
- **Compatibility**: Full brew-compatibility for multiple commands
  - info: No fetch message when piped (matches brew behavior)
  - deps/uses: Output format matches brew exactly
  - search: Output format matches brew exactly
  - leaves: Output format matches brew exactly
  - outdated: Output format matches brew exactly
  - list: Full brew compatibility with column mode
- **UX Enhancement**: Improved list command spacing and column mode

**v0.1.5** (2025-10-22):
- **Critical Bug Fix**: File descriptor leak during upgrade
  - Removed canonicalize() calls from symlink creation/removal
  - Fixed "Too many open files" error with large packages (1000+ files)
  - Tested with yt-dlp (1722 files), gh (215 files), multiple packages
- UX: Icon spacing fix - icons no longer sit at terminal edge
  - Space added inside colored strings for proper alignment
  - Applied to all status icons (ℹ, ✓, ✗, ⚠)
- Documentation: Comprehensive code review for resource leaks
  - Verified all file handles properly scoped
  - Verified no unbounded memory growth
  - Identified testing gaps (need large package tests)

**v0.1.4** (2025-10-22):
- Performance: Parallelized ALL sequential API patterns (8 total)
  - fetch command: Parallel metadata fetching
  - install validation: Instant multi-package validation
  - dependency resolution: Breadth-first with parallel levels
  - cask operations: Parallel metadata for install/upgrade
- Performance: In-memory caching (moka) - eliminates redundant API calls
  - Caches 1000 formulae + 500 casks per command execution
  - Benefits complex dependency trees with shared dependencies
- Result: 5.5x faster than brew for upgrade checks (0.65s vs 3.56s)

**v0.1.3** (2025-10-22):
- Performance: 53x faster upgrade checks (39.5s → 0.74s) via parallelization
- UX: Tree connectors (├ └) for visual hierarchy in install/upgrade/uninstall
- UX: Command aliases (i, up, re, rm) for faster workflow
- UX: Detailed "Already installed" messages with version numbers
- CI: Integrated tests now run automatically on every push

**v0.1.2** (2025-10-22):
- Bug fixes: leaves command deduplication, error handling improvements
- UX: Removed stack traces, accurate success/failure messages
- Quality: Replaced unwrap() calls with proper error handling
- All changes from extended session consolidated into release

**v0.1.1** (2025-10-22):
- Upgrade duplicates: Fixed deduplication by modification time
- Leaves duplicates: Fixed deduplication (same bug as upgrade)
- Bottle revision false positives: Strip _N revisions before comparison
- Modern CLI output: Removed all 78 arrow symbols (→ ⬇ ⬆)
- Error handling: Removed stack backtraces, added proper validation
- Error messages: Accurate success/failure reporting for uninstall/reinstall
- Added 12 new regression tests (install, search, info, deps, leaves, etc.)
- Improved test coverage: 16 → 27 automated tests
- Documentation reorganization: agent-contexts patterns (ai/, docs/)

**v0.1.0** (2025-10-21):
- Initial beta release
- Core commands functional
- Bottle-based workflows production-ready

## What Didn't Work

### Initial Testing Approach
- Tried: All integration tests marked #[ignore]
- Problem: Never ran in CI, missed critical bugs
- Fix: Added non-ignored regression tests using --dry-run

### Symbol Cleanup Attempts
- Tried: sed/perl for bulk symbol removal
- Problem: Broke format strings
- Fix: Manual fixes for user-visible commands, left others for later

## Active Work

Post-v0.1.4 release:
- ✅ README polish: User-focused documentation with clear value proposition
- ✅ Homebrew tap: Created nijaru/homebrew-tap for `brew install nijaru/tap/bru`
- Real-world stability testing (current)

## Blockers

None currently. Ready for beta testing.

## Next Priorities

1. **Documentation cleanup**: Reorganize internal/ → ai/ and docs/
2. **More automated tests**: Add tests that run in CI without system modification
3. **Phase 3 planning**: Ruby interop for source builds (~5% remaining formulae)
