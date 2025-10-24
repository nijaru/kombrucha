# Testing Strategy

**⚠️ DEPRECATED - SEE ai/TESTING_REMEDIATION.md**

This document is **outdated and unsafe**. Our previous testing strategy caused system corruption on Oct 23, 2025 by directly modifying `/opt/homebrew/Cellar/` without isolation.

## What Went Wrong

The testing strategy described in this document violated Homebrew best practices:

1. **No Isolation**: Integration tests modified real system directories
2. **System Corruption**: Tests corrupted node binary, mise shims, and Claude Code
3. **Wrong Patterns**: Used real Homebrew API but didn't isolate system state
4. **Missing Tools**: Didn't use testcontainers or brew test-bot for isolation

## Correct Approach

See **`ai/TESTING_REMEDIATION.md`** for the state-of-the-art testing strategy that follows Homebrew best practices:

### Key Principles (From Homebrew)

1. **Isolation First**
   - Use `testcontainers-rs` for Docker-based integration tests
   - Use `brew test-bot --local` for formula testing
   - Never modify system directories during tests

2. **Proper Tools**
   - testcontainers-rs: Industry standard for Rust integration testing
   - brew test-bot: Homebrew's official testing tool
   - GitHub Actions: Automated bottle building and multi-platform testing

3. **Test Environments**
   - Unit tests: Temp directories with automatic cleanup
   - Integration tests: Docker containers (complete isolation)
   - Formula tests: brew test-bot with testpath (temporary directory)

## Migration Plan

1. **Phase 1 (P0 - CRITICAL)**
   - Delete `tests/integration_tests.rs` (dangerous, causes system corruption)
   - Add testcontainers-rs for Docker-based testing
   - Add test helpers with proper temp directory isolation

2. **Phase 2 (P1)**
   - Add GitHub Actions workflows for brew test-bot
   - Implement meaningful formula test block
   - Set up automated bottle building

3. **Phase 3 (P2)**
   - Comprehensive test suite with functional domain organization
   - CI verification of system integrity
   - Multi-platform testing (macOS 13/14, Ubuntu)

## References

- **New Strategy**: `ai/TESTING_REMEDIATION.md` (complete, verified against Homebrew docs)
- **Incident Report**: `ai/STATUS.md` (Oct 23-24, 2025 system corruption)
- **Root Cause**: Tests in `tests/integration_tests.rs` modified real system

---

**Last updated**: 2025-10-24
**Status**: DEPRECATED - Do not use this strategy
**Replacement**: ai/TESTING_REMEDIATION.md
