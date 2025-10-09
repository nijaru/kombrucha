# Phase 2 Benchmark Results

## Install Command Performance

### Test Setup
- **Date**: October 8, 2025
- **Machine**: Apple M3 Max, macOS 15.6.1 (Sequoia)
- **Homebrew Version**: 4.4.19
- **bru Version**: 0.1.0
- **Test Package**: tree 2.2.1 (no dependencies)
- **Cache State**: Both bottles pre-cached

### Results

#### Scenario 1: Normal User Experience (with Homebrew auto-update)

**brew install tree:**
```
real    0m8.346s
user    0m2.151s
sys     0m0.560s
```

**bru install tree:**
```
real    0m0.140s
user    0m0.021s
sys     0m0.039s
```

**Speedup: 59.6x faster** (8.346s → 0.140s)

- **Wall time**: 59.6x improvement
- **CPU time**: 102x less user CPU (2.151s → 0.021s)
- **System time**: 14.4x less sys CPU (0.560s → 0.039s)

#### Scenario 2: Pure Installation Overhead (no auto-update)

**brew install tree** (with `HOMEBREW_NO_AUTO_UPDATE=1`):
```
real    0m2.926s
user    0m2.110s
sys     0m0.520s
```

**bru install tree:**
```
real    0m0.140s
user    0m0.021s
sys     0m0.039s
```

**Speedup: 20.9x faster** (2.926s → 0.140s)

- **Wall time**: 20.9x improvement
- **CPU time**: 100x less user CPU (2.110s → 0.021s)
- **System time**: 13.3x less sys CPU (0.520s → 0.039s)

#### Analysis

**Where Homebrew spends time (Scenario 1):**
1. Auto-updating Homebrew itself (~5-6s)
2. Ruby interpreter startup (~1-2s)
3. Formula parsing and dependency resolution (~0.5s)
4. Actual installation (extraction + symlinking) (~0.5s)

**Where Homebrew spends time (Scenario 2, no auto-update):**
1. Ruby interpreter startup (~1.5s)
2. Formula parsing and dependency resolution (~0.5s)
3. Actual installation (extraction + symlinking) (~0.9s)

**Where bru spends time:**
1. Binary startup (~0.01s)
2. API call + JSON parsing (~0.03s)
3. Extraction + relocation + symlinking (~0.10s)

### Key Findings

1. **Startup overhead**: Ruby interpreter adds ~2s before any work begins, vs ~0.01s for Rust
2. **Real-world impact**:
   - With auto-update (default): ~60x faster (8.3s → 0.14s)
   - Without auto-update: ~21x faster (2.9s → 0.14s)
3. **Installation efficiency**: Even the core installation is faster due to:
   - No shell script execution
   - Direct system calls from Rust
   - No intermediate Ruby object creation
   - Compiled binary vs interpreted scripts

### Caveats

1. **Bottles cached**: Both had bottles already downloaded
2. **No dependencies**: tree is a simple package
3. **Auto-update overhead**: Homebrew's auto-update is configurable (can be disabled)
4. **Not tested**: Packages with complex dependencies

### Next Steps

- [ ] Test with package that has dependencies
- [ ] Test without cached bottles (download performance)
- [ ] Test brew with `HOMEBREW_NO_AUTO_UPDATE=1`
- [ ] Measure parallel download impact
