# Compatibility Requirements: Hard vs Flexible

## Hard Requirements (MUST Match Brew Exactly)

### 1. CLI Interface
**Must be identical for drop-in replacement**:
- ✅ Command names (`install`, `uninstall`, `upgrade`, etc.)
- ⚠️ Flag names (`--force`, `--verbose`, `--dry-run`, etc.)
- ⚠️ Flag behavior (what each flag does)
- ✅ Argument patterns (`FORMULA`, `[OPTIONS]`, etc.)
- ⚠️ Exit codes (0 = success, 1 = error, etc.)
- ⚠️ Environment variables (`HOMEBREW_*`)
- ✅ Command aliases

**Why**: Scripts, CI/CD, automation all depend on exact CLI

### 2. File System Structure
**Must match for compatibility**:
- ✅ `/opt/homebrew` or `/usr/local` prefix
- ✅ `Cellar/` structure
- ✅ `Caskroom/` structure
- ✅ Symlink paths (`bin/`, `lib/`, `include/`)
- ✅ Cache directory structure
- ✅ Tap directory structure (`Library/Taps/`)

**Why**: Other tools expect files in specific locations

### 3. JSON Output Structure
**Must match for parsing**:
- ⚠️ `--json` flag availability on all read commands
- ⚠️ JSON schema/structure for each command
- ⚠️ Field names in JSON output
- ⚠️ Data types (strings, arrays, objects)

**Why**: Scripts parse JSON output programmatically

### 4. Brewfile Format
**Must match**:
- ✅ Brewfile syntax
- ✅ `brew`, `cask`, `tap` directives
- ✅ Comments and options
- ✅ Bundle dump format

**Why**: Users share Brewfiles, expect compatibility

---

## Flexible Areas (Can Improve Over Brew)

### 1. Human-Readable Output Formatting
**We can improve**:
- ✅ Colors and styling (cyan, bold, dimmed)
- ✅ Progress bars and indicators
- ✅ Table formatting
- ✅ Tree visualization
- ✅ Emoji/icons (optional, we removed these)
- ✅ Spacing and layout

**Current status**: Already improved - better colors, clearer formatting

### 2. Progress Indicators
**We can improve**:
- ✅ Download progress (we show progress bars)
- ✅ Concurrent operation visibility
- ⚠️ Estimated time remaining (could add)
- ⚠️ Download speeds (could add)
- ⚠️ Parallel operation summary (could improve)

**Current status**: Good progress bars, could enhance

### 3. Error Messages
**We can improve**:
- ⚠️ More detailed error context
- ⚠️ Suggestions for fixes ("Did you mean...?")
- ⚠️ Links to documentation
- ⚠️ Troubleshooting steps
- ⚠️ Better error categorization

**Current status**: Basic errors, room for improvement

### 4. Performance Information
**We can add** (brew doesn't show this):
- ⚠️ Timing information (install took X seconds)
- ⚠️ Bandwidth usage
- ⚠️ Cache hit/miss stats
- ⚠️ Parallel efficiency metrics

**Current status**: Could leverage our speed advantage

### 5. Additional Helpful Information
**We can add** (not in brew):
- ⚠️ Dependency resolution explanation
- ⚠️ "X packages will be installed" summary
- ⚠️ Disk space requirements
- ⚠️ Security/deprecation warnings
- ⚠️ Update recommendations

**Current status**: Basic info, could enhance

### 6. Interactive Features
**We can add** (brew has minimal):
- ⚠️ Better confirmation prompts
- ⚠️ Interactive conflict resolution
- ⚠️ Progress visualization
- ⚠️ Clearer "what will happen" previews

**Current status**: Minimal interactivity

---

## Priority Audit: Command Output Review

### Commands to Review (by priority)

**Tier 1: High-Use Commands** (review first)
1. `install` - Most used, needs perfect output
2. `upgrade` - Very common, needs clear diff
3. `uninstall` - Common, needs confirmation
4. `list` - Very common, formatting matters
5. `search` - Very common, results display
6. `info` - Very common, data presentation
7. `outdated` - Common, needs clarity

**Tier 2: Discovery Commands**
8. `deps` - Tree visualization important
9. `uses` - List formatting
10. `desc` - Simple, probably fine
11. `leaves` - List formatting
12. `missing` - List formatting

**Tier 3: System Commands**
13. `config` - Key-value display
14. `doctor` - Diagnostic output
15. `env` - Environment display
16. `analytics` - State display

**Tier 4: Advanced/Dev Commands**
17. Everything else (audit later)

---

## Systematic Audit Process

For each command, check:

### 1. CLI Compatibility
- [ ] Does `bru <command> --help` work?
- [ ] Do all flags exist that brew has?
- [ ] Do flags behave the same way?
- [ ] Are arguments accepted in same format?

### 2. Output Quality
- [ ] Is output clear and well-formatted?
- [ ] Is color usage helpful (not distracting)?
- [ ] Are progress indicators smooth?
- [ ] Is information complete?

### 3. Improvement Opportunities
- [ ] Could we add helpful context?
- [ ] Could we improve error messages?
- [ ] Could we show performance metrics?
- [ ] Could we improve visual hierarchy?

### 4. JSON Output (if applicable)
- [ ] Does `--json` flag exist?
- [ ] Does JSON structure match brew?
- [ ] Is output parseable?

---

## Quick Audit Template

```bash
# For each command:

# 1. Test basic usage
bru <command>
brew <command>

# 2. Test with flags
bru <command> --help
brew <command> --help

# 3. Test JSON (if applicable)
bru <command> --json
brew <command> --json

# 4. Compare side-by-side
diff <(brew <command>) <(bru <command>)
```

---

## Output Comparison: Current Status

### Already Audited This Session
- ✅ `bru` (no args) - **FIXED**: Matches brew with better formatting
- ✅ `bru --help` - **FIXED**: Matches brew with better formatting
- ✅ `bru install --cask` - **TESTED**: Working, good output
- ✅ `bru uninstall --cask` - **TESTED**: Working, good output

### Need to Audit
- ⚠️ `bru install` (formula) - Check output vs brew
- ⚠️ `bru upgrade` - Check output vs brew
- ⚠️ `bru list` - Check output vs brew
- ⚠️ `bru search` - Check output vs brew
- ⚠️ `bru info` - Check output vs brew
- ⚠️ `bru deps` - Check tree visualization
- ⚠️ `bru outdated` - Check output format
- And ~100+ more commands...

---

## Recommended Approach

### Phase 1: Tier 1 Commands (30 min)
Audit the 7 most-used commands:
1. Compare `bru` vs `brew` output side-by-side
2. Note differences (good and bad)
3. Note missing flags
4. Note improvement opportunities

### Phase 2: Fix Critical Gaps (1-2 hours)
Based on Phase 1 findings:
1. Add missing flags to Tier 1 commands
2. Fix any broken output
3. Add `--help` support for subcommands

### Phase 3: Tier 2-3 Commands (1-2 hours)
Continue systematic audit

### Phase 4: Improvements (ongoing)
Implement improvements identified in audit

---

## Success Criteria

**Minimum for Beta**:
- [ ] Tier 1 commands have full flag parity
- [ ] Tier 1 commands have good output
- [ ] Subcommand `--help` works
- [ ] No broken output in any command
- [ ] JSON output matches brew (where applicable)

**Ideal for v1.0**:
- [ ] All commands have full flag parity
- [ ] All commands have improved output
- [ ] Helpful error messages with suggestions
- [ ] Performance metrics shown (leverage speed advantage)
- [ ] Better progress indicators

---

## Key Insight

**CLI**: Must be identical (users depend on it)
**Output**: Can be better (users appreciate it)
**Performance**: Is better (7-60x faster)

This is our competitive advantage - same interface, better experience, way faster.
