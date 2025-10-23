# Testing Quality Issues - Post-Mortem

## The Cleanup Bug That Slipped Through

### What Happened
`bru cleanup` had a **critical data loss bug** that deleted the NEWEST package versions instead of old ones.

Example:
- User has: jq 1.7.0 (old) and jq 1.8.1 (new)
- `bru cleanup` would:
  - ❌ DELETE 1.8.1 (the version they want!)
  - ✅ KEEP 1.7.0 (the old garbage)

### Why Our Tests Missed It

#### Current Test Coverage (98 total tests)
- **70 unit tests** - Test isolated functions, not commands
- **14 regression tests** - Only check "didn't crash"
- **8 integration tests** - IGNORED in CI
- **6 cleanup tests** - JUST ADDED (would have caught it)

#### What the "cleanup test" actually checked:
```rust
fn test_cleanup_dry_run() {
    let output = Command::new(bru_bin())
        .args(["cleanup", "--dry-run"])
        .output()
        .expect("Failed to run bru cleanup --dry-run");

    assert!(output.status.success());  // ❌ Only checks: didn't crash
    assert!(stdout.contains("Dry run")); // ❌ Only checks: printed something
}
```

**Missing assertions:**
- ✅ Command runs → TESTED
- ❌ **Keeps CORRECT version** → NOT TESTED
- ❌ **Removes CORRECT version** → NOT TESTED
- ❌ **Version comparison logic** → NOT TESTED
- ❌ **Sort order** → NOT TESTED

## Root Causes

### 1. **Shallow Testing Philosophy**
We tested for **"doesn't crash"** not **"does the right thing"**

```
Bad:  assert!(command.success())
Good: assert_eq!(kept_version, "1.8.1")
      assert_eq!(removed_version, "1.7.0")
```

### 2. **No Behavior Verification**
Tests verify structure, not semantics:
- ✅ "cleanup printed output"
- ❌ "cleanup kept the right version"

### 3. **Integration Tests Ignored**
```toml
[[test]]
name = "integration_tests"
harness = true
# ⚠️  Not run in CI - requires actual Homebrew installation
```

### 4. **False Sense of Security**
"92 tests passing" sounds great, but:
- 70 are trivial (test string formatting, percentage calc)
- 14 just check exit codes
- 8 are ignored
- **Real command logic: ~10 tests**

## What We Should Have Tested

### For Cleanup Command

#### Test 1: Version Ordering (MISSING - would have caught bug!)
```rust
#[test]
fn test_cleanup_keeps_newest_version() {
    // Create 3 versions: 1.6.0, 1.7.0, 1.8.1
    // Run cleanup
    // Assert: 1.8.1 still exists
    // Assert: 1.7.0 and 1.6.0 removed
}
```

#### Test 2: Version Comparison Edge Cases (MISSING!)
```rust
#[test]
fn test_version_10_vs_9() {
    // Bug: lexicographic "1.9.0" > "1.10.0"
    // Correct: numeric 1.10.0 > 1.9.0
}
```

#### Test 3: Filesystem Order Independence (MISSING!)
```rust
#[test]
fn test_cleanup_regardless_of_inode_order() {
    // The bug: assumed fs::read_dir()[0] is latest
    // Reality: fs::read_dir() order is random (inode order)
}
```

## Similar Bugs Likely Lurking

Commands we haven't properly tested for correctness:

### 1. `bru upgrade`
- ✅ Tests it runs
- ❌ Tests it upgrades to CORRECT version
- ❌ Tests it handles version conflicts

### 2. `bru reinstall`
- ✅ Tests it runs
- ❌ Tests old version removed
- ❌ Tests new version linked correctly

### 3. `bru autoremove`
- ✅ Tests it runs
- ❌ Tests it only removes unused deps
- ❌ Tests it doesn't remove required deps

## Lessons Learned

### 1. Test Behavior, Not Just Execution
```rust
// BAD
assert!(cleanup().is_ok());

// GOOD
let kept = get_installed_versions("jq");
assert_eq!(kept.len(), 1);
assert_eq!(kept[0], "1.8.1");  // The newest
```

### 2. Test Edge Cases That Matter
- Version 1.10 vs 1.9 (lexicographic trap)
- Empty inputs
- Single version (nothing to clean)
- **Filesystem order independence**

### 3. Property-Based Testing
```rust
// For ANY set of versions...
#[quickcheck]
fn cleanup_always_keeps_newest(versions: Vec<Version>) {
    let result = cleanup(versions.clone());
    assert_eq!(result.kept, versions.iter().max());
}
```

### 4. Integration Tests Must Run
Either:
- Make them work in CI (mock Homebrew)
- Or run them locally as part of release checklist

## Action Items

### Immediate (Next PR)
- [x] Add cleanup_tests.rs with version comparison tests
- [ ] Add tests for upgrade version selection
- [ ] Add tests for reinstall version handling
- [ ] Document "must test behavior, not just execution"

### Short Term
- [ ] Review all commands for untested critical paths
- [ ] Add property-based tests (quickcheck crate)
- [ ] Create "behavior test" template

### Long Term
- [ ] Integration test strategy (Docker with Homebrew?)
- [ ] Test coverage tool (tarpaulin)
- [ ] Pre-release manual test checklist

## Metrics

### Before This Bug
- Tests: 92
- Lines of code: ~7000
- Coverage: Unknown (probably ~40%)
- Critical bugs caught: 0

### After This Bug
- Tests: 98 (+6 cleanup tests)
- Coverage: Still unknown
- **Critical bugs caught by new tests: 1** (would have caught cleanup bug)

## The Real Problem

**We optimized for "tests passing" not "tests catching bugs"**

This is like:
- Security theater at airports (looks secure, isn't)
- Code coverage goals (100% coverage, 0% effectiveness)

The cleanup bug was **trivial to test** but we never wrote a test that would catch it.
