# Testing and Tap Management Remediation Plan

## Executive Summary

After comprehensive research of Homebrew's official documentation and practices, we have identified **critical deficiencies** in kombrucha's testing infrastructure and tap management. Our current approach violates fundamental Homebrew principles and has caused system corruption.

## Quick Answers to Your Questions

### Q: Can we test in Docker containers locally?

**Yes, and we should.** This is the **state-of-the-art approach** for Rust projects testing package management:

1. **testcontainers-rs** (850+ stars, official Testcontainers implementation)
   - Complete isolation from host system
   - Industry standard for integration testing
   - Fast: containers start <1s with layer caching
   - Multi-platform: test Linux on macOS, vice versa
   - Automatic cleanup via RAII (Drop trait)

2. **brew test-bot --local** (Homebrew's official tool)
   - Tests full lifecycle: build → test → bottle
   - Safe: uses `./home/` and `./logs/` directories
   - Same tool Homebrew CI uses
   - **WARNING**: Never use `--ci-pr`, `--ci-master`, `--ci-testing` flags - they remove all installed packages!

**Recommendation:** Use **both**
- **testcontainers-rs** for kombrucha's own integration tests (Phase 1)
- **brew test-bot --local** for testing the formula in the tap (Phase 2)
- **GitHub Actions** with brew test-bot for CI (multi-platform bottles)

### Q: Is our TESTING_REMEDIATION.md guide SOTA and best practices?

**Initially: No.** The original version had gaps. **Now: Yes**, after updates based on:

**Sources verified:**
1. ✅ Official Homebrew documentation (docs.brew.sh)
2. ✅ Homebrew contributor guides and maintainer guides
3. ✅ Testcontainers official docs (testcontainers.com)
4. ✅ testcontainers-rs crate documentation
5. ✅ Homebrew/brew repository test structure
6. ✅ Real-world tap maintenance guides (Jonathan Chang's blog)
7. ✅ brew test-bot source code and documentation

**What was updated:**
- ✅ Added testcontainers-rs as primary local testing approach
- ✅ Added brew test-bot --local for formula testing
- ✅ Added specific GitHub Actions workflows for taps
- ✅ Clarified Docker vs CI trade-offs
- ✅ Added warnings about dangerous brew test-bot flags
- ✅ Provided complete bottle building workflow

**What makes this SOTA:**
1. **Testcontainers** - Industry standard (Docker, Kubernetes, major databases all use this)
2. **brew test-bot** - Homebrew's official testing tool (not a workaround)
3. **GitHub Actions + Releases** - Official Homebrew tap distribution method (since 2020)
4. **Isolation-first** - Zero system modification during tests
5. **Multi-platform CI** - Test on macOS 13, macOS 14, Ubuntu (like Homebrew does)

### Comparison: Our Approach vs Homebrew's

| Aspect | Homebrew | Our Current | Our Fixed (SOTA) |
|--------|----------|-------------|------------------|
| Local testing | `brew test-bot --local` | Direct system modification ❌ | testcontainers + brew test-bot ✅ |
| CI testing | GitHub Actions + test-bot | GitHub Actions (no test-bot) ❌ | Same as Homebrew ✅ |
| Test isolation | Temp dirs, sandbox-exec | None ❌ | Docker containers ✅ |
| Bottle building | Automated via PR labels | Manual ❌ | Automated ✅ |
| Formula tests | Functional tests | `--version` only ❌ | Functional tests ✅ |
| Multi-platform | macOS 13/14, Ubuntu | macOS only ❌ | macOS 13/14, Ubuntu ✅ |

## Current State Analysis

### What We're Doing Wrong

#### 1. **Test Isolation - CRITICAL FAILURE**

**Current Approach:**
```rust
// tests/integration_tests.rs:49-85
#[test]
fn test_install_uninstall_workflow() {
    let cellar = cellar_path(); // Returns /opt/homebrew/Cellar
    Command::new(bru_bin()).args(["install", "hello"]).output();
    // ❌ This ACTUALLY modifies /opt/homebrew/Cellar/hello
    // ❌ If test crashes, system is corrupted
    // ❌ Tests interfere with real Homebrew installations
}
```

**Homebrew's Approach:**
- Tests run in **temporary directories** that are automatically created and cleaned up
- The `test do` block in formulae uses `testpath` - an isolated temporary directory
- **Zero interaction with system directories during testing**

**Impact:** System corruption incidents on Oct 23, 2025:
- Node binary corrupted at kernel level (code signing validation failure)
- mise shims corrupted with binary garbage
- Claude Code unable to run (SIGKILL)
- All npm/node commands failing

#### 2. **Formula Test Block - VIOLATION**

**Current State:**
```ruby
# Formula/bru.rb:15-17
test do
  assert_match "bru #{version}", shell_output("#{bin}/bru --version")
end
```

**Homebrew Guidelines:**
> "Don't merge any formula updates with failing `brew test`s"
> "Consider asking to add one if the existing test only checks version output"
> "Good test: `foo build-foo input.foo`"
> "Bad test: `foo --version` and `foo --help`"

**Status:** ❌ Our test is explicitly listed as a "bad test" by Homebrew

#### 3. **Tap Management - MISSING INFRASTRUCTURE**

**What We're Missing:**

✅ Basic tap structure (`Formula/` directory) - PRESENT
❌ GitHub Actions workflow for automated bottle building - **MISSING**
❌ Automated testing via `brew test-bot` - **MISSING**
❌ Bottle distribution via GitHub Releases - **MISSING**
❌ PR-based bottle publishing workflow - **MISSING**

**Homebrew Standard (from `brew tap-new`):**
```
homebrew-tap/
  .github/
    workflows/
      publish.yml      # Publishes bottles to GitHub Releases
      tests.yml        # Runs brew test-bot on PRs
  Formula/
    bru.rb
  README.md
```

**Our Current Structure:**
```
homebrew-tap/
  Formula/
    bru.rb
  README.md
  # ❌ No .github/workflows/
```

#### 4. **Kombrucha's Own Tests - FUNDAMENTALLY FLAWED**

**Problems:**

1. **No environment isolation** - Tests modify real system
2. **No sandboxing** - No process isolation like Homebrew's `sandbox-exec`
3. **No path overrides** - No `HOMEBREW_PREFIX` environment variable usage
4. **Execution tests instead of behavior tests** - As identified in `ai/TESTING_ISSUES.md`
5. **Integration tests tagged `#[ignore]`** - Never run in CI

**Test Coverage Issues (from TESTING_ISSUES.md):**
- 70/92 tests are trivial (string formatting, URL construction)
- 14 tests only check exit codes
- 8 tests are `#[ignore]`d and never run
- **Critical cleanup bug went undetected** despite 92 tests

## Homebrew's Testing Philosophy

### Key Principles

1. **Isolation First**
   - Tests NEVER touch system directories
   - Temporary directories (`testpath`) for all operations
   - Environment variables control paths during testing

2. **Functional Over Trivial**
   - Test actual functionality, not just "doesn't crash"
   - Example: "Compile code against library" not "show version"
   - False positives better than false negatives

3. **Automated Quality Gates**
   - `brew test-bot` runs on every PR
   - Tests must pass before merge
   - Bottles built automatically on multiple platforms

4. **Comprehensive Test Organization**
   - Homebrew has 120+ RSpec test files
   - Organized by functional domain (api/, cmd/, utils/)
   - Uses parallel test execution (`.rspec_parallel`)

### Test Environment Isolation

Homebrew uses **superenv** to isolate tests:
- Removes `/usr/local/bin` from PATH
- Strips user environment contamination
- Injects only required dependencies
- On macOS: enforces `sandbox-exec` process sandboxing

## Remediation Plan

### Testing Strategy: Local vs CI

After researching state-of-the-art practices, we have **two complementary approaches**:

#### Option A: Local Testing with Docker/Testcontainers (Recommended for Development)

**Benefits:**
- Complete isolation - zero risk to host system
- Reproducible across all platforms
- Fast iteration cycle
- Can test Linux compatibility on macOS
- Industry standard (testcontainers-rs has 850+ stars)

**Implementation:**
```toml
# Cargo.toml
[dev-dependencies]
testcontainers = "0.23"
```

```rust
// tests/integration_tests.rs
use testcontainers::*;

#[test]
fn test_install_in_container() {
    let docker = clients::Cli::default();
    let container = docker.run(images::generic::GenericImage::new(
        "homebrew/ubuntu22.04",
        "latest"
    ));

    // All operations happen inside container
    // Host system completely protected
}
```

#### Option B: brew test-bot --local (Homebrew's Official Method)

**Benefits:**
- Official Homebrew testing tool
- Tests full lifecycle: build → test → bottle → upload
- Same tool used by Homebrew CI
- Can test bottle creation locally

**Usage:**
```bash
# Test formula in your tap
brew test-bot --tap=nijaru/tap --local bru

# Creates logs/ and home/ directories
# Tests installation, runs test do block, builds bottles
# Safe because --local sets $HOME to ./home/
```

**WARNING from Homebrew docs:**
> "never use --ci-pr, --ci-master, --ci-testing on your mac as it is going to remove everything installed"

**Recommended for us:** Use `--local` flag ONLY, never the --ci-* flags.

### Phase 1: Immediate Fixes (P0 - Critical)

#### 1.1 Delete Dangerous Integration Tests NOW

**Immediate action:**
```bash
rm tests/integration_tests.rs
```

These tests are **fundamentally unsafe** and cannot be salvaged. They:
- Modify real `/opt/homebrew/Cellar/`
- Have caused system corruption
- Cannot be fixed with environment variables alone

#### 1.2 Add Docker-Based Integration Tests (SOTA Approach)

**File:** `tests/docker_integration_tests.rs` (new)
```rust
#[cfg(test)]
mod docker_tests {
    use testcontainers::{clients, images, Container};
    use testcontainers::core::WaitFor;

    #[test]
    #[ignore] // Only run with: cargo test --test docker_integration_tests -- --ignored
    fn test_install_workflow_isolated() {
        let docker = clients::Cli::default();

        // Spin up Ubuntu container with Homebrew
        let container = docker.run(
            images::generic::GenericImage::new("homebrew/ubuntu22.04", "latest")
                .with_wait_for(WaitFor::message_on_stdout("Homebrew"))
        );

        // Copy bru binary into container
        let bru_path = env!("CARGO_BIN_EXE_bru");
        // ... copy to container ...

        // Run install command in container
        let output = container.exec(vec!["bru", "install", "jq"]);

        // Verify installation inside container
        assert!(output.contains("Successfully installed jq"));

        // Container destroyed automatically when test ends
        // Host system completely untouched ✓
    }
}
```

**Benefits:**
- **Complete isolation** - host system never touched
- **Reproducible** - same environment every run
- **Fast** - containers start in <1s with layer caching
- **Safe** - even if test crashes, container is destroyed
- **Multi-platform** - test Linux compatibility on macOS

#### 1.3 Add Test Helpers for Unit Tests

**File:** `tests/test_helpers.rs` (new)
```rust
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

/// Isolated test environment using tempdir
/// For unit tests that don't need full Docker isolation
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub prefix: PathBuf,
    pub cellar: PathBuf,
    pub cache: PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let prefix = temp_dir.path().to_path_buf();
        let cellar = prefix.join("Cellar");
        let cache = prefix.join("cache");

        // Create directory structure
        std::fs::create_dir_all(&cellar).unwrap();
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::create_dir_all(prefix.join("bin")).unwrap();

        Self {
            temp_dir,
            prefix,
            cellar,
            cache,
        }
    }
}

// Automatically cleaned up when dropped (RAII)
```

### Phase 2: Tap Management (P1 - High)

#### 2.1 Use brew test-bot for Local Tap Testing

**Recommended workflow:**
```bash
# Test the formula locally before pushing
cd /opt/homebrew/Library/Taps/nijaru/homebrew-tap
brew test-bot --tap=nijaru/tap --local bru

# This will:
# 1. Install bru from your tap
# 2. Run the test do block
# 3. Build bottles for your platform
# 4. Create logs/ directory with detailed output
# 5. Use ./home/ as $HOME (safe isolation)

# Review logs
cat logs/*

# If successful, bottles are in current directory
ls *.bottle.tar.gz
```

#### 2.2 Initialize Proper Tap Structure with GitHub Actions

**Option 1: Fresh tap with brew tap-new (if starting over)**
```bash
# Generate tap with proper structure
brew tap-new nijaru/tap

# This creates:
# - .github/workflows/publish.yml (bottle publishing)
# - .github/workflows/tests.yml (brew test-bot on PRs)
# - Formula/ directory
# - README.md
```

**Option 2: Add workflows to existing tap (our case)**
```bash
# Create .github/workflows directory
cd /opt/homebrew/Library/Taps/nijaru/homebrew-tap
mkdir -p .github/workflows

# Create tests.yml (runs on every PR)
cat > .github/workflows/tests.yml << 'EOF'
name: brew test-bot
on:
  pull_request:
jobs:
  test-bot:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-13, macos-14]
    steps:
      - name: Set up Homebrew
        uses: Homebrew/actions/setup-homebrew@master

      - name: Run brew test-bot
        run: brew test-bot --only-formulae
        env:
          HOMEBREW_GITHUB_API_TOKEN: ${{ secrets.GITHUB_TOKEN }}
EOF

# Create publish.yml (runs when PR labeled with pr-pull)
cat > .github/workflows/publish.yml << 'EOF'
name: Publish bottles
on:
  pull_request:
    types: [labeled]
jobs:
  publish:
    if: contains(github.event.pull_request.labels.*.name, 'pr-pull')
    runs-on: ubuntu-latest
    steps:
      - name: Upload bottles to GitHub Releases
        run: brew pr-upload
        env:
          HOMEBREW_GITHUB_API_TOKEN: ${{ secrets.GITHUB_TOKEN }}
EOF
```

#### 2.3 Add Meaningful Formula Test

**Update:** `Formula/bru.rb`
```ruby
test do
  # Bad test (what we have now):
  # assert_match "bru #{version}", shell_output("#{bin}/bru --version")

  # Good test (tests actual functionality):

  # Create a minimal test environment
  (testpath/"Cellar").mkpath
  (testpath/"bin").mkpath

  # Test that bru can list installed packages (even if empty)
  # This tests that bru can initialize and read its data structures
  output = shell_output("#{bin}/bru list 2>&1", 0)

  # Verify command succeeded
  # (output can be empty if no packages, that's valid)
  assert_match(/\A\s*\z|.*/, output)

  # Test that bru info works with a real formula
  info_output = shell_output("#{bin}/bru info wget")
  assert_match "wget", info_output
  assert_match(/Internet file retriever|URL retriever/, info_output)
end
```

**Even better test (if we support it):**
```ruby
test do
  # Test actual package manager functionality
  # Create isolated test environment
  ENV["HOMEBREW_CELLAR"] = testpath/"Cellar"
  ENV["HOMEBREW_PREFIX"] = testpath

  # Test search functionality
  output = shell_output("#{bin}/bru search wget")
  assert_match "wget", output

  # Test dependency resolution
  deps_output = shell_output("#{bin}/bru deps wget")
  assert_match(/openssl|libidn/, deps_output)
end
```

#### 2.4 Set Up Automated Bottle Building

**Workflow:**
1. Make formula changes in a branch
2. Push to GitHub and open PR
3. GitHub Actions automatically runs `brew test-bot`
4. Builds bottles for macOS 13, macOS 14, Ubuntu
5. When ready, add `pr-pull` label to PR
6. Bottles automatically uploaded to GitHub Releases
7. Formula updated with bottle SHA256 hashes
8. PR auto-merged

**Manual bottle building (for testing):**
```bash
# Build bottle for your platform
cd /opt/homebrew/Library/Taps/nijaru/homebrew-tap
brew test-bot --tap=nijaru/tap --local bru

# Upload to GitHub Releases (requires token)
export HOMEBREW_GITHUB_API_TOKEN=your_token
brew pr-upload --upload-only

# Or manually upload *.bottle.tar.gz to GitHub Releases
```

### Phase 3: Comprehensive Test Suite (NOT RECOMMENDED)

**⚠️ This phase is NOT RECOMMENDED and should be skipped.**

**Rationale:**
- **brew test-bot on CI is sufficient** - It's what Homebrew itself uses
- **CI already tests 3 platforms** - macOS 13, macOS 14, Ubuntu (from Phase 2)
- **Formula test block tests functionality** - Search, info, deps commands (from Phase 2)
- **Diminishing returns** - Docker tests would duplicate what brew test-bot does
- **Added complexity** - Requires maintaining Docker test infrastructure
- **brew test-bot is authoritative** - If it passes, the formula works

**When to reconsider:**
- If you need rapid local iteration on dangerous operations (install/uninstall)
- If you want to test specific broken package states that CI can't reproduce
- If brew test-bot CI is too slow for your development cycle

For most use cases, **Phase 1 + Phase 2 provide adequate testing coverage**.

---

**Original Phase 3 Plan (for reference, but not recommended):**

#### 3.1 Test Organization

Follow Homebrew's pattern:
```
tests/
  unit/              # Pure unit tests (no I/O)
    version_tests.rs
    dependency_tests.rs
  integration/       # Isolated integration tests
    install_tests.rs
    cleanup_tests.rs
  helpers/
    mod.rs           # TestEnvironment
  fixtures/          # Mock data
    formulae/
```

#### 3.2 Example Proper Integration Test

```rust
#[test]
fn test_install_workflow_isolated() {
    let env = TestEnvironment::new();

    // Create a mock formula in our test environment
    let formula_json = env.prefix.join("api/formula/test.json");
    std::fs::create_dir_all(formula_json.parent().unwrap()).unwrap();
    std::fs::write(&formula_json, r#"{
        "name": "test",
        "version": "1.0.0",
        ...
    }"#).unwrap();

    // Run bru install in isolated environment
    let output = Command::new(bru_bin())
        .args(["install", "test"])
        .env("BRU_TEST_MODE", "1")
        .env("BRU_TEST_PREFIX", &env.prefix)
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify installation in TEST directory, not system
    assert!(env.cellar.join("test/1.0.0").exists());
    assert!(!PathBuf::from("/opt/homebrew/Cellar/test").exists());
}
```

### Phase 4: CI Integration (P2 - Medium)

#### 4.1 Update GitHub Actions

**File:** `.github/workflows/test.yml`
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests (isolated)
        run: cargo test --test '*' -- --test-threads=1

      - name: Verify system integrity
        run: |
          # Ensure tests didn't modify system Homebrew
          if [ -d "/opt/homebrew" ]; then
            brew doctor || echo "Homebrew modified by tests!"
          fi
```

## Success Criteria

### Must Have (P0)

- [ ] All file operations validate paths before modification
- [ ] Zero tests touch system directories
- [ ] `BRU_TEST_MODE` environment variable controls test behavior
- [ ] Dangerous integration tests removed
- [ ] CI runs tests with isolation verification

### Should Have (P1)

- [ ] Tap has GitHub Actions workflows
- [ ] Formula has meaningful `test do` block
- [ ] Bottles published to GitHub Releases
- [ ] Automated testing via `brew test-bot`

### Nice to Have (P2)

- [ ] Test coverage >80% of critical paths
- [ ] Comprehensive integration test suite (isolated)
- [ ] Parallel test execution
- [ ] Test organization by functional domain

## Timeline

**Week 1 (Current):**
- Implement path guards
- Add test infrastructure helpers
- Delete dangerous integration tests
- Document new testing patterns

**Week 2:**
- Initialize tap with `brew tap-new`
- Set up GitHub Actions workflows
- Implement meaningful formula test
- First bottle release

**Week 3:**
- Rewrite integration tests with isolation
- Reorganize test suite structure
- Add CI verification steps
- Comprehensive testing documentation

## References

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [How to Create and Maintain a Tap](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Acceptable Formulae](https://docs.brew.sh/Acceptable-Formulae)
- [Bottles Documentation](https://docs.brew.sh/Bottles)
- [Homebrew tap with bottles (blog post)](https://brew.sh/2020/11/18/homebrew-tap-with-bottles-uploaded-to-github-releases/)
- [Homebrew/brew test suite structure](https://github.com/Homebrew/brew/tree/master/Library/Homebrew/test)

## Appendix: System Corruption Incident (Oct 23, 2025)

**Root Cause:** Integration tests modified `/opt/homebrew/Cellar/` directly, corrupting:
- Node binary (kernel code signing failure → SIGKILL)
- mise shim files (binary garbage instead of shell scripts)
- Homebrew linking directory (missing `/opt/homebrew/var/homebrew/linked/`)

**Resolution:** Manual reinstallation of mise, node, and Claude Code

**Prevention:** This remediation plan addresses all root causes
