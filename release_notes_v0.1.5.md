# v0.1.5 - Critical Bug Fix Release

## Critical Bug Fix

### File Descriptor Leak Fixed
**Impact:** Users could not upgrade packages with 1000+ files (like yt-dlp, python@3.14)
**Error:** `IO error: Too many open files (os error 24)`

**Root Cause:**
- `canonicalize()` was called thousands of times during symlink creation/removal
- Each call opens a file to resolve symlinks
- For yt-dlp (1722 files): 6,888 file descriptor opens
- Combined with previous packages in upgrade sequence: exceeded system limit

**Fix:**
- Compare symlink targets directly without canonicalizing
- Reduced file descriptor usage by ~7000 for large packages
- Tested with yt-dlp (1722 files), gh (215 files), multiple packages in sequence

## UX Improvements

### Icon Spacing
Icons no longer sit at the edge of the terminal window:
```
Before: ℹ Dry run mode...
After:   ℹ Dry run mode...
```

**Technical:** Space moved inside colored strings (`" ℹ".blue()`) so it doesn't get consumed by ANSI escape codes.

Applied to all status icons: ℹ (info), ✓ (success), ✗ (error), ⚠ (warning)

## Documentation

### Code Review
- Comprehensive review for resource leaks
- Verified all file handles properly scoped
- Verified no unbounded memory growth
- Identified testing gaps (need large package integration tests)
- Documented in `ai/CODE_REVIEW_2025_10_22.md`

## Upgrade Instructions

### Via Cargo
```bash
cargo install kombrucha --force
```

### Via Homebrew
```bash
brew upgrade nijaru/tap/bru
```

### From Source
```bash
git pull
cargo install --path . --force
```

## Testing

Tested on macOS 15.1 (M3 Max) with:
- yt-dlp upgrade (1722 files in cellar)
- gh upgrade (215 linked files)
- Multiple package upgrade sequences
- No file descriptor errors
- All operations complete successfully

## Breaking Changes

None - fully backward compatible with v0.1.4

## Known Issues

- Source builds not yet supported (use `brew` for those cases)
- Integration tests don't cover large package operations (identified in code review)

---

**Full Changelog:** https://github.com/nijaru/kombrucha/compare/v0.1.4...v0.1.5
