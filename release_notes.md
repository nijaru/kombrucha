# v0.1.30 - CRITICAL FIX: Autoremove Root Cause

## ⚠️ CRITICAL BUG FIX

**Fixed upgrade/reinstall writing receipts with EMPTY runtime_dependencies**

This bug caused autoremove to incorrectly remove required packages, breaking curl, llvm, lld, and other installed software.

## What Was Wrong

- **Root Cause**: upgrade/reinstall commands passed incomplete formula map to `build_runtime_deps()`
- **Result**: All dependency lookups failed → receipts written with `runtime_dependencies: []`
- **Impact**: Autoremove saw packages had no dependencies → removed required packages
- **Affected Versions**: v0.1.18 - v0.1.29

## What Was Fixed

### 1. Upgrade Command
- Now resolves complete dependency tree before generating receipts
- Matches install command behavior

### 2. Reinstall Command
- Now resolves complete dependency tree before generating receipts
- Matches install command behavior

### 3. Autoremove Command
- Reverted to receipt-based traversal (removed async/API calls)
- **100-300x faster** (<20ms vs 1-3s)
- Works offline (no network calls)
- Matches Homebrew behavior exactly

## Impact

✅ Receipts now correct after upgrade/reinstall
✅ Autoremove no longer removes required dependencies
✅ 100-300x faster autoremove
✅ Works offline
✅ Simpler code (-23 lines net)

## Upgrade Instructions

**IMPORTANT**: If you were affected by this bug (installed packages have broken dependencies), you may need to reinstall:

```bash
# Optional: Reinstall packages to regenerate correct receipts
bru reinstall $(bru list)

# Or selectively reinstall affected packages
bru reinstall curl llvm lld
```

## Documentation

- Updated README and CLAUDE.md to note **experimental/unstable** status
- Use bru alongside Homebrew, not as a replacement
- Always keep brew installed as fallback

## Files Changed

- `src/commands.rs`: Fixed upgrade, reinstall, autoremove
- `src/main.rs`: Updated autoremove call
- `README.md`: Added experimental status warning
- `CLAUDE.md`: Added version and status
- `ai/STATUS.md`: Detailed changelog

## Testing

- ✅ All 76 unit tests pass
- ✅ upgrade --dry-run works correctly
- ✅ autoremove --dry-run: <20ms (was 1-3s)
- ✅ No regressions in other commands
- ✅ Testing improvements documented for future work

## Full Changes

See [COMPREHENSIVE_REVIEW.md](COMPREHENSIVE_REVIEW.md) for complete technical analysis.
