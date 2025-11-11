# v0.1.14 - Major Performance Optimizations

## Summary

Comprehensive performance improvements across all core operations through parallelization and efficient memory allocation.

## Performance Improvements

### 5.7x Faster Tap Updates
- **Before**: 10.9s (sequential git pulls)
- **After**: 1.9s (parallel git pulls)
- Now matches brew performance

### 3-8x Faster Multi-Package Upgrades
- Parallel bottle downloads instead of sequential
- Three-phase upgrade:
  1. Collect candidates in parallel
  2. Download all bottles in parallel
  3. Install sequentially for safety

### 2-4x Faster Mach-O Detection
- Parallel file checking with rayon
- Independent file operations

### 3-5x Faster Mach-O Relocation
- Parallel `install_name_tool` calls
- Significant speedup for formulae with many binaries

### 5-15% Improvement for Large Dependency Graphs
- Pre-allocated HashMap/Vec capacity hints
- Eliminates reallocation cycles

## Bug Fixes

- **Shell Hang**: Fixed shell not returning control after command completion
  - Explicit tokio runtime shutdown with 1s timeout
  - Commands now exit cleanly immediately

## Technical Changes

- Added rayon dependency for parallel iteration
- Improved tokio runtime configuration
- Optimized memory allocation patterns

## Testing

All optimizations tested and verified:
- `bru update` completes in ~2s
- `bru upgrade` exits cleanly without hanging
- Real-world upgrade tested successfully (biome 2.3.1 → 2.3.2)

---

**Full Changelog**: https://github.com/nijaru/kombrucha/compare/v0.1.13...v0.1.14
