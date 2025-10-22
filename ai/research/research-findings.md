# Research Findings: Homebrew Performance & Dependency Management

## Summary

This document compiles research on Homebrew's performance limitations, dependency management issues, and maintainer perspectives to inform Kombrucha's development priorities.

## Key Findings from Homebrew Maintainers

### Performance Bottlenecks

**Maintainer Perspective (from GitHub issues #7755, #3901):**
- Mike McQuaid and other maintainers stated: "Homebrew's performance is dominated by networking and housekeeping operations, not the Ruby runtime"
- CLI startup time: ~0.6 seconds
- Maintainers were skeptical of rewrites, viewing them as "vast, buggy undertaking to shave off relatively small amounts of time"

**Community Pain Points:**
- Sequential downloads are a major bottleneck
- Dependency resolution not parallelized
- No concurrent operations during install/upgrade

### Recent Improvements (Homebrew 4.6.0 - 2025)

- **Concurrent Downloads**: Finally added as opt-in via `HOMEBREW_DOWNLOAD_CONCURRENCY=auto`
- Will become default in future version
- Currently only for `brew fetch`, expanding to other commands
- Maintainers were previously concerned about server load from parallel downloads

### Dependency Management Issues

#### Current Problems:

1. **No Version Flexibility**: Homebrew forces everything to latest version, can't mix versions
   - "Everything a formula depends on needs to be upgraded to the latest version"
   - "Any given upgrade can upgrade many other seemingly unrelated formulae"

2. **Keg-Only Complexity**:
   - Formulas installed but not linked to avoid conflicts
   - Breaks after `brew upgrade` due to version mismatches
   - Confusing for users

3. **Dependency Resolution Disagreements**: Issues where `brew missing` and `brew install --only-dependencies` conflict

4. **No External Tooling Recognition**: Homebrew doesn't cooperate with non-Homebrew installs (e.g., rustup)

5. **Link Conflicts**: Require manual `brew link --overwrite` to resolve

## Modern Package Manager Best Practices

### Dependency Resolution Algorithms

**SAT Solver Approach:**
- Used by: OpenSUSE (libsolv), Python Anaconda, Solaris pkg
- Translates dependency problem to Boolean satisfiability
- Uses CDCL (Conflict-Driven Clause Learning)
- Can solve complex trees in milliseconds to seconds
- Trade-off: Loses domain-specific error reporting

**Cargo (Rust) Approach:**
- Uses backtracking solver (simpler than full SAT)
- Prefers highest compatible versions
- SemVer-aware
- Unifies dependency versions where possible
- Deterministic resolution
- Fast enough for most cases

**npm Approach:**
- Flattens dependency tree
- Places packages at highest ancestor where constraints satisfied
- Keeps separate copies only when version ranges incompatible

### Key Features Modern Resolvers Provide:

1. **Version Flexibility**: Allow compatible versions to coexist
2. **Conflict Detection**: Clear error messages about incompatibilities
3. **Optimized Resolution**: Prefer version unification to minimize duplicates
4. **Fast Performance**: Parallel/concurrent operations
5. **Deterministic**: Same inputs → same outputs
6. **SemVer-Aware**: Understand compatibility rules

## Opportunities for Kombrucha

### Performance Wins

1. **Parallel Downloads from Day 1**
   - Homebrew took years to add this
   - Major user pain point
   - Rust + tokio makes this trivial

2. **Concurrent Dependency Resolution**
   - Fetch metadata for entire tree in parallel
   - Homebrew does this sequentially

3. **Fast Startup**
   - Compiled binary = instant startup
   - No Ruby interpreter overhead

4. **Parallel Bottle Extraction**
   - Extract multiple archives concurrently
   - Homebrew does one at a time

### Dependency Management Improvements

1. **Better Resolver**
   - Consider SAT solver (libsolv has Rust bindings: `resolvo`)
   - Or implement Cargo-style backtracking solver
   - Provide clear conflict resolution messages

2. **Version Flexibility** (Maybe)
   - Allow compatible versions to coexist?
   - Trade-off: Breaks Homebrew's "test one configuration" philosophy
   - Could be opt-in: strict mode (Homebrew-compatible) vs flexible mode

3. **Better Keg-Only Handling**
   - Automatic environment setup for keg-only deps
   - Clear messaging about why something is keg-only

4. **Dependency Graph Visualization**
   - `bru deps --graph <formula>` with pretty tree output
   - Show why versions were chosen

5. **Lock File Support**
   - Optional `bru.lock` for reproducible installs
   - Like Cargo.lock, package-lock.json

### UX Improvements

1. **Pretty Output**
   - Progress bars (indicatif)
   - Colored output (colored/owo-colors)
   - Clear status of parallel operations
   - Tree views for dependencies

2. **Better Error Messages**
   - Show WHY a conflict exists
   - Suggest resolution steps
   - Link to relevant docs

3. **Progress Transparency**
   - Show what's happening in parallel
   - Download speeds
   - ETA for operations

## Related Projects

- **pkgx**: Max Howell's (Homebrew creator) new project, written in Rust
- **sp**: Another Rust-based Homebrew alternative (uses Homebrew API)

## Recommendations for Kombrucha

### Phase 1 Priorities:
1. Parallel dependency metadata fetching
2. Fast, correct dependency resolver (start with backtracking, consider SAT later)
3. Pretty CLI output from day 1

### Phase 2 Priorities:
1. Parallel downloads (multiple bottles at once)
2. Parallel bottle extraction
3. Dependency graph visualization

### Phase 3+:
1. Advanced resolver features (version flexibility, better conflict resolution)
2. Lock file support
3. External tooling cooperation (rustup, nvm, etc.)

### Key Differentiators:
- **Speed**: Parallel everything that Homebrew does sequentially
- **UX**: Beautiful, informative output
- **Clarity**: Better error messages and dependency understanding
- **Optional Advanced Features**: Lock files, version flexibility (without breaking Homebrew compatibility)

## Sources

- Homebrew GitHub Issues: #7755, #3901, #1865, #13064, #18278
- Homebrew 4.6.0 Release Notes
- Dependency Resolution Articles (ochagavia.nl, Cargo docs)
- SAT Solver Research (research.swtch.com/version-sat)
