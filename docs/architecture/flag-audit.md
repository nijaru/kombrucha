# Command Flag Audit

**Date**: October 15, 2025
**Purpose**: Systematic comparison of flags between brew and bru

---

## Summary

This document tracks all missing command-line flags across all commands. Flags are categorized by priority:

- **🚨 HIGH**: Breaking compatibility or critical functionality
- **⚠️ MEDIUM**: Important for common workflows
- **💡 LOW**: Nice-to-have, edge cases, or rarely used

---

## Tier 1 Commands

### install

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `-f, --force` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-21 - force install even if already installed |
| `-n, --dry-run` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-21 - show what would be installed without installing |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - disambiguation flag |
| `-d, --debug` | ✅ | ❌ | ⚠️ MEDIUM | Interactive debugging session |
| `--display-times` | ✅ | ❌ | 💡 LOW | Print install times at end |
| `--ask` | ✅ | ❌ | 💡 LOW | Confirm before installing |
| `--ignore-dependencies` | ✅ | ⚠️ | 💡 LOW | bru has `--only-dependencies` (opposite) |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 2 flags (0 high, 1 medium, 2 low)

---

### upgrade

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `-n, --dry-run` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-21 - show what would be upgraded without upgrading |
| `-f, --force` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-21 - force upgrade even if already at latest version |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - disambiguation flag |
| `-d, --debug` | ✅ | ❌ | 💡 LOW | Interactive debugging |
| `--display-times` | ✅ | ❌ | 💡 LOW | Print upgrade times |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 2 flags (0 high, 0 medium, 2 low)

---

### uninstall

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--zap` | ✅ | ❌ | ⚠️ MEDIUM | Remove all files for casks (shared files warning) |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - disambiguation flag |
| `-d, --debug` | ✅ | ❌ | 💡 LOW | Debug output |
| `-q, --quiet` | ✅ | ❌ | 💡 LOW | Quiet mode |
| `--ignore-dependencies` | ✅ | ⚠️ | ⚠️ MEDIUM | bru has `--force` which may serve similar purpose |
| `-f, --force` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 3 flags (0 high, 2 medium, 2 low)

---

### search

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--desc` | ✅ | ❌ | ⚠️ MEDIUM | Search descriptions (bru already does this by default) |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--eval-all` | ✅ | ❌ | 💡 LOW | Evaluate all formulae (performance impact) |
| `--pull-request` | ✅ | ❌ | 💡 LOW | Search GitHub PRs |
| `--open` | ✅ | ❌ | 💡 LOW | Only open PRs |
| `--closed` | ✅ | ❌ | 💡 LOW | Only closed PRs |
| External DB flags | ✅ | ❌ | 💡 LOW | --alpine, --repology, --macports, etc. (12 flags) |
| `-d, --debug` | ✅ | ❌ | 💡 LOW | Debug output |
| `-q, --quiet` | ✅ | ❌ | 💡 LOW | Quiet mode |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing ~17 flags (0 high, 1 medium, 16 low)
**Note**: bru already searches descriptions by default, so `--desc` may be redundant

---

### info

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--analytics` | ✅ | ❌ | 💡 LOW | Analytics data (requires GitHub API integration) |
| `--days` | ✅ | ❌ | 💡 LOW | Analytics timeframe |
| `--category` | ✅ | ❌ | 💡 LOW | Analytics category |
| `--github` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-18 - opens GitHub source page in browser |
| `--fetch-manifest` | ✅ | ❌ | 💡 LOW | Fetch extra GitHub Packages info |
| `--installed` | ✅ | ❌ | 💡 LOW | Filter to installed only |
| `--json` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 5 flags (0 high, 0 medium, 5 low)

---

### list

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--full-name` | ✅ | ❌ | 💡 LOW | Fully-qualified names |
| `--multiple` | ✅ | ❌ | 💡 LOW | Only show formulae with multiple versions |
| `--pinned` | ✅ | ❌ | 💡 LOW | Not implemented - would show only pinned formulae |
| `--installed-on-request` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - show manually installed packages |
| `--installed-as-dependency` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - show dependency installs |
| `--poured-from-bottle` | ✅ | ❌ | 💡 LOW | Show bottle vs source installs |
| `--built-from-source` | ✅ | ❌ | 💡 LOW | Show source builds |
| `-1, --quiet` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-21 - pipe-aware quiet mode |
| `-l` | ✅ | ❌ | 💡 LOW | Long format (ls -l style) |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-15 - disambiguation flag |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--versions` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--json` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 3 flags (0 high, 0 medium, 3 low)

---

### deps

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `-n, --topological` | ✅ | ❌ | 💡 LOW | Topological sort |
| `-1, --direct, --declared` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-18 - only direct dependencies |
| `--union` | ✅ | ❌ | 💡 LOW | Union vs intersection for multiple formulae |
| `--full-name` | ✅ | ❌ | 💡 LOW | Fully-qualified names |
| `--include-implicit` | ✅ | ❌ | 💡 LOW | Download/unpack dependencies |
| `--include-build` | ✅ | ✅ | ✅ DONE | Not needed - bru shows build dependencies by default (verified 2025-10-18) |
| `--include-optional` | ✅ | ❌ | 💡 LOW | Optional dependencies |
| `--include-test` | ✅ | ❌ | 💡 LOW | Test dependencies |
| `--skip-recommended` | ✅ | ❌ | 💡 LOW | Skip recommended deps |
| `--include-requirements` | ✅ | ❌ | 💡 LOW | Include requirements |
| `--tree` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--installed` | ✅ | ✅ | ✅ DONE | Already implemented |
| `-v, --verbose` | ✅ | ✅ | ✅ DONE | Already implemented |

**Status**: Missing 9 flags (0 high, 1 medium, 8 low)
**Note**: bru shows build dependencies by default, brew requires `--include-build`

---

### outdated

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--fetch-HEAD` | ✅ | ❌ | 💡 LOW | Detect outdated HEAD installations |
| `-g, --greedy` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-18 - include casks with auto_updates or version :latest |
| `--greedy-latest` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-18 - only show casks with version :latest |
| `--greedy-auto-updates` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-18 - only show casks with auto_updates |
| `--formula, --formulae` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-17 - disambiguation flag |
| `--cask, --casks` | ✅ | ✅ | ✅ DONE | Already implemented |
| `--json` | ✅ | ✅ | ✅ DONE | Implemented 2025-10-17 - JSON output for outdated packages |
| `-v, --verbose` | ✅ | ❌ | 💡 LOW | Verbose output (bru has this but not listed?) |

**Status**: Missing 1 flag (0 high, 0 medium, 1 low)

---

## Tier 2 Commands

### uses

| Flag | brew | bru | Priority | Notes |
|------|------|-----|----------|-------|
| `--recursive` | ✅ | ❌ | 💡 LOW | Not implemented - would show recursive dependents |
| `--installed` | ✅ | ❌ | 💡 LOW | Not implemented - would filter to installed only |
| `--include-build` | ✅ | ❌ | 💡 LOW | Include build dependents |
| `--include-optional` | ✅ | ❌ | 💡 LOW | Include optional dependents |
| `--include-test` | ✅ | ❌ | 💡 LOW | Include test dependents |
| `--skip-recommended` | ✅ | ❌ | 💡 LOW | Skip recommended dependents |
| `--formula, --formulae` | ✅ | ❌ | 💡 LOW | Treat as formula |
| `--cask, --casks` | ✅ | ❌ | 💡 LOW | Treat as cask |
| `-v, --verbose` | ✅ | ❌ | 💡 LOW | Verbose output |

**Status**: Missing 7 flags (0 high, 0 medium, 7 low)

---

## Priority Summary

### 🚨 HIGH Priority (Block Beta) ✅ COMPLETED

1. ✅ **install --dry-run** - Show what would be installed without installing (DONE 2025-10-15)
2. ✅ **upgrade --dry-run** - Show what would be upgraded without upgrading (DONE 2025-10-15)

**Status**: All high-priority flags implemented and tested. Beta release no longer blocked by missing safety flags.

---

### ⚠️ MEDIUM Priority (Target for v1.0)

1. ✅ **--formula/--formulae** - DONE (2025-10-15) - Disambiguation flag on install, upgrade, uninstall, list, outdated
2. ✅ **install --force** - DONE (2025-10-15) - Force install even if already installed
3. ✅ **list --installed-on-request** - DONE (2025-10-15) - Distinguish manual vs dependency installs
4. ✅ **list --installed-as-dependency** - DONE (2025-10-15) - Show dependency installs
5. ✅ **list --pinned** - DONE (2025-10-17) - Show pinned packages
6. ✅ **deps --direct** - DONE (2025-10-18) - Show only direct dependencies
7. ✅ **outdated --json** - DONE (2025-10-17) - JSON output for outdated
8. ✅ **outdated --greedy** - DONE (2025-10-18) - Include casks with auto-updates or version :latest
9. ✅ **info --github** - DONE (2025-10-18) - Open GitHub source page in browser
10. ✅ **uses --recursive** - DONE (2025-10-18) - Show dependents of dependents recursively
11. ✅ **uses --installed** - DONE (2025-10-18) - Only show installed dependents
12. ✅ **deps --include-build** - DONE (not needed - bru shows build deps by default)
13. **uninstall --zap** - Blocked: requires Ruby cask parsing (zap stanzas not in JSON API)

**Rationale**: These affect common workflows and scripting, but have workarounds or are less critical than dry-run.

---

### 💡 LOW Priority (Nice to Have)

Everything else (~70+ flags), including:
- Debug flags (`-d, --debug`)
- Quiet flags (`-q, --quiet`)
- Display options (`--display-times`)
- Analytics flags
- External database search flags
- Advanced dependency filtering
- etc.

**Rationale**: Rarely used, edge cases, or provide minimal value. Can be added incrementally based on user demand.

---

## Implementation Plan

### Phase 1: High Priority (Block Beta) ✅ COMPLETED

1. ✅ Implement `--dry-run` for `install` command - DONE (2025-10-15)
2. ✅ Implement `--dry-run` for `upgrade` command - DONE (2025-10-15)

### Phase 2: Medium Priority (v1.0)

1. Add `--formula/--formulae` to all major commands
2. Verify `--force` implementation matches brew
3. Add install tracking (`--installed-on-request` vs dependencies)
4. Add `--json` to `outdated`
5. Add `--github` to `info`
6. Add advanced filtering to `list`, `deps`, `uses`

### Phase 3: Low Priority (Post-v1.0)

1. Add debug/quiet flags as needed
2. Add analytics integration
3. Add external database search
4. Add advanced dependency filtering options

---

## Notes

- **Already Better**: bru's search already includes descriptions by default (brew requires `--desc`)
- **Already Better**: bru's deps shows build dependencies by default (brew requires `--include-build`)
- **Already Better**: bru's outdated shows upgrade arrows (brew just lists names)
- **Different Approach**: bru has `--only-dependencies` while brew has `--ignore-dependencies` (opposite)

---

**Next Steps**:
1. Implement high-priority flags (--dry-run for install/upgrade)
2. Add --formula/--formulae disambiguation
3. Audit --force implementation for full compatibility
