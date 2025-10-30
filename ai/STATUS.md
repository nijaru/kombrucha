# Project Status

Last updated: 2025-10-30

## Current State

**Version**: 0.1.18+ (Development - preparing v0.1.19)
**Status**: Production-ready with comprehensive UX and testing improvements

### Recent Improvements (unreleased)

**Critical Bug Fixes:**
- Script & library relocation: Fixed broken shebangs and Python crashes (src/relocate.rs)
  - **Script Issue**: @@HOMEBREW_CELLAR@@ placeholders not replaced in executable scripts
    - Root cause: relocate_bottle() only processed Mach-O binaries, ignored scripts
    - Impact: Scripts like `#!/@@HOMEBREW_CELLAR@@/package/version/bin/python` would fail with "bad interpreter"
    - Fix: Added find_scripts_with_placeholders() and relocate_script_shebang()
    - Only processes executable files in bin/ directories with placeholder shebangs
    - Example: huggingface-cli's `hf` command now works correctly
  - **Code Signature Issue**: Python shared libraries crashed with SIGKILL (Code Signature Invalid)
    - Root cause: install_name_tool invalidates code signatures when relocating Mach-O files
    - Impact: Python imports would crash with "EXC_BAD_ACCESS (SIGKILL (Code Signature Invalid))"
    - Fix: Added `codesign --remove-signature` after all install_name_tool operations
    - Applies to both relocate_file() and fix_library_id()
  - User reported: bru-installed packages had broken scripts and Python crash popups

### Previous Improvements (fadded4)

**Performance & UX Polish:**
- Live progress display for parallel tap updates (361c845)
- Improved error messages with consistent, colored formatting (cd3f51f)
- Better visual feedback for long operations

**Testing & Quality:**
- Added comprehensive parallel operations test suite (a61a24f)
- 5 new integration tests for parallel correctness
- Services filtering correctness validation
- Performance regression prevention

**Documentation:**
- Updated README with real performance benchmarks (fadded4)
- Documented 2-24x speedup across commands
- Added modern CLI pattern documentation

### Performance Benchmarks (M3 Max, October 2025)
- `bru outdated`: 2.1x faster (780ms vs 1.63s)
- `bru info`: 9.6x faster (107ms vs 1.04s)
- `bru search`: 24x faster (43ms vs 1.04s)
- `bru update`: 5.7x faster (1.9s vs ~11s sequential)
- `bru upgrade`: 3-8x faster (parallel downloads)

### Metrics
- **Test Coverage**: 97 automated tests (76 unit + 21 integration)
- **Integration Tests**: 16 tests (5 parallel operations, 11 regression)
- **Command Coverage**: Core user-facing commands fully functional
- **Bottle-Based Support**: 95% of Homebrew formulae (native)
- **Source Build Support**: 100% via automatic brew fallback

### What Works ✅
- Core package management: install, uninstall, upgrade, reinstall
- **Source builds**: Automatic brew fallback for formulae without bottles
- Cask support: macOS application management (DMG, ZIP, PKG)
- Discovery commands: search, info, deps, uses, list, outdated
- Repository management: tap, untap, update
- System utilities: doctor, config, env, shellenv
- Services: launchd integration
- Bundle: Brewfile install and dump
- Modern CLI output: Tree connectors, clean formatting, command aliases
- Distribution: Homebrew tap (brew install nijaru/tap/bru)

### What Doesn't Work ❌
- Development tools: create, audit, livecheck, test (stubs only)
- CI/internal commands: Not implemented

### Performance
Verified benchmarks (M3 Max, macOS 15.7, 338 packages, October 2025):
- upgrade --dry-run: **1.85x faster than brew** (0.92s vs 1.71s average over 3 runs)
  - bru times: 1.23s, 0.86s, 0.66s (first run slower due to cold cache)
  - brew times: 2.28s, 1.43s, 1.41s
  - Best case: 0.66s vs 1.41s (2.1x faster)
- upgrade optimization: **53x faster** than v0.1.2 (0.74s vs 39.5s)
- All API operations: Fully parallelized with in-memory caching
- Startup time: <0.01s (measured, claimed 0.014s)

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

**v0.1.14** (2025-10-28):
- **Major Performance Optimizations**: Comprehensive parallelization and efficiency improvements [7aa494f, 96f470d, ceb35e0, 3a7a40d, c208f48]
  - **Shell Hang Fix**: Explicit tokio runtime shutdown with 1s timeout prevents shell hang after command completion
  - **Parallel Tap Updates**: 5.7x speedup (10.9s → 1.9s) via parallel git pulls, now matches brew performance
  - **Parallel Upgrade Downloads**: 3-8x speedup for multi-package upgrades via parallel bottle downloads
  - **HashMap/Vec Capacity Hints**: 5-15% improvement for large dependency graphs via pre-allocation
  - **Parallel Mach-O Operations**: 2-4x faster detection, 3-5x faster relocation using rayon
  - **New Dependency**: Added rayon for parallel iteration
  - **Real-world impact**: `bru update` now ~2x faster than before, upgrade scales linearly with packages

**v0.1.11** (2025-10-26):
- **Brew Fallback for Source Builds**: Automatic fallback to brew for formulae without bottles [476cedc]
  - **Feature**: install, upgrade, reinstall now automatically delegate to brew when no bottle available
  - **User Experience**: Clear messaging: "requires building from source (no bottle available)"
  - **Implementation**: Added check_brew_available() and fallback_to_brew() helpers
  - **Rationale**: Long-term solution (same Cellar, compatible receipts, zero maintenance)
  - **Coverage**: 100% of Homebrew formulae (95% native bottles + 5% brew fallback)
  - **Design**: Full analysis in ai/FALLBACK_DESIGN.md
  - Commands updated: install (line ~1115), upgrade (line ~1469), reinstall (line ~1606)
- **CI Improvements**: Fixed Homebrew integrity check to avoid false positives [c1af9e1]
  - Updated check to verify Cellar integrity instead of brew doctor
  - Pre-existing runner warnings no longer fail builds
- **Distribution**: Released to crates.io, GitHub, and Homebrew tap

**v0.1.13** (2025-10-27):
- **Critical Bug Fix**: Resolved "Too many open files" error in parallel downloads [df09a0b]
  - Root cause: Each download created new reqwest::Client, exhausting file descriptors
  - Fix: Create shared HTTP client once per operation, pass by reference
  - Reduced MAX_CONCURRENT_DOWNLOADS from 16 to 8 for more conservative resource usage
  - Client connection pooling now works correctly across all downloads
  - Tested: Successfully upgraded 3 packages without errors

**v0.1.12** (2025-10-27):
- **Quick Wins**: UX improvements and code quality [50b4f65, c786cac, fabf8ba]
  - Added --no-color flag for easier color disabling (works like NO_COLOR env var)
  - Resolved all 36 clippy warnings (collapsible_if, type_complexity, if_same_then_else)
  - Added 8 edge case tests validating unwrap fixes from f071069
  - Improved git error message in doctor command with install guidance
  - Better let-chain formatting for consistency
  - All 76 unit tests passing, CI green
- **Robustness Improvements**: Eliminated all edge-case unwrap() calls [f071069]
  - Fixed 9 problematic unwraps (path handling, empty strings, option unwrapping)
  - Better error messages for invalid UTF-8 paths, empty formula names
  - No more panics on edge-case inputs
  - Reduced unwraps from 20 to 10 (all remaining are safe)
- **UX Polish**: Final refinements to progress indicators and symbols [5736f97]
  - Removed arrow symbols (⬇ →) from output for cleaner appearance
  - Improved progress bar with Unicode blocks (━━╸ instead of #>-)
  - Simplified download messages: "Downloading wget" instead of "⬇ wget"
  - Changed relationship indicators to ASCII (->) for better compatibility
- **Quiet Mode**: Added --quiet/-q flag and BRU_QUIET env var [17f41aa]
  - Suppresses progress bars and spinners
  - ProgressBar::hidden() for quiet mode
  - Respects user preference for minimal output
- **Progress Indicators**: Enhanced UX for long-running operations [cf7067d]
  - Dependency resolution spinner with status messages
  - Install counter showing progress (Installing 3/10...)
  - Modern spinner characters (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
- **Code Quality**: Comprehensive audit and improvements [b2d8c94]
  - Added profiling support (debug = true in release profile)
  - Created flamegraph.svg for performance analysis
  - Documented UX best practices in ai/UX_AUDIT.md
  - Comprehensive code review in ai/CODE_REVIEW.md
  - Performance profile analysis in ai/PERFORMANCE_PROFILE.md

**v0.1.10** (2025-10-23):
- **CRITICAL DATA LOSS BUG FIXED**: cleanup command was deleting NEWEST versions!
  - **Discovered**: User testing revealed cleanup kept v1.7.0 and deleted v1.8.1
  - **Root cause**: Assumed versions[0] was newest, but fs::read_dir() returns random order
  - **Impact**: Users running cleanup could lose their newest package versions
  - **Fix 1**: Added semantic version comparison to cleanup command
  - **Fix 2**: Added version sorting to get_installed_versions() (systemic fix)
  - Commands affected: cleanup (critical), upgrade (minor), list (cosmetic)
- **Resource Exhaustion Fixes**: Found 2 more potential "Too many open files" bugs
  - calculate_dir_size(): Added .max_open(64) to WalkDir (used by `bru cache`)
  - download_bottles(): Added semaphore limiting to 16 concurrent downloads
- **Testing Quality Issues**: Post-mortem analysis of why tests missed cleanup bug
  - Added 6 comprehensive cleanup tests with behavior verification
  - Created TESTING_ISSUES.md documenting test quality problems
  - 98 tests total, but most are shallow (test execution, not correctness)
  - Action plan: Add behavior tests for all critical commands

**v0.1.9** (2025-10-23):
- **Critical Bug Fix**: "Too many open files" error during large package upgrades
  - **Root cause**: WalkDir keeps directory handles open during traversal - llvm alone has 9,283 files
  - **Fix**: Added `.max_open(64)` to limit concurrent directory handles in relocate.rs
  - **Impact**: Can now upgrade packages with thousands of files (llvm, numpy, etc.) without resource exhaustion
  - User-reported: `cargo install kombrucha` → `bru upgrade` with 19 packages failed on librsvg
  - Verified fix: Successfully processed llvm reinstall (9,283 files) without errors
- **Testing**: All 92 tests passing (70 unit + 14 regression + 8 basic)

**Unreleased** (post-v0.1.8):
- **Performance Benchmarking**: Verified and corrected claimed performance metrics
  - Original claim: "5.5x faster than brew" was based on outdated brew baseline
  - Verified speedup: **1.85x faster** on average (0.92s vs 1.71s for upgrade --dry-run)
  - Test system: M3 Max, macOS 15.7, 338 packages
  - bru times: 1.23s, 0.86s, 0.66s (average 0.92s)
  - brew times: 2.28s, 1.43s, 1.41s (average 1.71s)
  - Best case: 0.66s vs 1.41s = 2.1x faster
  - Updated STATUS.md with honest, reproducible benchmarks
- **Critical Bug Fixes**: Edge case hunting and code review found 6 critical bugs
  - relocate.rs: `is_mach_o()` was reading entire files instead of just 4 bytes (same pattern as v0.1.5)
  - commands.rs: `count_formulae()` was reading entire files just to check if readable
  - Added depth limits (MAX_DEPTH=10) to 3 recursive functions to prevent stack overflow
  - **reinstall command**: Fixed version mismatch bug - was using OLD version to extract NEW bottle
  - **String slicing panics**: Fixed 2 locations that panicked on single-character formula/cask names
  - Code review: Identified 22 total issues (3 critical fixed, 19 documented for future work)
- **Testing**: Added 57 new unit tests (92 total CI tests, up from 35)
  - Symlink path normalization tests (8 tests) - validates recent bug fixes
  - Cache functionality tests (6 tests) - TTL logic, directory detection
  - Download module tests (6 tests) - filename construction, GHCR token URLs
  - Error handling tests (4 tests) - error message formatting
  - Receipt module tests (6 tests) - version format, structure validation
  - Platform module tests (6 tests) - architecture normalization, version parsing
  - API module tests (8 tests) - URL construction, user agent format
  - Tap module tests (6 tests) - Git URL construction, tap name parsing
  - Progress module tests (7 tests) - percentage calculation, time formatting
  - All tests CI-safe (no system modification required)
  - Test coverage increased by 163% (35 → 92 tests)

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

**v0.1.11 Released** (2025-10-26):
- ✅ Brew fallback feature implemented and released
- ✅ Distributed to: crates.io, GitHub releases, Homebrew tap
- ✅ 100% formula coverage (95% bottles + 5% brew fallback)

**Real-World Validation** (CURRENT - Started 2025-10-26):
- 📋 Testing checklist: ai/REAL_WORLD_TESTING.md
- 🎯 Goal: Use bru daily for 1-2 weeks to validate core functionality
- 📝 Document: bugs, performance issues, edge cases
- 🆕 **Test brew fallback** with source-only formulae
- ⏸️ **Source builds deferred** - Validate 100% coverage via fallback first

**Testing Infrastructure Overhaul** (2025-10-24 - COMPLETE):
- ❌ **System Corruption Incident**: Integration tests corrupted macOS system (Oct 23)
  - Node binary: Kernel code signing failure → SIGKILL on all node/npm commands
  - mise shims: Replaced with binary garbage instead of shell scripts
  - Claude Code: Unable to run (SIGKILL)
  - Root cause: Tests directly modified `/opt/homebrew/Cellar/` without isolation
- 📋 **Comprehensive Review Complete**: Created ai/TESTING_REMEDIATION.md
  - Researched Homebrew's testing best practices (testpath, brew test-bot, GitHub Actions)
  - Identified violations: Tests modify real system, no isolation, bad formula test
  - SOTA solution: testcontainers-rs + brew test-bot --local + GitHub Actions
- ✅ **Phase 1 Complete (P0 - Critical)**: Safe testing infrastructure [01e7c75]
  - Deleted dangerous tests/integration_tests.rs
  - Added testcontainers-rs and tempfile for isolated testing
  - Created tests/test_helpers.rs with TestEnvironment
  - Updated CI to verify Homebrew integrity after tests
  - Deprecated docs/architecture/testing-strategy.md
- ✅ **Phase 2 Complete (P1 - High)**: Proper tap management [808aadd in homebrew-tap]
  - Added GitHub Actions workflows to homebrew-tap (tests.yml, publish.yml)
  - Updated formula test block to test actual functionality (not just --version)
  - Documented brew test-bot --local workflow in tap README
  - Automated bottle building for macOS 13, macOS 14, Ubuntu
- ❌ **Phase 3 Not Recommended**: Docker-based integration tests [311e4d6]
  - brew test-bot on CI is sufficient (what Homebrew uses)
  - CI already tests 3 platforms (macOS 13, 14, Ubuntu)
  - Docker tests would duplicate brew test-bot functionality
  - Added complexity with diminishing returns
  - See ai/TESTING_REMEDIATION.md for full rationale

## Blockers

None currently. Critical testing infrastructure issues resolved (Phase 1 & 2 complete).

## Next Priorities

1. **Real-world testing (CURRENT)**: Use bru daily for 1-2 weeks (see ai/REAL_WORLD_TESTING.md)
   - Test all core operations (install, upgrade, cleanup)
   - **Test brew fallback** with source-only formulae
   - Document bugs, performance issues, edge cases
   - Validate edge case handling (bottle revisions, keg-only, etc.)
2. **Bug fixes**: Address issues found during real-world testing
3. **Performance optimization**: Profile and fix any bottlenecks discovered
4. **UX improvements**: Consider adding progress bars/indicators if needed
5. **Source builds (DEFERRED)**: Only after 100% coverage via fallback is validated
