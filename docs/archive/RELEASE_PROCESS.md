# Release Process for kombrucha

**CRITICAL: Follow this checklist for every release!**

## Pre-Release Checklist

- [ ] All tests passing locally (`cargo test`)
- [ ] All critical bugs fixed
- [ ] Documentation updated (README.md, ai/STATUS.md)
- [ ] No uncommitted changes

## Step-by-Step Release Process

### 1. Bump Version
```bash
# Update version in Cargo.toml
# Update ai/STATUS.md: move "Unreleased" section to new version heading

git add -u
git commit -m "chore: bump version to X.Y.Z"
git push
```

### 2. Wait for CI ⚠️ CRITICAL
```bash
# NEVER tag before CI passes!
gh run watch

# Or check manually:
gh run list --limit 1

# Wait until you see:
# ✓ Build (Linux)
# ✓ Test
# ✓ Lint
# ✓ Integration Tests
```

**If CI fails**: Fix issues, commit, push, wait again. Do NOT proceed to tagging.

### 3. Tag Release
```bash
git tag -a vX.Y.Z -m "vX.Y.Z - Brief description

Key changes:
- Feature/fix 1
- Feature/fix 2
- Feature/fix 3"

git push --tags
```

### 4. Create GitHub Release
```bash
gh release create vX.Y.Z --notes "## vX.Y.Z - Release Title

### Key Changes
- Feature/fix 1
- Feature/fix 2

### Installation
\`\`\`bash
cargo install kombrucha
# or upgrade
cargo install --force kombrucha
\`\`\`"
```

**Note**: The tag push triggers `.github/workflows/release.yml` which builds binaries for:
- macOS ARM64
- macOS x86_64
- Linux x86_64

### 5. Publish to crates.io
```bash
cargo publish
```

**If error "Cargo.lock not committed"**:
```bash
git add Cargo.lock
git commit -m "chore: update Cargo.lock for vX.Y.Z"
git push
cargo publish
```

### 6. Update Homebrew Tap ⚠️ OFTEN FORGOTTEN

```bash
# 1. Calculate SHA256 for new tarball
SHA=$(curl -sL https://github.com/nijaru/kombrucha/archive/refs/tags/vX.Y.Z.tar.gz | shasum -a 256 | cut -d' ' -f1)
echo $SHA

# 2. Update tap formula
cd /opt/homebrew/Library/Taps/nijaru/homebrew-tap

# Edit Formula/bru.rb:
# - Update url to vX.Y.Z
# - Update sha256 to $SHA

# 3. Commit and push
git add Formula/bru.rb
git commit -m "chore: update bru to vX.Y.Z

Brief description of changes"
git push

# 4. Verify
brew update
brew info nijaru/tap/bru  # Should show vX.Y.Z
```

### 7. Post-Release Verification

```bash
# Test installation from crates.io
cargo install --force kombrucha
bru --version  # Should show vX.Y.Z

# Test installation from tap
brew upgrade nijaru/tap/bru
bru --version  # Should show vX.Y.Z
```

## Troubleshooting

### CI Fails After Tagging
```bash
# Delete tag locally and remotely
git tag -d vX.Y.Z
git push --delete origin vX.Y.Z

# Fix issues, restart from Step 1
```

### Homebrew Tap Not Updating
```bash
# Force update
brew update --force
brew tap --repair

# Check tap repo
cd /opt/homebrew/Library/Taps/nijaru/homebrew-tap
git status
git pull
```

### Wrong SHA256
```bash
# Recalculate
curl -sL https://github.com/nijaru/kombrucha/archive/refs/tags/vX.Y.Z.tar.gz | shasum -a 256

# Must match the GitHub release tarball exactly
```

## Release Checklist Summary

- [ ] 1. Bump version in Cargo.toml and STATUS.md
- [ ] 2. Commit and push
- [ ] 3. **WAIT FOR CI TO PASS** ✅
- [ ] 4. Tag release
- [ ] 5. Create GitHub release
- [ ] 6. Publish to crates.io
- [ ] 7. **UPDATE HOMEBREW TAP** (often forgotten!)
- [ ] 8. Verify installations work

## Version Numbering

Follow semantic versioning (MAJOR.MINOR.PATCH):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backwards compatible)
- **PATCH**: Bug fixes

**No drastic jumps**: 0.1.8 → 0.1.9 → 0.1.10 (not 0.1.8 → 1.0.0)

Use 1.0.0 only when:
- Production-ready
- Stable API
- Well-tested
- Ready for public use

## Common Mistakes to Avoid

1. ❌ **Tagging before CI passes** - Always wait for CI ✅
2. ❌ **Forgetting to update tap** - Users can't install latest version
3. ❌ **Wrong SHA256** - Homebrew installation will fail
4. ❌ **Not testing installation** - Release might be broken
5. ❌ **Skipping STATUS.md update** - Lost history of changes
