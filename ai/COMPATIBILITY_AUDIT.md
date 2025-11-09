# Homebrew Compatibility Audit Report
**Date**: 2025-01-07
**Context**: Comprehensive audit to find ALL compatibility issues before they break the local environment

## Executive Summary

**Critical Issues**: 2
**High Priority**: 3
**Medium Priority**: 3
**Low Priority**: 2

**Status**: Multiple compatibility issues found that could cause brew/bru interoperability problems.

---

## CRITICAL Issues (Fix Immediately)

### 1. Missing `bottle_rebuild` field in RuntimeDependency
**File**: `src/cellar.rs:37`
**Severity**: CRITICAL
**Impact**: Receipt format incompatibility causes brew to fail reading bru-generated receipts

**Current State**:
```rust
pub struct RuntimeDependency {
    pub full_name: String,
    pub version: String,
    pub revision: u32,
    pub pkg_version: String,
    pub declared_directly: bool,
}
```

**Expected (Homebrew format)**:
```json
{
  "full_name": "protobuf",
  "version": "33.0",
  "revision": 0,
  "bottle_rebuild": 1,  // <-- MISSING
  "pkg_version": "33.0",
  "declared_directly": true
}
```

**Fix Required**:
```rust
pub struct RuntimeDependency {
    pub full_name: String,
    pub version: String,
    pub revision: u32,
    #[serde(default)]
    pub bottle_rebuild: u32,  // ADD THIS
    pub pkg_version: String,
    pub declared_directly: bool,
}
```

**Build function** (src/commands.rs:1601):
```rust
RuntimeDependency {
    full_name: f.name.clone(),
    version: v.clone(),
    revision: 0,
    bottle_rebuild: 0,  // ADD THIS
    pkg_version: v.clone(),
    declared_directly: true,
}
```

### 2. Missing version in receipt `source.versions` field
**File**: `src/receipt.rs:125`
**Severity**: CRITICAL
**Impact**: brew may fail to determine installed version from receipt

**Current State** (line 125):
```rust
source: Some(SourceInfo {
    path: None,
    tap: "homebrew/core".to_string(),
    tap_git_head: None,
    spec: "stable".to_string(),
    versions: None,  // <-- WRONG
}),
```

**Expected (Homebrew format)**:
```json
"source": {
  "spec": "stable",
  "versions": {
    "stable": "12.1.2",
    "head": null,
    "version_scheme": 0
  }
}
```

**Fix Required**:
Add formula version to `new_bottle()` function signature and populate versions:
```rust
pub fn new_bottle(
    formula: &Formula,
    runtime_deps: Vec<RuntimeDependency>,
    installed_on_request: bool,
) -> Self {
    // ...
    source: Some(SourceInfo {
        path: Some("/Users/USER/Library/Caches/Homebrew/api/formula.jws.json".to_string()),
        tap: "homebrew/core".to_string(),
        tap_git_head: None,
        spec: "stable".to_string(),
        versions: Some(SourceVersions {
            stable: formula.versions.stable.clone(),
            head: None,
            version_scheme: 0,
            compatibility_version: None,
        }),
    }),
}
```

---

## HIGH Priority Issues (Fix Before Next Release)

### 3. Reinstall doesn't respect pinned packages
**File**: `src/commands.rs:2040` (reinstall function)
**Severity**: HIGH
**Impact**: Users can accidentally reinstall pinned packages, breaking their intentional version locks

**Current Behavior**:
- `reinstall` function iterates through all requested formulas without checking pins
- No check at line 2040 to filter out pinned packages

**Expected Behavior**:
- Read pinned formulae like upgrade does (line 1750)
- Skip pinned packages with warning message

**Fix Required**:
```rust
pub async fn reinstall(api: &BrewApi, formula_names: &[String], force: bool) -> Result<()> {
    // ADD: Check for pinned formulae
    let pinned = read_pinned()?;

    for formula_name in formula_names {
        // ADD: Skip pinned packages
        if pinned.contains(formula_name) && !force {
            println!(
                "  {} {} is pinned (use --force to reinstall)",
                "⚠".yellow(),
                formula_name.bold()
            );
            continue;
        }
        // ... existing logic
    }
}
```

### 4. Incorrect `homebrew_version` format in receipts
**File**: `src/receipt.rs:106`
**Severity**: HIGH
**Impact**: May break tools that parse homebrew_version field

**Current**:
```rust
homebrew_version: format!("bru/{}", env!("CARGO_PKG_VERSION")),  // "bru/0.1.32"
```

**Homebrew format**: `"4.6.17-37-gc76f378"`

**Issue**:
- Homebrew uses semantic versioning with git info
- Some tools may parse this field expecting Homebrew's format
- The "bru/" prefix may cause parsing issues

**Recommendations**:
1. **Option A (Conservative)**: Use a valid Homebrew-compatible version like `"4.0.0-bru-0.1.32"`
2. **Option B (Transparent)**: Continue using `"bru/0.1.32"` but document the format
3. **Option C (Full Compat)**: Query actual Homebrew version: `brew --version` and use that

**Recommended**: Option C - Use actual Homebrew version for maximum compatibility

### 5. Missing Xcode/CLT detection in receipt built_on field
**File**: `src/receipt.rs:173-174`
**Severity**: HIGH (macOS only)
**Impact**: Some formulae may check these fields for rebuild decisions

**Current** (lines 173-174):
```rust
xcode: None,
clt: None,
```

**Homebrew populates**:
```json
"xcode": "16.3",
"clt": "16.3.0.0.1.1742442376"
```

**Fix Required**:
```rust
fn detect_build_environment() -> Option<BuiltOn> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // ... existing os_version detection ...

        // Detect Xcode version
        let xcode = Command::new("xcodebuild")
            .arg("-version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .next()
                    .and_then(|l| l.split_whitespace().nth(1))
                    .map(|v| v.to_string())
            });

        // Detect CLT version
        let clt = Command::new("pkgutil")
            .args(&["--pkg-info", "com.apple.pkg.CLTools_Executables"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("version:"))
                    .and_then(|l| l.split(':').nth(1))
                    .map(|v| v.trim().to_string())
            });

        Some(BuiltOn {
            os: "Macintosh".to_string(),
            os_version: format!("macOS {}", os_version),
            cpu_family: homebrew_arch().to_string(),
            xcode,
            clt,
            preferred_perl: Some("5.34".to_string()),
        })
    }
}
```

---

## MEDIUM Priority Issues (Fix When Convenient)

### 6. Missing source.path in receipts
**File**: `src/receipt.rs:121`
**Severity**: MEDIUM
**Impact**: Minor - brew may not know where formula metadata came from

**Current**: `path: None`
**Homebrew**: `"path": "/Users/nick/Library/Caches/Homebrew/api/formula.jws.json"`

**Fix**: Detect cache location and populate path

### 7. Missing source.tap_git_head in receipts
**File**: `src/receipt.rs:123`
**Severity**: MEDIUM
**Impact**: Minor - brew can't verify formula version against tap commit

**Current**: `tap_git_head: None`
**Homebrew**: `"tap_git_head": "f857183be195550aed9246019faa03bc21ba12c0"`

**Fix**: Fetch homebrew-core tap HEAD commit hash when writing receipt

### 8. CPU family mismatch in built_on field
**File**: `src/receipt.rs:172`
**Severity**: MEDIUM (cosmetic)
**Impact**: Cosmetic difference - doesn't affect functionality

**Current**:
```rust
cpu_family: homebrew_arch().to_string(),  // "arm64"
```

**Homebrew uses**: `"cpu_family": "dunno"` (literally)

**Note**: This is a Homebrew quirk. Our implementation is actually MORE correct, but brew may expect "dunno" for compatibility. Not urgent to fix unless it causes issues.

---

## LOW Priority Issues (Nice to Have)

### 9. Empty changed_files array vs null
**File**: `src/receipt.rs:24, 114`
**Severity**: LOW
**Impact**: Cosmetic JSON difference

**Current**: `changed_files: None` (serialized as omitted)
**Homebrew**: `"changed_files": []` (empty array present)

**Fix**: Change line 114 to `changed_files: Some(vec![]),`

### 10. Compiler string format
**File**: `src/receipt.rs:117`
**Severity**: LOW
**Impact**: Cosmetic difference in JSON format

**Current**: `compiler: Some("clang".to_string())`  (string "clang")
**Homebrew**: `"compiler": "clang"` (string, same)

**Note**: Actually matches! No fix needed - keeping for completeness.

---

## Additional Checks Performed (No Issues Found)

✅ **Keg-only handling**: FIXED (this audit)
✅ **Pinned packages in upgrade**: Works correctly (line 1750, 1755)
✅ **Symlink cleanup**: Appears correct
✅ **Version comparison**: Handles bottle revisions correctly
✅ **Bottle selection**: Platform detection looks correct
✅ **Dependency resolution**: Runtime deps only (correct for bottles)

---

## Recommended Action Plan

### Phase 1: CRITICAL (Do Now)
1. Add `bottle_rebuild` field to RuntimeDependency
2. Add `source.versions` to receipt with formula version
3. Test receipt compatibility with `brew reinstall <bru-installed-package>`

### Phase 2: HIGH (Before Next Release)
4. Add pinned package check to reinstall command
5. Fix homebrew_version format (use actual brew version)
6. Add Xcode/CLT detection to receipts

### Phase 3: MEDIUM (Next Minor Release)
7. Populate source.path from API cache location
8. Fetch and populate source.tap_git_head

### Phase 4: LOW (Optional)
9. Change changed_files to empty array
10. Consider if cpu_family should be "dunno" for compat

---

## Testing Strategy

For each fix:
1. Install package with bru
2. Verify INSTALL_RECEIPT.json format matches Homebrew
3. Test `brew reinstall <package>` works
4. Test `brew upgrade <package>` works
5. Test bru can read Homebrew-generated receipts
6. Test brew can read bru-generated receipts

**Regression Test Cases**:
- Install with bru → upgrade with brew → verify works
- Install with brew → upgrade with bru → verify works
- Install with bru → brew list → verify shows package
- Install with bru → brew info → verify shows correct metadata
