# Homebrew Edge Cases & Testing Strategy

## Critical Edge Cases to Test

### 1. **Bottle Platform Variants**

**Issue Found**: Packages may have different bottle types:
- Platform-specific: `arm64_sequoia`, `arm64_sonoma`, `x86_64_linux`
- Platform-independent: `"all"` (e.g., yarn, node packages)
- Missing for newer OS versions

**Solution Implemented**:
- Platform fallback chain: `sequoia → sonoma → ventura → all`
- Graceful degradation to older macOS versions
- Always try `"all"` as final fallback

**Test Cases**:
```bash
# Platform-independent bottles
bru i yarn           # Uses "all" bottle
bru i node           # Uses "all" bottle

# Platform-specific bottles
bru i tmux           # ARM/x86 specific
bru i rust           # Architecture-specific

# Missing bottles (should fail gracefully)
bru i <some-obscure-package>
```

### 2. **Version String Formats**

**Issue Found**: Multiple version string formats in Homebrew:
- Simple: `3.5a`
- With bottle revision: `1.4.0_32`
- Keg-only versions: `openssl@3`, `python@3.12`

**Solution Implemented**:
- Strip bottle revision suffix when comparing versions
- Handle `@` versioned packages

**Test Cases**:
```bash
# Bottle revisions
bru outdated         # Should handle _XX suffix correctly

# Keg-only versions
bru i openssl@3
bru i python@3.12

# Multiple versions installed
bru i rust
bru upgrade rust     # Should detect current version correctly
```

### 3. **Dependency Edge Cases**

**Potential Issues**:
- Circular dependencies
- Optional dependencies
- Build-only dependencies
- Recommended dependencies

**Test Cases**:
```bash
# Complex dependency trees
bru deps vim --tree
bru deps postgresql --tree

# Dependency conflicts
bru i <package-with-conflicts>
```

### 4. **Tap/Cask Variants**

**Potential Issues**:
- Formula vs Cask with same name
- Third-party taps
- Tap name formats

**Test Cases**:
```bash
# Formula vs Cask conflicts
bru search docker
bru i docker --formula
bru i docker --cask

# Third-party taps
bru tap homebrew/cask-versions
bru i firefox@developer-edition
```

### 5. **Installation States**

**Potential Issues**:
- Multiple versions installed
- Linked vs unlinked
- Pinned packages
- Keg-only packages

**Test Cases**:
```bash
# Multiple versions
bru list --versions

# Pinned packages
bru pin rust
bru upgrade          # Should skip rust

# Keg-only
bru i openssl@3      # Keg-only, won't link to /usr/local
bru link openssl@3   # Should fail or warn
```

### 6. **Artifact Types**

**Issue**: Different packages ship different artifact types:
- `.tar.gz` bottles (standard)
- Binary artifacts in bottles
- Suites (multiple binaries)
- App bundles (.app for casks)

**Test Cases**:
```bash
# Standard bottle
bru i tmux

# Cask with .app
bru i --cask firefox

# Package with suite artifacts
bru i postgresql     # Has multiple binaries
```

### 7. **Error Scenarios**

**Potential Issues**:
- Network failures during download
- Checksum mismatches
- Disk space issues
- Permission errors

**Test Cases**:
```bash
# Network interruption (manual test)
# Kill network during download

# Checksum verification
# Corrupt cached file and reinstall

# Permission errors
# Remove write permissions from Cellar
```

## Testing Matrix

### Quick Smoke Tests (Run Before Release)
```bash
# Core operations
bru update
bru search rust
bru i tmux
bru upgrade
bru rm tmux
bru outdated

# Edge cases
bru i yarn           # "all" bottle
bru i openssl@3      # Keg-only
bru i rust           # Large package
```

### Comprehensive Test Suite (Weekly/Major Changes)

**Platform Coverage**:
- [ ] macOS ARM64 (Sequoia)
- [ ] macOS ARM64 (Sonoma)
- [ ] macOS x86_64
- [ ] Linux ARM64
- [ ] Linux x86_64

**Package Types**:
- [ ] Simple package (tmux)
- [ ] Platform-independent (yarn)
- [ ] Large package (rust, postgresql)
- [ ] Keg-only (openssl@3)
- [ ] Versioned (python@3.12)
- [ ] With dependencies (vim)
- [ ] Cask (.app bundle)

**Operations**:
- [ ] Install
- [ ] Upgrade
- [ ] Reinstall
- [ ] Uninstall
- [ ] Pin/Unpin
- [ ] Link/Unlink

### Parity Tests (Ensure Homebrew Compatibility)

For each command, compare output:
```bash
brew <command> <args> > brew.out
bru <command> <args> > bru.out
diff brew.out bru.out
```

Test commands:
- `search`, `info`, `list`, `outdated`
- `deps`, `uses`
- Exit codes should match

## Automated Testing Strategy

### Unit Tests
- [ ] Platform detection fallback logic
- [ ] Version string parsing
- [ ] Bottle revision stripping
- [ ] Dependency resolution

### Integration Tests
- [ ] Download with fallback platforms
- [ ] Install with various bottle types
- [ ] Upgrade detection with multiple versions
- [ ] Symlink creation/removal

### Regression Tests
- [ ] All fixed bugs documented as test cases
- [ ] Run on CI for every PR

## Homebrew Compatibility Checks

### Before Each Release
1. **Check Homebrew API Changes**
   - Visit https://formulae.brew.sh/docs/api/
   - Verify JSON schema hasn't changed
   - Test with latest formulae

2. **Test Platform Support**
   ```bash
   # Check which platforms Homebrew officially supports
   curl -s https://formulae.brew.sh/api/formula.json | \
     jq -r '.[0].bottle.stable.files | keys[]' | sort -u
   ```

3. **Verify Bottle Naming**
   - Check if new OS versions added
   - Update platform fallback chain

4. **Test Problematic Packages**
   - Packages known to have issues
   - Large packages (rust, llvm)
   - Complex dependencies (vim, postgresql)
   - Keg-only packages (openssl, python)

## Monitoring in Production

### Metrics to Track
- Install success rate by package
- Platform fallback usage (how often?)
- Most common errors
- Packages that fail frequently

### User Reports
- Create issue template for installation failures
- Ask for: package name, platform, error message, `--verbose` output

## Known Homebrew Quirks

### Documented Behaviors
1. **Keg-only packages** don't link to /usr/local by default
2. **Multiple versions** can coexist but only one is linked
3. **Bottle revisions** increment when rebuilt without version change
4. **Pinned packages** are excluded from upgrades
5. **"all" bottles** work on any platform (usually Node/JS packages)
6. **Taps** are just git repos in `/opt/homebrew/Library/Taps`

### Potential Gotchas
- Homebrew changes JSON API schema occasionally
- New macOS releases may not have bottles immediately
- Some formulae are deprecated/migrated
- Cask tokens may differ from app names
- Dependencies can change between versions

## Future Proofing

### When New macOS Version Releases
1. Update `macos_name()` function in `src/platform.rs`
2. Add new fallback in `get_platform_fallbacks()`
3. Test with packages that don't have bottles yet
4. Document expected failures

### API Changes
1. Monitor https://github.com/Homebrew/brew for API changes
2. Add API version check if schema changes
3. Maintain backwards compatibility when possible
