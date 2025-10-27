# UX/UI Audit - October 2025

## Current State

### ✅ Excellent - Already Implemented

1. **NO_COLOR Support** (colors.rs)
   - NO_COLOR environment variable
   - CLICOLOR and CLICOLOR_FORCE
   - TTY detection with std::io::IsTerminal
   - Automatic color disable when piped

2. **Progress Indicators** (v0.1.11)
   - Dependency resolution spinner
   - Download progress bars with Unicode blocks (━━╸)
   - Install counter (3/10)
   - Quiet mode (--quiet/-q, BRU_QUIET env var)

3. **Modern CLI Output**
   - Colored status icons (✓ ✗ ⚠ ℹ)
   - Tree connectors for hierarchy (├ └)
   - Command aliases (i, up, re, rm)

### 🟡 Good - Could Be Better

1. **Error Messages**
   - Currently using eprintln!() which goes to stderr ✓
   - Could add more context (what, why, how to fix)
   - Could use consistent error format
   - Example from research: "Error: <what> because <why>. Try: <solution>"

2. **Terminal Width Handling**
   - We have term_size dependency
   - Progress bars use indicatif (handles width automatically)
   - Could check if we handle narrow terminals gracefully

3. **Help Text**
   - Uses clap default formatting
   - Could review against clig.dev guidelines
   - Could ensure examples are included

### ❌ Missing - Industry Standards

1. **--no-color Flag**
   - We have NO_COLOR env var ✓
   - Missing explicit --no-color flag (nice-to-have, env var is standard)

2. **Structured Output**
   - --json flag exists for some commands
   - Could expand to more commands for scripting

## Research Findings

### Best Practices from clig.dev (2025)

1. **Error Handling**
   - stderr for errors, warnings ✓
   - stdout for actual output ✓
   - Exit code 0 for success, non-zero for failure ✓
   - Clear "what went wrong" and "how to fix" messages

2. **Progress & Interactivity**
   - Only show progress if stderr is a terminal ✓
   - Support quiet mode ✓
   - Don't show interactive elements when piped ✓

3. **Colors**
   - Respect NO_COLOR ✓
   - Auto-detect TTY ✓
   - Provide override flags ✓

### Unicode Progress Bar Best Practices

From research on Unicode progress bars:

1. **Terminal Width**
   - Auto-detect and adjust to terminal size
   - Minimum width handling (if < 12 chars, skip labels)
   - Handle resize (WINCH signal)

2. **Character Sets**
   - UTF-8 block characters (█ ▮ ▰) ✓ (we use ━━╸)
   - Fallback to ASCII for compatibility
   - Handle monospace vs proportional font issues

3. **Content Priority**
   - Essential: progress bar, percentage or count
   - Nice-to-have: ETA, speed
   - Skip in very narrow terminals

## Performance Profiling

### Tools Available

1. **cargo-flamegraph** (recommended)
   - Install: `cargo install flamegraph`
   - Usage: `cargo flamegraph --bin=bru -- upgrade --dry-run`
   - Need: debug = true in release profile for accurate stacks

2. **Configuration for Profiling**
   ```toml
   [profile.release]
   debug = true  # Add this for profiling (keep other opts)
   ```

3. **perf on Linux**
   - `perf record -F 99 -g -- ./target/release/bru upgrade --dry-run`
   - `perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg`

### What to Profile

1. **Hot Paths** (likely bottlenecks)
   - API calls (already parallelized, but check)
   - Bottle downloads (check semaphore effectiveness)
   - File operations (relocate.rs walk operations)
   - Dependency resolution

2. **Memory Usage**
   - Caching effectiveness (moka)
   - Large data structure allocations
   - Clone vs reference usage

3. **Async Performance**
   - Task spawn overhead
   - Semaphore contention
   - Future completion patterns

## Recommendations

### High Priority (Do Now)

1. **Add debug = true to release profile** for profiling capability
2. **Run flamegraph on `bru upgrade --dry-run`** to find bottlenecks
3. **Review error messages** for consistency and helpfulness

### Medium Priority (Consider)

1. **Add --no-color flag** as alias for NO_COLOR=1 (nice for users)
2. **Expand --json support** to more commands
3. **Add help examples** to major commands

### Low Priority (Future)

1. **Terminal width edge cases** (very narrow terminals)
2. **Interactive prompts** if we ever need confirmation dialogs
3. **Localization** (i18n) - only if international users request

## Action Items

- [x] Research modern CLI UX best practices
- [ ] Profile performance with cargo-flamegraph
- [ ] Review error message consistency
- [ ] Document findings in STATUS.md
- [ ] Consider --no-color flag addition
- [ ] Expand --json support if needed
