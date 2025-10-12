# Test Scripts

Automated testing and benchmarking scripts for kombrucha (bru).

## Test Suites

### Smoke Tests (`test-smoke.sh`)
Fast validation that core commands work. No detailed assertions, just checks that commands complete successfully.

**Usage:**
```bash
./scripts/test-smoke.sh
```

**Tests:**
- commands, config, cache, doctor, tap, list (local commands)
- info, deps, search (API commands)

**Runtime:** ~15-20 seconds

### Integration Tests (`test-integration.sh`)
End-to-end workflow testing for install → reinstall → uninstall.

**Usage:**
```bash
./scripts/test-integration.sh
```

**Tests:**
- Install package
- Verify package appears in list
- Reinstall package
- Get info on installed package
- Uninstall package
- Verify package removed from list

**Test Package:** tree (simple package, no dependencies)

**Runtime:** ~5-10 seconds

### Functional Tests (`test-functional.sh`, `test-quick.sh`)
Comprehensive validation of command outputs. These test more edge cases and JSON output formats.

**Usage:**
```bash
./scripts/test-quick.sh  # Faster, focuses on critical tests
./scripts/test-functional.sh  # Complete test suite (slower)
```

**Tests:**
- JSON output validation (info --json, list --json)
- Command existence and basic output format
- API connectivity
- Local command correctness

**Runtime:**
- test-quick.sh: ~20 seconds
- test-functional.sh: ~60 seconds

## Benchmarks

### Simple Package (`../benchmarks/phase2-install.sh`)
Benchmarks installation of a package with no dependencies.

**Package:** tree

**Usage:**
```bash
./benchmarks/phase2-install.sh
```

### Complex Package (`../benchmarks/phase2-complex.sh`)
Benchmarks installation of a package with dependencies.

**Package:** jq (depends on oniguruma)

**Usage:**
```bash
./benchmarks/phase2-complex.sh
```

### Search Benchmark (`benchmark.sh`)
Benchmarks search performance with multiple queries (3 runs each).

**Usage:**
```bash
./scripts/benchmark.sh
```

## Running All Tests

```bash
# Unit tests
cargo test

# Smoke tests (quick validation)
./scripts/test-smoke.sh

# Integration tests (workflow)
./scripts/test-integration.sh

# Benchmarks
./benchmarks/phase2-install.sh
./benchmarks/phase2-complex.sh
```

## CI/CD Integration

These scripts are designed to run in CI environments:
- Exit code 0 on success, non-zero on failure
- Timeout-protected (won't hang indefinitely)
- Minimal dependencies (bash, cargo, standard tools)

## Development Workflow

When adding new features:
1. Write code
2. Run `cargo test` (unit tests)
3. Run `./scripts/test-smoke.sh` (quick validation)
4. Run `./scripts/test-integration.sh` (workflow check)
5. Run `cargo clippy` (linting)
6. Commit

Before releasing:
1. Run all test suites
2. Run benchmarks to document performance
3. Update benchmarks/results.md with new numbers
