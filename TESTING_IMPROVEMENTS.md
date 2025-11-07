# Testing Improvements Needed

## Critical Gaps Identified from v0.1.30 Bug

### What We Missed

The autoremove bug (v0.1.18-0.1.30) existed for weeks and wasn't caught by tests because:

1. **No integration tests for command sequences**
   - Test: install → verify receipt
   - Test: upgrade → verify receipt has dependencies
   - Test: upgrade → autoremove → verify nothing incorrectly removed

2. **No receipt validation tests**
   - Current: Tests only check command execution
   - Missing: Verify INSTALL_RECEIPT.json content
   - Missing: Verify runtime_dependencies field is populated

3. **No behavior verification tests**
   - Current: Test that command runs without error
   - Missing: Test that command produces correct results
   - Example: autoremove test should verify it DOESN'T remove required deps

4. **No regression tests for critical bugs**
   - v0.1.18-19: Broken bottle relocation (binaries crashed)
   - v0.1.18-30: Empty runtime_dependencies in receipts
   - Neither had regression tests added after fixes

## Recommended Test Additions

### Priority 0 (Critical)

**Test 1: Receipt Generation After Upgrade**
```rust
#[tokio::test]
async fn test_upgrade_generates_correct_receipts() {
    // 1. Install a package with dependencies (e.g., wget)
    // 2. Upgrade the package
    // 3. Read INSTALL_RECEIPT.json
    // 4. Verify runtime_dependencies is NOT empty
    // 5. Verify dependencies match expected list
}
```

**Test 2: Autoremove Doesn't Remove Required Deps**
```rust
#[test]
fn test_autoremove_preserves_required_dependencies() {
    // 1. Set up: Install package A that depends on B
    // 2. Mark A as installed_on_request, B as dependency
    // 3. Run autoremove
    // 4. Verify B is NOT in removal list
    // 5. Verify only truly unused packages are listed
}
```

**Test 3: Upgrade → Autoremove Sequence**
```rust
#[tokio::test]
async fn test_upgrade_then_autoremove_sequence() {
    // 1. Install old version of package with dependencies
    // 2. Upgrade package (dependencies may change)
    // 3. Run autoremove
    // 4. Verify new dependencies NOT removed
    // 5. Verify old dependencies removed only if truly unused
}
```

### Priority 1 (High)

**Test 4: Receipt Validation Helper**
```rust
fn verify_receipt_structure(receipt_path: &Path) -> Result<()> {
    let receipt = InstallReceipt::read(receipt_path)?;

    // Verify required fields
    assert!(!receipt.homebrew_version.is_empty());
    assert!(receipt.time > 0);

    // For packages with dependencies, verify populated
    if has_dependencies(receipt_path.parent().unwrap()) {
        assert!(!receipt.runtime_dependencies.is_empty(),
            "Receipt has empty runtime_dependencies but package has deps");
    }

    Ok(())
}
```

**Test 5: Dependency Resolution Consistency**
```rust
#[tokio::test]
async fn test_dependency_resolution_matches_across_commands() {
    // Verify install, upgrade, reinstall all resolve deps the same way
    let deps_from_install = resolve_for_install("wget").await?;
    let deps_from_upgrade = resolve_for_upgrade("wget").await?;
    let deps_from_reinstall = resolve_for_reinstall("wget").await?;

    assert_eq!(deps_from_install, deps_from_upgrade);
    assert_eq!(deps_from_install, deps_from_reinstall);
}
```

### Priority 2 (Medium)

**Test 6: Bottle Execution Verification** (Prevents v0.1.18-19 bug)
```rust
#[tokio::test]
#[ignore] // Expensive - requires actual bottle download
async fn test_installed_binary_executes() {
    // 1. Install a simple package (e.g., bat)
    // 2. Extract binary path from receipt
    // 3. Execute binary with --version
    // 4. Verify exit code 0 (not SIGKILL)
    // 5. Verify output contains version string
}
```

**Test 7: Symlink Integrity After Upgrade**
```rust
#[tokio::test]
async fn test_symlinks_valid_after_upgrade() {
    // 1. Install package
    // 2. Record all symlinks created
    // 3. Upgrade package
    // 4. Verify all symlinks still valid (not broken)
    // 5. Verify symlinks point to new version
}
```

## Testing Strategy Improvements

### Current Approach
- ✅ Unit tests for individual functions
- ✅ Regression tests with --dry-run
- ❌ No integration tests for command sequences
- ❌ No receipt content validation
- ❌ No actual binary execution tests

### Proposed Approach

**1. Test Pyramid**
```
       /\        E2E Tests (few, expensive)
      /  \       - Actual bottle install + execution
     /    \      - Full upgrade → autoremove sequence
    /------\
   /        \    Integration Tests (moderate)
  / Integ.  \   - Command sequences with receipt validation
 /------------\  - Dependency resolution across commands
/              \
/  Unit Tests  \ Unit Tests (many, fast)
/________________\ - Function-level logic
                   - Edge cases, error handling
```

**2. Behavior Verification Pattern**
```rust
// BAD: Only tests execution
#[test]
fn test_autoremove() {
    let result = autoremove(true);
    assert!(result.is_ok()); // Only checks no crash
}

// GOOD: Tests correctness
#[test]
fn test_autoremove_correctness() {
    let result = autoremove(true)?;
    let to_remove = result.packages_to_remove;

    // Verify behavior
    for pkg in &to_remove {
        assert!(!pkg.installed_on_request, "Can't remove requested pkg");
        assert!(!is_required_by_any(pkg), "Can't remove required dep");
    }
}
```

**3. Regression Test Template**
When a critical bug is found:
```rust
#[test]
fn test_regression_issue_XXX() {
    // 1. Describe the bug
    // 2. Reproduce the scenario
    // 3. Verify the bug is fixed
    // 4. Include issue/commit reference
}
```

## Implementation Plan

### Phase 1 (This Week)
- [ ] Add receipt validation tests (Test 1, 4)
- [ ] Add autoremove correctness test (Test 2)
- [ ] Add upgrade → autoremove integration test (Test 3)

### Phase 2 (Next Week)
- [ ] Add dependency resolution consistency test (Test 5)
- [ ] Add symlink integrity test (Test 7)
- [ ] Create test fixtures for common scenarios

### Phase 3 (Future)
- [ ] Add E2E bottle execution tests (expensive, run in CI only)
- [ ] Add performance regression tests
- [ ] Add fuzzing for edge cases

## Test Fixtures Needed

```rust
// fixtures/packages/
// - wget/ (simple package with deps)
// - llvm/ (large package, many files)
// - python@3.13/ (versioned formula)
// - formula-with-no-bottle/ (source build scenario)

// fixtures/receipts/
// - valid_receipt.json
// - receipt_with_deps.json
// - receipt_without_deps.json
// - malformed_receipt.json
```

## Success Metrics

**Before:**
- 76 unit tests
- 0 integration tests
- 0 receipt validation tests
- Missed: Critical autoremove bug (weeks undetected)

**Target:**
- 76 unit tests (maintain)
- 10+ integration tests
- Receipt validation in all install/upgrade/reinstall tests
- Catch: Similar bugs within hours, not weeks

## References

- Homebrew testing: `test/` directory in homebrew/brew
- Rust testing best practices: https://doc.rust-lang.org/book/ch11-00-testing.html
- Integration testing patterns: https://matklad.github.io/2021/05/31/how-to-test.html
