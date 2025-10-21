# UX Improvements for Drop-in Replacement

**Goal**: Full drop-in replacement `brew` → `bru` with better formatting
**Primary**: Performance (✅ achieved: 7-60x faster)
**Secondary**: UX improvements while maintaining compatibility

## Current Status

### What Works Well ✅
- Core functionality (install, uninstall, upgrade)
- Comprehensive cask support (DMG, ZIP, TAR.GZ, binaries, suites)
- Beautiful colorized output with clear formatting
- Default help output matches brew exactly
- Dependency resolution
- Fast operations (parallelism)

### Critical Gaps for Drop-in Replacement

#### 1. **CLI Flag Compatibility** 🚨 HIGH PRIORITY
**Problem**: Missing many brew flags that users rely on

**Missing flags for `install`**:
- `--force` - Overwrite existing installations
- `--dry-run` / `-n` - Show what would be installed
- `--ignore-dependencies` - Skip dependencies
- `--formula` / `--formulae` - Explicitly treat as formula
- `--HEAD` - Install HEAD version (Phase 3)
- `--build-from-source` - Build from source (Phase 3)
- `--only-dependencies` - Only install deps (we have this!)
- `--cc` - Compiler to use (Phase 3)
- `--display-times` - Show install times
- `--interactive` - Interactive mode

**Missing flags for other commands**:
- `search --desc` - Search descriptions
- `list --full-name` - Show full names
- `list --multiple` - Show formulae installed multiple times
- `upgrade --greedy` - Upgrade auto-updating casks
- `cleanup --prune` - Remove files older than specified days
- `info --github` - Show GitHub stats
- Many more...

**Impact**: Users with scripts using these flags will fail
**Priority**: HIGH - Add most common flags first

#### 2. **Subcommand Help** 🚨 HIGH PRIORITY
**Problem**: `bru install --help` doesn't work (clap limitation)

**Needed**:
```bash
bru install --help          # Should show install-specific help
bru upgrade --help          # Should show upgrade-specific help
bru list --help             # Should show list-specific help
```

**Impact**: Users can't discover command-specific options
**Priority**: HIGH
**Solution**: Catch --help in each command handler or use clap properly

#### 3. **Environment Variables** 🚨 MEDIUM PRIORITY
**Problem**: Don't respect HOMEBREW_* environment variables

**Variables brew respects** (100+ total):
- `HOMEBREW_NO_AUTO_UPDATE` - Skip auto-update
- `HOMEBREW_NO_INSTALL_CLEANUP` - Skip cleanup after install
- `HOMEBREW_NO_INSTALL_UPGRADE` - Don't upgrade already-installed
- `HOMEBREW_CACHE` - Cache directory
- `HOMEBREW_PREFIX` - Installation prefix
- `HOMEBREW_CELLAR` - Cellar directory
- `HOMEBREW_CASK_OPTS` - Cask install options
- `HOMEBREW_DISPLAY_INSTALL_TIMES` - Show timing
- `HOMEBREW_VERBOSE` - Verbose output
- `HOMEBREW_DEBUG` - Debug mode
- Many more...

**Impact**: Behavior differs from brew in scripted environments
**Priority**: MEDIUM - Add most common ones first

#### 4. **JSON Output Consistency** 📊 MEDIUM PRIORITY
**Problem**: `--json` flag not available on all read commands

**Commands that should have --json**:
- ✅ `info --json` - Have it
- ✅ `list --json` - Have it
- ❌ `deps --json` - Missing
- ❌ `uses --json` - Missing
- ❌ `outdated --json` - Missing
- ❌ `search --json` - Missing
- ❌ `config --json` - Missing
- ❌ `doctor --json` - Missing

**Impact**: Scripts that parse output will fail
**Priority**: MEDIUM

#### 5. **Error Messages & Exit Codes** ⚠️ MEDIUM PRIORITY
**Problem**: Unknown if we match brew's exit codes

**Brew exit codes**:
- `0` - Success
- `1` - Generic error
- `2` - Formula/cask not found
- `3` - Already installed
- Various others...

**Needed**:
- Test and document our exit codes
- Match brew's exit codes exactly
- Improve error messages with suggestions

**Impact**: Scripts that check exit codes may behave incorrectly
**Priority**: MEDIUM

#### 6. **Progress Indicators** 📈 LOW PRIORITY (UX)
**Problem**: Our progress is good but could be better

**Current**: Basic progress for downloads/installs
**Better UX**:
- Show concurrent operations in progress
- Display download speeds
- Show estimated time remaining
- Better parallel operation visibility
- Summary at end showing timing

**Impact**: Better user experience, not critical for compatibility
**Priority**: LOW (UX improvement)

#### 7. **Symlink Management** 🔗 UNKNOWN PRIORITY
**Problem**: Unknown if we handle keg linking identically to brew

**Needed**:
- Verify `bru link` / `bru unlink` match brew exactly
- Test keg-only formulae linking
- Verify conflict detection
- Test multiple version linking

**Impact**: Could break existing setups
**Priority**: HIGH to test, MEDIUM to fix

#### 8. **Formula Path Discovery** 📁 UNKNOWN PRIORITY
**Problem**: Unknown if our path detection matches brew exactly

**Needed**:
- Verify we use same Cellar structure
- Verify we detect bottles correctly
- Test with custom prefix locations
- Test with multiple architectures (x86_64 vs arm64)

**Impact**: Could cause compatibility issues
**Priority**: MEDIUM to test

## Recommendations: Phased Approach

### Phase 1: Critical Compatibility (1-2 days)
**Goal**: Don't break user scripts

1. Add subcommand --help support
2. Add most common missing flags:
   - `--force`
   - `--dry-run`
   - `--formula` / `--cask` (explicit)
   - `--ignore-dependencies`
3. Add critical environment variables:
   - `HOMEBREW_NO_AUTO_UPDATE`
   - `HOMEBREW_CACHE`
   - `HOMEBREW_PREFIX`
   - `HOMEBREW_VERBOSE`
4. Test and document exit codes

### Phase 2: Drop-in Replacement (2-3 days)
**Goal**: True brew replacement

1. Add remaining common flags to all commands
2. Add JSON output to all read commands
3. Verify symlink management matches brew
4. Test with real-world scripts and workflows
5. Add missing environment variables

### Phase 3: UX Improvements (1-2 days)
**Goal**: Better than brew

1. Better progress indicators for concurrent operations
2. Improved error messages with suggestions
3. Better formatting while maintaining compatibility
4. Performance optimizations
5. Display timing by default (our advantage!)

### Phase 4: Edge Cases (1-2 days)
**Goal**: Handle everything brew handles

1. Test keg-only formulae extensively
2. Test complex dependency scenarios
3. Test with custom prefixes
4. Test architecture-specific bottles
5. Document any intentional differences

## What NOT to Change

**Keep these as-is** (our improvements over brew):
- ✅ Colorized output (better UX)
- ✅ Better formatting (clearer, more readable)
- ✅ Parallel operations (faster)
- ✅ Better progress visibility
- ✅ Cleaner metadata tracking

**Philosophy**:
- Match brew's CLI interface exactly (flags, args, exit codes)
- Improve visual output (colors, formatting, progress)
- Better performance (parallelism, less overhead)

## Testing Checklist

### Compatibility Testing
- [ ] Run common brew scripts with `alias brew=bru`
- [ ] Test with Brewfile workflows
- [ ] Test with CI/CD pipelines using brew
- [ ] Test with dotfiles that use brew
- [ ] Document any breaking changes

### Flag Parity Testing
- [ ] Audit all 116 commands for flag differences
- [ ] Test most common 20 flag combinations
- [ ] Document unsupported flags (Phase 3 only)

### Environment Variable Testing
- [ ] List all HOMEBREW_* vars brew respects
- [ ] Test common env var combinations
- [ ] Document which vars we support

## Bottom Line

**Current State**:
- Core functionality: ✅ Excellent
- Performance: ✅ Outstanding (7-60x faster)
- Cask support: ✅ Comprehensive (85-90%)
- CLI compatibility: ⚠️ 70-80% (missing many flags)
- Environment compatibility: ⚠️ Unknown (need testing)

**For True Drop-in Replacement**:
1. Add missing CLI flags (highest priority)
2. Support HOMEBREW_* environment variables
3. Match exit codes exactly
4. Test with real-world scripts

**Timeline to Beta**:
- Phase 1 (Critical): 1-2 days → Ready for careful testing
- Phase 2 (Drop-in): +2-3 days → Ready for general use
- Phase 3 (UX): +1-2 days → Better than brew

**Current Status**: Approaching Beta (need Phase 1 complete first)
