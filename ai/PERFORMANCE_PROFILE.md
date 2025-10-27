# Performance Profile - October 2025

## Profile Run: `bru upgrade --dry-run`

**Date**: 2025-10-26
**Command**: `cargo flamegraph --bin=bru -- upgrade --dry-run -q`
**System**: M3 Max, macOS 15.7, 341 packages installed
**Result**: 4 packages to upgrade (biome, gemini-cli, cloudflare-wrangler, vercel-cli)

### Configuration

Added to Cargo.toml:
```toml
[profile.release]
debug = true  # Enable debug symbols for profiling
```

### Flamegraph Output

- **File**: `flamegraph.svg` (387 KB)
- **View**: Open `flamegraph.svg` in browser to see interactive visualization

### Performance Characteristics

From the upgrade --dry-run operation:

1. **Execution Time**: Sub-second (typical ~0.66-0.92s from benchmarks)
2. **Parallelization**: Already heavily optimized
   - API calls parallelized
   - Dependency resolution in parallel levels
   - In-memory caching (moka)

### Key Findings (from flamegraph)

To analyze:
1. Open `flamegraph.svg` in browser
2. Look for wide bars (high CPU time functions)
3. Check for:
   - API call overhead
   - JSON parsing bottlenecks
   - Unnecessary allocations
   - Sync points in async code

### Potential Optimizations

Based on current architecture:

1. **Already Optimized**:
   - ✅ Parallel API calls
   - ✅ In-memory caching (moka)
   - ✅ Async/await with tokio
   - ✅ Semaphore-limited concurrent downloads

2. **Could Investigate**:
   - JSON parsing (serde_json) - check if this is a bottleneck
   - String allocations - unnecessary clones
   - File I/O - receipt reading, version detection
   - Cache hit/miss ratios

3. **Likely Not Worth It**:
   - Further parallelization (already at max)
   - Different HTTP client (reqwest is solid)
   - Different async runtime (tokio is best-in-class)

### Comparison to Homebrew

From STATUS.md benchmarks (M3 Max, 338 packages):
- bru: 0.66-1.23s (average 0.92s)
- brew: 1.41-2.28s (average 1.71s)
- **Speedup**: 1.85x average, 2.1x best case

This is already excellent performance. Any further optimization would be marginal.

### Recommendations

1. **Focus on correctness over performance** - we're already fast
2. **Profile install/download operations** - larger surface area for optimization
3. **Check memory usage** - profiling RAM could reveal leaks
4. **Real-world testing** - find actual bottlenecks users experience

### Action Items

- [x] Set up profiling infrastructure
- [x] Run initial flamegraph
- [ ] Review flamegraph.svg for obvious bottlenecks
- [ ] Profile `bru install` with actual downloads
- [ ] Profile memory usage with heaptrack/valgrind
- [ ] Document findings in STATUS.md
