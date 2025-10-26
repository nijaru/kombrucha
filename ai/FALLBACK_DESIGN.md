# Brew Fallback Design

## Decision: Use brew fallback instead of Ruby interop

**Rationale:**
- Simpler (30 min vs 10-16 days)
- Works perfectly long-term (same Cellar, compatible receipts)
- True drop-in replacement (bru install anything always works)
- No complexity, no bugs, no maintenance burden

## Commands with Fallback

### install
**Trigger:** When formula has no bottle available

**Behavior:**
```
$ bru install some-source-only-package

Resolving dependencies...
✓ 3 dependencies already installed
→ 1 formula to install: some-source-only-package

Downloading bottles...
ℹ some-source-only-package requires building from source (no bottle available)
  Falling back to brew install...

[brew's build output...]

✓ Installed some-source-only-package via brew
```

**Implementation:**
```rust
// In install command, when downloading bottles:
let mut brew_fallback_needed = Vec::new();

for formula in &to_install {
    if formula.bottle.is_none() || /* no stable bottle */ {
        brew_fallback_needed.push(formula.name.clone());
    }
}

// Install via brew
for name in &brew_fallback_needed {
    println!(
        "\n{} {} requires building from source (no bottle available)",
        "ℹ".blue(),
        name.bold()
    );
    println!("  Falling back to {}...", "brew install".cyan());

    let status = Command::new("brew")
        .arg("install")
        .arg(name)
        .status()?;

    if status.success() {
        println!("  {} Installed {} via brew", "✓".green(), name.bold());
    } else {
        println!("  {} Failed to install {} via brew", "✗".red(), name.bold());
    }
}

// Continue with bottle-based installs for the rest...
```

### upgrade
**Trigger:** When upgrading to a version without bottle

**Behavior:**
```
$ bru upgrade some-package

Checking for upgrades...
→ 1 package to upgrade: some-package (1.0.0 → 2.0.0)

ℹ some-package 2.0.0 requires building from source (no bottle available)
  Falling back to brew upgrade...

[brew's build output...]

✓ Upgraded some-package to 2.0.0 via brew
```

**Implementation:**
```rust
// In upgrade command, when checking for bottles:
if formula.bottle.is_none() || /* no bottle for new version */ {
    println!(
        "\n{} {} {} requires building from source (no bottle available)",
        "ℹ".blue(),
        formula.name.bold(),
        new_version
    );
    println!("  Falling back to {}...", "brew upgrade".cyan());

    let status = Command::new("brew")
        .arg("upgrade")
        .arg(&formula.name)
        .status()?;

    if status.success() {
        println!("  {} Upgraded {} to {} via brew", "✓".green(), formula.name.bold(), new_version);
    }
    continue;
}

// Regular bottle-based upgrade for this package...
```

### reinstall
**Trigger:** When formula has no bottle

**Behavior:**
Same as install - falls back to `brew reinstall`

**Implementation:**
```rust
// Same pattern as install
if formula.bottle.is_none() {
    println!("  Falling back to {}...", "brew reinstall".cyan());
    Command::new("brew").arg("reinstall").arg(name).status()?;
}
```

## Error Message Improvements

### Current (Unclear):
```
⚠ No bottle available for X
⚠ Skipping X (no bottle)
```

**Problems:**
- Doesn't explain it's a bru limitation
- Doesn't say what to do
- User might think it's broken

### With Fallback (Clear):
```
ℹ X requires building from source (no bottle available)
  Falling back to brew install...
[brew output]
✓ Installed X via brew
```

**Benefits:**
- Clear explanation
- Automatic solution
- Seamless experience

## Compatibility

### Receipts
Both brew and bru write compatible `INSTALL_RECEIPT.json`:
- Same format (JSON)
- Same location (`Cellar/formula/version/INSTALL_RECEIPT.json`)
- bru writes `homebrew_version: "bru/0.1.10"`
- brew writes `homebrew_version: "4.x.x"`
- Neither cares about the other's version string

### Cellar
- Both use `/opt/homebrew/Cellar/`
- Same directory structure
- bru can read brew-installed packages
- brew can read bru-installed packages

### Symlinks
- Both link to `/opt/homebrew/bin/`
- Compatible symlink structure
- No conflicts

### Commands
All bru commands work with brew-installed packages:
- `bru list` shows brew-installed packages
- `bru outdated` checks brew-installed packages
- `bru upgrade` can upgrade brew-installed packages (with fallback)
- `bru uninstall` can uninstall brew-installed packages
- `bru cleanup` can clean up brew-installed packages

## Edge Cases

### Mixed installations
**Scenario:** Package installed via bru (bottle), later upgraded via brew (source)

**Result:** ✅ Works fine
- Both write receipts in same location
- Latest receipt wins
- bru commands continue to work

### Dependencies
**Scenario:** Main package has bottle, but dependency doesn't

**Result:** ✅ Works with fallback
- Dependency installed via brew fallback
- Main package installed via bru bottle
- Both in same Cellar
- All commands work

### Cleanup
**Scenario:** Multiple versions, some from bru, some from brew

**Result:** ✅ Works fine
- cleanup looks at version numbers only
- Doesn't care how they were installed
- Keeps newest, removes oldest (as fixed in v0.1.10)

## Long-Term Viability

**Question:** Should we implement Ruby interop eventually?

**Answer:** NO - Fallback is the right long-term solution

**Why:**
1. **Simplicity** - 30 minutes vs 10-16 days of work
2. **Maintenance** - Zero complexity vs Ruby/Rust interop bugs
3. **Compatibility** - Perfect, proven by shared Cellar
4. **User experience** - Seamless, works for everything
5. **Coverage** - ~95% via bottles (fast), ~5% via brew (works)

**Only reason to NOT use fallback:** If brew isn't installed. But if you're using a Homebrew-compatible tool, you almost certainly have brew.

## Implementation Checklist

- [ ] Add `check_brew_available()` helper function
- [ ] Update `install` command with fallback
- [ ] Update `upgrade` command with fallback
- [ ] Update `reinstall` command with fallback
- [ ] Improve error messages in all three commands
- [ ] Add tests for fallback behavior
- [ ] Update documentation to mention fallback
- [ ] Test with real source-only packages

## Future Considerations

If we ever wanted to remove brew dependency:
1. Implement source builds (Ruby interop)
2. Keep fallback as backup
3. Only use source builds if brew not available

But this is probably YAGNI (You Aren't Gonna Need It).
