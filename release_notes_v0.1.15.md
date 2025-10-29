# v0.1.15 - UX & Quality Improvements

This release focuses on user experience enhancements and comprehensive testing to ensure the reliability of our parallel operations.

## 🎨 UX Improvements

### Live Progress Display
- **Parallel tap updates now show results as they complete** - No more waiting for all taps to finish before seeing output
- Better visual feedback during long operations
- Users can see parallelism in action

### Improved Error Messages
- **Consistent, colored error formatting** across all commands
- Clear "Error:" prefix with red highlighting
- Actionable suggestions with `--help` hints
- Reduced code duplication in error handling

## 🧪 Testing & Quality

### New Test Suite
- Added **5 comprehensive parallel operations tests**:
  - Parallel tap update correctness and performance
  - Parallel upgrade download functionality
  - Parallel API fetch operations
  - Services filtering correctness (cask-only exclusion)
- Total test count: **97 automated tests** (76 unit + 21 integration)

### Bug Fixes
- Fixed services list showing cask-only plists (e.g., tailscale-app)
- Now correctly filters to only show services for installed formulae
- Parity with `brew services list` behavior

## 📚 Documentation

### Updated Performance Benchmarks
Real-world measurements on M3 Max (October 2025):
- `bru outdated`: **2.1x faster** (780ms vs 1.63s)
- `bru info`: **9.6x faster** (107ms vs 1.04s)
- `bru search`: **24x faster** (43ms vs 1.04s)
- `bru update`: **5.7x faster** (1.9s vs ~11s sequential)
- `bru upgrade`: **3-8x faster** (parallel downloads)

### Modern CLI Patterns
- Documented approach to live progress indicators
- Error message best practices
- Clean, modern output formatting

## 🔧 Technical Details

### Commits
- 361c845: Live progress display for parallel operations
- cd3f51f: Improved error message formatting
- a61a24f: Comprehensive parallel operations test suite
- fadded4: Updated documentation with real benchmarks

### Breaking Changes
None - fully backward compatible with v0.1.14

## 📦 Installation

```bash
brew update
brew upgrade nijaru/tap/bru
```

Or download binaries from the release page.

## What's Next

Looking ahead to v0.1.16:
- Additional feature enhancements based on user feedback
- Continued performance optimizations
- Extended test coverage for edge cases
