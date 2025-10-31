# Bug: bru fails to install from custom taps

## Status: FIXED ✅

**Fixed in:** Pending release
**Fix:** Detect tap-prefixed formulas and delegate to brew (src/commands.rs:22-25, 1169-1209)

## Issue
`bru install nijaru/tap/sy` fails with error:
```
Error: nijaru/tap/sy dependencies not built for the arm64 CPU architecture:
  z3 was built for aarch64

==> Fetching downloads for: sy
```

After fixing z3 issue:
```
Installing 1 formulae...
Resolving dependencies...
✗ nijaru/tap/sy: Formula not found: nijaru/tap/sy
Error: All formulae failed to install
```

## Root Cause
`bru` cannot properly resolve formulas from custom taps, while real Homebrew works correctly.

## Evidence

### Real Homebrew works:
```bash
$ /opt/homebrew/bin/brew search nijaru/tap/
nijaru/tap/bru
nijaru/tap/sy

$ /opt/homebrew/bin/brew install nijaru/tap/sy
==> Installing sy from nijaru/tap
...
🍺  /opt/homebrew/Cellar/sy/0.0.55: 10 files, 13.7MB, built in 51 seconds
```

### bru fails:
```bash
$ bru install nijaru/tap/sy
✗ nijaru/tap/sy: Formula not found: nijaru/tap/sy
Error: All formulae failed to install
```

## Tap Configuration (verified correct)
- **Tap URL**: https://github.com/nijaru/homebrew-tap
- **Formula location**: `Formula/sy.rb`
- **Formula accessible**: ✅ https://raw.githubusercontent.com/nijaru/homebrew-tap/main/Formula/sy.rb
- **Tap installed**: ✅ `brew tap-info nijaru/tap` shows it's tapped
- **Formulas visible to real brew**: ✅ `brew search nijaru/tap/` finds both `bru` and `sy`

## Expected Behavior
`bru install nijaru/tap/sy` should:
1. Resolve the formula from the tapped `nijaru/tap` repository
2. Install dependencies
3. Build and install `sy` version 0.0.55

## Actual Behavior
`bru` reports "Formula not found" even though:
- The tap is installed
- Real Homebrew can see the formula
- The formula file exists and is valid

## Workaround
Use real Homebrew for custom tap installations:
```bash
/opt/homebrew/bin/brew install nijaru/tap/sy
```

Or install via cargo:
```bash
cargo install sy
```

## Investigation Needed

### 1. Formula Resolution
Check how `bru` resolves formulas from custom taps:
- Does it read from tap directories correctly?
- Does it handle tap prefix (`nijaru/tap/`) properly?
- Is there a cache issue?

### 2. Tap Path Handling
Verify `bru` uses correct tap directories:
```
/opt/homebrew/Library/Taps/nijaru/homebrew-tap/Formula/sy.rb
```

### 3. Dependency Resolution
The initial error mentioned z3 architecture mismatch:
```
nijaru/tap/sy dependencies not built for the arm64 CPU architecture:
  z3 was built for aarch64
```
This suggests `bru` may be checking dependencies but failing formula lookup.

## Debug Commands

```bash
# Check tap status
brew tap-info nijaru/tap

# List formulas in tap
brew search nijaru/tap/

# Check if formula file exists locally
ls -la $(brew --repository)/Library/Taps/nijaru/homebrew-tap/Formula/sy.rb

# Compare bru vs brew behavior
bru search nijaru/tap/     # What does this show?
/opt/homebrew/bin/brew search nijaru/tap/  # Shows: nijaru/tap/bru, nijaru/tap/sy

# Try direct formula URL (bypass tap resolution)
bru install https://raw.githubusercontent.com/nijaru/homebrew-tap/main/Formula/sy.rb
```

## Potential Fixes

### Option 1: Fix Formula Resolution
Ensure `bru` searches tapped directories when resolving `tap/formula` names:
1. Parse tap prefix (`nijaru/tap`)
2. Look in `/opt/homebrew/Library/Taps/nijaru/homebrew-tap/Formula/`
3. Load and parse the formula file

### Option 2: Improve Error Messages
If `bru` doesn't support custom taps yet, fail fast with a clear message:
```
Error: bru does not yet support installing from custom taps.
Use: /opt/homebrew/bin/brew install nijaru/tap/sy
Or: cargo install sy
```

### Option 3: Delegate to Homebrew
For custom tap installs, `bru` could delegate to real Homebrew:
```rust
if is_custom_tap(formula_name) {
    exec!("/opt/homebrew/bin/brew", "install", formula_name);
}
```

## Test Case
```bash
# Setup
brew tap nijaru/tap

# Should work (currently fails)
bru install nijaru/tap/sy

# Expected:
# - Resolves sy formula from nijaru/tap
# - Installs dependencies (rust, z3)
# - Builds sy from source
# - Installs to /opt/homebrew/Cellar/sy/0.0.55/
```

## Fix Implementation

### Root Cause
`api.fetch_formula()` only queries the Homebrew API (https://formulae.brew.sh/api) which contains core formulas only. Custom tap formulas exist only as local Ruby files in tap directories.

### Solution
Added tap formula detection in `install` command (src/commands.rs):

1. **Added `is_tap_formula()` helper (line 22-25)**:
   - Detects tap-prefixed names by counting slashes (≥2 = tap formula)
   - Example: "nijaru/tap/sy" has 2 slashes → tap formula

2. **Modified `install()` function (lines 1169-1209)**:
   - Separate tap formulas from core formulas at start
   - Delegate tap formulas to brew (they typically need source builds)
   - Continue with core formulas via fast bottle installation

### Why This Approach
- Tap formulas typically don't have bottles → need source builds
- bru's design: fast bottles for core, delegate source builds to brew
- Clean separation: tap handling doesn't complicate core formula logic
- Supports mixed installs: `bru install jq nijaru/tap/sy`

### Test Results
```bash
# Tap formula only
$ bru install nijaru/tap/sy
Installing 1 formulae...
  ℹ nijaru/tap/sy is from a custom tap - delegating to brew
  ✓ nijaru/tap/sy installed successfully

# Mixed tap + core
$ bru install jq nijaru/tap/sy
Installing 2 formulae...
  ℹ nijaru/tap/sy is from a custom tap - delegating to brew
  ✓ nijaru/tap/sy installed successfully
  ✓ Installed 1 packages (jq via bottles)
```

## Related Code
Fixed in `bru` source:
- src/commands.rs:22-25 - `is_tap_formula()` helper
- src/commands.rs:1169-1209 - Tap formula handling in `install()`

## Priority
**Medium-High** - Custom taps are a core Homebrew feature. Users expect `bru` to be a drop-in replacement.

## Notes
- Issue discovered during `sy` v0.0.55 release
- Real Homebrew works perfectly with the same tap/formula
- Tap configuration is verified correct (works with official Homebrew)
- Both formulas in the tap (`bru` and `sy`) have the same issue
- **Fixed:** Tap formulas now properly detected and delegated to brew
