# PR #5 Review Issues - All Resolved ✅

## Critical

| Issue | Impact | Status | Details |
|-------|--------|--------|---------|
| Incomplete unlink error handling | Race condition | ✅ FIXED | Changed `let _ =` to log errors instead of silently ignoring. |
| Relocation orphans | Data leak | ✅ FIXED | Added cleanup of extracted bottle on relocation failure. |

## High

| Issue | Impact | Status | Details |
|-------|--------|--------|---------|
| Weakened test parity | Hidden bugs | ✅ FIXED | Added minimum threshold check (brew_count - 20) for bidirectional validation. |
| Fallback path unclear | Incomplete review | ✅ VERIFIED | `without_bottles` packages use `fallback_to_brew("upgrade", ...)` - maintains consistency. |
| Atomic counter ordering | Potential jumps | ✅ DOCUMENTED | Added comment explaining Relaxed ordering is sufficient for UI-only progress updates. |

## Medium

| Issue | Impact | Status | Details |
|-------|--------|--------|---------|
| Removed function docs | Clarity | ✅ DOCUMENTED | Bottle revision behavior documented in test_regression_upgrade_bottle_revision. |
| Progress bar visibility | UX | ✅ FIXED | Changed to `finish_with_message()` to show "Extracted N bottles" status. |
| Test dates | Maintenance | ✅ VERIFIED | Dates are accurate: 2025-10-21 (bug discovery), 2025-11-16 (today - behavior docs). |
| MSRV changes incomplete | Verification | ✅ VERIFIED | Uses modulo operations; `manual_is_multiple_of` lint allowed in CI config. MSRV 1.85.0 compatible. |

---

# Current Issues

## Outdated Detection Can Miss Recent Updates

**Severity**: Medium
**Status**: ✅ FIXED
**Affects**: `bru outdated`, `bru upgrade`
**Fixed in**: Commit [current]

### Description

`bru` can miss recently updated packages that `brew` correctly identifies as outdated. This happens because `bru` relies on Homebrew's JSON API (https://formulae.brew.sh/api/) which can lag behind the actual git tap repositories.

### Example

```bash
❯ bru update && bru upgrade
# Finds 4 outdated packages

❯ brew update && brew upgrade
# Finds 7 additional outdated packages that bru missed:
# - mise: 2025.11.5 → 2025.11.6
# - uv: 0.9.9 → 0.9.10
# - gemini-cli: 0.15.4 → 0.16.0
# - biome: 2.3.5 → 2.3.6
# - difftastic: 0.65.0 → 0.67.0
# - git: 2.51.2 → 2.52.0
# - opencode: 1.0.68 → 1.0.74
```

### Root Cause

**bru's approach:**
- Uses `api.fetch_formula()` → calls `https://formulae.brew.sh/api/formula/{name}.json` for each package
- API responses may be cached/served from CDN with stale data

**brew's approach:**
- Uses local git tap at `/opt/homebrew/Library/Taps/homebrew/homebrew-core`
- Updated via `git pull` to get latest formula definitions

The JSON API can lag behind git repository updates, especially for recently pushed changes.

### Code Location

The issue is in `src/commands/install.rs:706` where homebrew/core packages fall back to the API:

```rust
// Fallback to API for homebrew/core packages
let formula = api.fetch_formula(&pkg.name).await.ok()?;
```

For custom taps, the code already reads from local tap repositories (lines 688-703), but this logic doesn't apply to homebrew/core.

### Proposed Solutions

#### Option 1: Check local homebrew/core tap first (Recommended)
Read formula files directly from `/opt/homebrew/Library/Taps/homebrew/homebrew-core/Formula/` before falling back to API. This ensures parity with brew's behavior.

**Pros:**
- Always consistent with brew
- No API latency/staleness issues
- Matches brew's source of truth

**Cons:**
- Requires homebrew/core tap to be present (standard for brew installations)
- Need to parse Ruby formula files

#### Option 2: Use bulk API endpoint
Fetch all formulas at once via `https://formulae.brew.sh/api/formula.json` instead of individual endpoints.

**Pros:**
- Single HTTP request for all formulas
- May be fresher than individual endpoints

**Cons:**
- Still dependent on API freshness
- Large payload (~25 MB)

#### Option 3: Add force-refresh flag
Allow users to bypass cache with `bru update --force` or `bru upgrade --force-api-refresh`.

**Pros:**
- Simple to implement
- User control

**Cons:**
- Requires user awareness of the issue
- Doesn't solve the default case

#### Option 4: Hybrid approach
1. Check local git tap (homebrew/core and custom taps)
2. Fall back to API for packages not in any tap
3. Optionally verify API responses against tap data

**Pros:**
- Best of both worlds
- Handles edge cases

**Cons:**
- More complex implementation

### Fix Implemented

Implemented **Option 1**: Extended tap-reading logic to check homebrew/core's local tap before calling the API.

**Changes made:**
1. Added `get_core_formula_version()` function in `src/tap.rs:377-414`
2. Enhanced `parse_formula_version()` to parse versions from URLs (supports GitHub tags, archives, npm packages, and standard tarballs)
3. Modified `src/commands/install.rs:706-714` to check homebrew/core tap before API fallback
4. Modified `src/commands/list.rs:497-518` to check homebrew/core tap before API fallback

**Supported URL patterns:**
- GitHub tags: `/tags/v1.2.3.tar.gz` or `/tags/@scope/package@1.2.3.tar.gz`
- GitHub archives: `/archive/v1.2.3.tar.gz`
- Standard tarballs: `/{name}-{version}.tar.xz`
- npm registry: `https://registry.npmjs.org/.../package-1.2.3.tgz`

**Testing:**
Verified with all 7 previously missed packages (mise, uv, gemini-cli, biome, difftastic, git, opencode) - all now correctly detected.
