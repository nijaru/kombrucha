# Real-World Testing Checklist

**Goal:** Validate bru's 95% bottle-based functionality through daily use before considering source builds.

**Timeline:** 1-2 weeks of regular usage

## Testing Approach

Use bru for **everything** instead of brew. Document:
- ✅ What works smoothly
- ❌ What breaks
- 🐌 What's slow
- 🤔 What's confusing

## Core Operations (Use Daily)

### Package Management
- [ ] Install common packages (wget, curl, jq, ripgrep, fd, etc.)
- [ ] Install large packages (llvm, node, python@3.13, rust)
- [ ] Install packages with many dependencies (ffmpeg, postgresql)
- [ ] Upgrade all packages with `bru upgrade`
- [ ] Upgrade specific packages with `bru upgrade <formula>`
- [ ] Reinstall broken packages with `bru reinstall <formula>`
- [ ] Uninstall packages with `bru uninstall <formula>`
- [ ] Run cleanup with `bru cleanup`

### Discovery & Info
- [ ] Search for packages with `bru search <query>`
- [ ] Get package info with `bru info <formula>`
- [ ] Check dependencies with `bru deps <formula>`
- [ ] Check reverse deps with `bru uses <formula>`
- [ ] List installed with `bru list`
- [ ] Check outdated with `bru outdated`
- [ ] Check leaves with `bru leaves`

### System Operations
- [ ] Update formula index with `bru update`
- [ ] Add/remove taps with `bru tap` / `bru untap`
- [ ] Run system check with `bru doctor`
- [ ] Check environment with `bru env`
- [ ] Test shellenv with `eval "$(bru shellenv)"`

### Cask Operations (macOS)
- [ ] Install casks with `bru install --cask <cask>`
- [ ] Upgrade casks with `bru upgrade --cask`
- [ ] List casks with `bru list --cask`
- [ ] Search casks with `bru search --cask <query>`

## Edge Cases to Test

### Version Handling
- [ ] Packages with bottle revisions (mosh, gh)
- [ ] Keg-only packages (openssl@3, python@3.13)
- [ ] Multiple installed versions (keep old after upgrade)
- [ ] Pinned packages (if we support pinning)

### Platform Compatibility
- [ ] Packages with arm64_sequoia bottles
- [ ] Packages with "all" platform bottles (yarn, node)
- [ ] Packages falling back to older macOS bottles
- [ ] Packages without bottles (should fail gracefully)

### Dependency Scenarios
- [ ] Install package with complex dep tree (ffmpeg → 44 deps)
- [ ] Install package with circular deps (if any)
- [ ] Install package with optional deps
- [ ] Install package with build-only deps

### Error Handling
- [ ] Try to install non-existent formula
- [ ] Try to install source-only formula (should show clear error)
- [ ] Try to upgrade already-current packages
- [ ] Try to uninstall non-installed package
- [ ] Try to reinstall with broken Cellar state

### Performance Testing
- [ ] Cold start: `time bru --version` (should be <0.01s)
- [ ] Search: `time bru search rust` (should be <1s)
- [ ] Info: `time bru info wget` (should be <0.5s)
- [ ] Outdated check: `time bru outdated` (should be <2s for 300+ packages)
- [ ] Full upgrade dry-run: `time bru upgrade --dry-run` (should be <2s)

### Compatibility with brew
- [ ] bru and brew show same outdated packages
- [ ] bru info matches brew info output format
- [ ] bru search matches brew search results
- [ ] bru list matches brew list
- [ ] Cellar structure is identical after install
- [ ] Can switch between bru/brew without issues

## Issues to Document

When something goes wrong, capture:

```markdown
### Issue: [Short description]
**Command:** `bru <command>`
**Expected:** What should happen
**Actual:** What happened instead
**Error:** Full error message
**Reproducer:** Minimal steps to reproduce
**Workaround:** If you found one
```

## Known Limitations (Expected)

These are OK to fail:
- ❌ Source builds (formulae without bottles) - use `brew` for these
- ❌ Formula creation (`bru create`) - stub only
- ❌ Formula auditing (`bru audit`) - stub only
- ❌ Livecheck (`bru livecheck`) - not implemented

## Success Criteria

After 1-2 weeks of daily use:

**Go/No-Go Decision:**

✅ **Ready for wider use if:**
- Core operations work reliably (install, upgrade, uninstall)
- Performance is consistently faster than brew
- No data loss bugs (like the Oct 23 cleanup issue)
- Edge cases handled gracefully
- Error messages are clear

❌ **Needs more work if:**
- Frequent crashes or hangs
- Incorrect version detection
- Corrupts Cellar or symlinks
- Performance regressions
- Confusing error messages

## After Testing

Based on findings, prioritize:
1. **Critical bugs** - Fix immediately
2. **Performance issues** - Profile and optimize
3. **Edge case handling** - Improve error messages
4. **Missing features** - Add if frequently needed
5. **Source builds** - Only if users actually need it

---

**Start Date:** _____
**End Date:** _____
**Status:** Not started
