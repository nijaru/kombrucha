# Kombrucha Development Guide for AI Agents

## Project Overview
Kombrucha (bru) is a fast, parallel Homebrew-compatible package manager written in Rust. It aims for 100% compatibility with Homebrew while providing significant performance improvements through parallelization.

**Status**: Experimental / Unstable - Use alongside Homebrew, not as a replacement.
**Current Version**: 0.1.30

## ⚠️ CRITICAL: Always Match Homebrew's Behavior

**BEFORE implementing ANY Homebrew-compatible feature:**

1. **ALWAYS check Homebrew source code FIRST**: https://github.com/Homebrew/brew
   - Don't guess how Homebrew works - look at the actual implementation
   - Check Ruby source in `Library/Homebrew/` directory
   - Check OS-specific code in `Library/Homebrew/extend/os/mac/` and `extend/os/linux/`
   - Use WebSearch or WebFetch to find relevant source files

2. **ALWAYS check Homebrew documentation**: https://docs.brew.sh/
   - Official docs explain expected behavior and edge cases
   - Bottles doc: https://docs.brew.sh/Bottles
   - Formula Cookbook: https://docs.brew.sh/Formula-Cookbook

3. **ALWAYS test against actual Homebrew behavior**:
   - Install the same package with both `brew` and `bru`
   - Compare outputs, file contents, binary behavior
   - Test edge cases (scripts, dylibs, binaries)

4. **NEVER assume or guess implementation details**:
   - "I think Homebrew probably does X" → WRONG ❌
   - "Let me check Homebrew source to see if it does X" → RIGHT ✅

## Critical Compatibility Requirements

### Bottle Relocation
**CRITICAL**: Bottle relocation must match Homebrew's behavior exactly.

Key Homebrew source files:
- Relocation logic: `Library/Homebrew/keg_relocate.rb`
- macOS-specific: `Library/Homebrew/extend/os/mac/keg_relocate.rb`
- Codesigning: `Library/Homebrew/extend/os/mac/keg.rb`

**Key Homebrew behaviors to match**:
   - Codesigning command on Apple Silicon: `codesign --sign - --force --preserve-metadata=entitlements,requirements,flags,runtime`
   - Only codesign AFTER running `install_name_tool` (which invalidates signatures)
   - Handle both `@@HOMEBREW_PREFIX@@` and `@@HOMEBREW_CELLAR@@` placeholders
   - Process Mach-O binaries, dylibs, AND executable scripts with shebangs
   - Preserve entitlements, requirements, flags, and runtime metadata

**Testing requirements**:
   - Test actual binary execution, not just installation
   - Test on Apple Silicon (ARM64) where codesigning is critical
   - Test packages with Python scripts (e.g., huggingface-cli)
   - Test packages with native binaries (e.g., bat, wget)
   - Verify no SIGKILL crashes (exit code 137)

**Historical Issues:**
- **v0.1.18**: Didn't relocate script shebangs → scripts failed with "bad interpreter"
- **v0.1.19**: Used `codesign --remove-signature` → ALL binaries crashed with SIGKILL
- **v0.1.20**: Fixed by matching Homebrew's exact codesign flags

### Other Critical Homebrew Features

When implementing any of these features, **ALWAYS check Homebrew source first**:

- **Formula installation**: Check `Library/Homebrew/formula_installer.rb`
- **Dependency resolution**: Check `Library/Homebrew/dependencies.rb`
- **Tap management**: Check `Library/Homebrew/tap.rb`
- **Cask handling**: Check `Library/Homebrew/cask/` directory
- **Service management**: Check `Library/Homebrew/service.rb`
- **Cleanup logic**: Check `Library/Homebrew/cleanup.rb`
- **Upgrade logic**: Check `Library/Homebrew/upgrade.rb`

**The pattern that caused v0.1.18-19 disasters:**
1. ❌ Assumed how Homebrew works
2. ❌ Implemented based on assumptions
3. ❌ Tests didn't catch the incompatibility
4. ❌ Broken code shipped to users

**The correct pattern:**
1. ✅ Check Homebrew source code
2. ✅ Match exact behavior
3. ✅ Test against actual Homebrew output/behavior
4. ✅ Ship working code

## Development Workflow

### Before Implementing Homebrew-Related Features
1. Check if Homebrew has equivalent functionality
2. Review Homebrew source code for implementation details
3. Check Homebrew documentation: https://docs.brew.sh/
4. Test against actual Homebrew behavior

### Testing Strategy
**Unit Tests**: Cover core logic and version comparison
**Integration Tests**: Test actual package installation and execution
**Regression Tests**: Prevent known bugs from returning

**Test Gap That Caused v0.1.18-19 Issues**:
- Tests only verified command output, never executed installed binaries
- Missing: Integration tests that install bottles and run the resulting executables

## Code Patterns

### Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for domain-specific error types
- Log warnings for non-critical failures, errors for critical ones

### Parallelization
- Use `rayon` for parallel operations
- Be careful with file handle limits (use `.max_open(64)` with WalkDir)
- Collect results before parallel processing when needed

### Homebrew Compatibility
- Match Homebrew's output format when possible
- Support same command arguments and flags
- Fall back to `brew` for unsupported operations

## File Locations
- **Core logic**: `src/`
- **Tests**: `tests/`
- **Project docs**: `docs/` (for users/team)
- **Agent context**: `ai/` (machine-optimized)
  - `ai/STATUS.md` - current state, recent changes, known issues
  - `ai/TODO.md` - pending tasks
  - `ai/DECISIONS.md` - architecture decisions
  - `ai/RESEARCH.md` - research findings

## Release Process
1. Fix bugs and test thoroughly
2. Update `ai/STATUS.md` with changes
3. Bump version in `Cargo.toml` (only when explicitly instructed)
4. Run tests: `cargo test`
5. Build: `cargo build --release`
6. Test locally before pushing
7. NEVER release without CI passing
8. See global `~/.claude/CLAUDE.md` for complete release workflow

## Performance Expectations
- **Parallelization**: Core operations should use parallel processing
- **API Calls**: Parallelize API requests with in-memory caching
- **Startup Time**: Keep under 50ms
- Benchmark against `brew` to ensure improvements

## References
- Homebrew source: https://github.com/Homebrew/brew
- Homebrew docs: https://docs.brew.sh/
- Homebrew API: https://formulae.brew.sh/
- Homebrew Bottles doc: https://docs.brew.sh/Bottles
