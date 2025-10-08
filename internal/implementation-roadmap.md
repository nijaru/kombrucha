# Kombrucha Implementation Roadmap

## Vision

Create a Homebrew-compatible package manager that is 10-20x faster than Homebrew on modern networks through aggressive parallelization and smart dependency resolution.

## Principles

1. **Parallel by Default**: Every operation that can be parallelized, will be
2. **Beautiful UX**: Output should be informative, not overwhelming
3. **Homebrew Compatible**: Use same API, formulae, bottles, taps
4. **Pragmatic**: Ship value incrementally, don't boil the ocean

## Phase 0: Foundation (Week 1-2)

**Goal**: Project structure and core types

### Tasks
- [ ] `cargo init` with workspace structure
- [ ] Core types: `Formula`, `Bottle`, `Dependency`, `Version`
- [ ] JSON API client with `reqwest` + `serde`
- [ ] Configuration management (read Homebrew paths)
- [ ] CLI scaffolding with `clap`
- [ ] Error handling framework
- [ ] Logging setup (`tracing` crate)

### Deliverable
```bash
bru --version  # Shows version instantly
bru --help     # Shows help
```

**Success Metric**: Project compiles, basic CLI works

---

## Phase 1: Read-Only Commands (Week 3-4)

**Goal**: Prove performance gains with simple operations

### 1.1: Search & Info
```bash
bru search <query>    # Search formulae/casks
bru info <formula>    # Show formula info
```

**Implementation**:
- Query Homebrew JSON API (`formulae.brew.sh`)
- Parallel search across formula + cask APIs
- Pretty formatted output with `comfy-table`

**Performance Target**: 3-5x faster than `brew search`

### 1.2: Dependency Graph
```bash
bru deps <formula>              # Show dependency tree
bru deps --tree <formula>       # Pretty tree view
bru deps --graph <formula>      # ASCII graph
bru uses <formula>              # Show reverse deps
```

**Implementation**:
- Parallel dependency resolution (fetch all metadata concurrently)
- Build complete dependency graph
- Visualize with `ptree` or custom tree builder
- Detect cycles

**Performance Target**: 10x faster than `brew deps` for complex packages

### 1.3: List & Status
```bash
bru list                  # List installed packages
bru list --versions       # Show all installed versions
bru outdated              # Show outdated packages
```

**Implementation**:
- Read Homebrew's Cellar directory
- Parse install receipts
- Parallel check for updates via API

**Performance Target**: 2-3x faster than `brew list`

### Deliverable
Working read-only CLI that demonstrates speed improvements

**Success Metric**: User can search, inspect, and visualize dependencies faster than Homebrew

---

## Phase 2: Bottle-Only Installation (Week 5-8)

**Goal**: Install packages from pre-built bottles with parallel downloads

### 2.1: Download Manager
```bash
bru fetch <formula>...    # Download bottles
```

**Implementation**:
- Parallel bottle downloads with `tokio`
- Progress bars with `indicatif` (multi-progress)
- Checksum verification (parallel)
- Resume support for interrupted downloads
- Connection pooling

**Features**:
- Show download speed per bottle
- Show overall progress
- Pretty status indicators
- ETA calculation

**Performance Target**: 10x faster downloads (10 concurrent vs sequential)

### 2.2: Installation Engine
```bash
bru install <formula>...     # Install from bottles
```

**Implementation**:
- Parallel dependency resolution
- Check for bottle availability (all deps must have bottles)
- Parallel downloads + extraction
- Smart scheduling: extract while downloading others
- Symlink management
- Install receipt generation

**Features**:
- Show dependency graph before install
- Progress for each parallel operation
- Rollback on failure
- Skip already-installed packages

**Performance Target**: 10-15x faster for multi-package installs

### 2.3: Upgrade & Reinstall
```bash
bru upgrade [formula]...     # Upgrade packages
bru reinstall <formula>      # Reinstall package
```

**Implementation**:
- Fetch outdated packages
- Resolve dependencies
- Download + install (parallel)
- Clean up old versions

### Deliverable
Full bottle-based package manager with parallel everything

**Success Metric**: Installing 10 packages is 10x faster than Homebrew

---

## Phase 3: Source Builds & Ruby Interop (Week 9-12)

**Goal**: Handle formulae without bottles via embedded Ruby

### 3.1: Ruby Embedding
**Implementation**:
- Embed Ruby interpreter with `magnus`
- Load formula DSL
- Execute `install` method
- Capture output
- Handle pre/post-install hooks

### 3.2: Source Build Manager
```bash
bru install --build-from-source <formula>
```

**Implementation**:
- Fall back to source when no bottle
- Call Ruby interpreter for build
- Show build progress
- Capture logs

### 3.3: Advanced Commands
```bash
bru uninstall <formula>      # Uninstall package
bru tap <repo>               # Add tap
bru untap <repo>             # Remove tap
```

### Deliverable
Full Homebrew compatibility for all formulae

**Success Metric**: Can install any formula, with or without bottles

---

## Phase 4: Advanced Features (Week 13-16)

**Goal**: Go beyond Homebrew with better UX and features

### 4.1: Better Dependency Resolution
**Options**:
- Option A: SAT solver with `resolvo` crate
- Option B: Cargo-style backtracking resolver

**Features**:
- Clear conflict messages
- Suggest resolutions
- Explain why versions were chosen
- Optional: Allow version flexibility (non-Homebrew mode)

### 4.2: Lock Files (Optional)
```bash
bru lock                     # Generate bru.lock
bru install --locked         # Install from lock file
```

**Implementation**:
- Generate lock file with exact versions
- Reproducible installs
- Share across team

### 4.3: Enhanced UX
```bash
bru why <formula>            # Why is this installed?
bru doctor                   # Check system health
bru config                   # Show configuration
```

**Features**:
- Better error messages
- Suggestions for fixes
- System diagnosis
- Performance stats

### Deliverable
Feature-complete package manager with innovations beyond Homebrew

**Success Metric**: Users prefer `bru` over `brew` for daily use

---

## Phase 5: Polish & Distribution (Week 17-20)

**Goal**: Production-ready release

### Tasks
- [ ] Comprehensive testing
- [ ] Error handling polish
- [ ] Documentation (README, wiki, examples)
- [ ] Installation script
- [ ] Homebrew formula for `bru` (meta!)
- [ ] CI/CD pipeline
- [ ] Benchmarking suite
- [ ] Migration guide from Homebrew
- [ ] Blog post / launch announcement

### Distribution
- Publish to crates.io
- GitHub releases with binaries
- Homebrew tap for installation
- Package for other systems

---

## MVP Scope (Phases 0-2)

**Timeline**: 8 weeks part-time, 4 weeks full-time

**Features**:
- Fast search, info, deps commands
- Parallel bottle downloads
- Parallel installation
- Basic upgrade/reinstall
- Pretty CLI output
- 10-15x faster than Homebrew

**Non-Goals for MVP**:
- Source builds (Ruby interop) - Phase 3
- Advanced dependency resolution - Phase 4
- Lock files - Phase 4
- Full command parity - Phases 4-5

**Why This MVP is Valuable**:
- Proves performance thesis
- Handles 80% of use cases (most packages have bottles)
- Gets feedback early
- Shows clear value vs Homebrew

---

## Technical Architecture

### Project Structure
```
kombrucha/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── bru/                # CLI binary
│   ├── kombrucha-core/     # Core types and logic
│   ├── kombrucha-api/      # Homebrew API client
│   ├── kombrucha-install/  # Installation engine
│   ├── kombrucha-resolve/  # Dependency resolver
│   ├── kombrucha-download/ # Download manager
│   └── kombrucha-ruby/     # Ruby interop (Phase 3)
├── tests/                  # Integration tests
├── benches/                # Performance benchmarks
└── internal/               # Documentation
```

### Key Dependencies
```toml
[dependencies]
# CLI & UX
clap = { version = "4", features = ["derive"] }
indicatif = "0.17"
comfy-table = "7"
owo-colors = "4"

# Async & HTTP
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
futures = "0.3"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Archive handling
tar = "0.4"
flate2 = "1"

# Utilities
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"

# Phase 3: Ruby
# magnus = "0.7"

# Phase 4: Dependency resolution
# resolvo = "0.4"
```

---

## Risk Mitigation

### Technical Risks
| Risk | Mitigation |
|------|------------|
| Ruby embedding is hard | Start with bottle-only MVP |
| API changes | Version API client, document assumptions |
| Formula compatibility | Test against common formulae |
| Homebrew updates break us | Pin to API version, monitor changes |

### Scope Risks
| Risk | Mitigation |
|------|------------|
| Too ambitious | Ship MVP (Phases 0-2) first |
| Feature creep | Stick to roadmap, defer nice-to-haves |
| Perfect is enemy of good | 80/20 rule, iterate |

---

## Success Metrics

### Phase 1 (Read-Only)
- [ ] `bru search` is 3x faster than `brew search`
- [ ] `bru deps` is 10x faster for complex packages
- [ ] Users report "it feels snappier"

### Phase 2 (Bottle Install)
- [ ] `bru install` 10+ packages is 10x faster than `brew install`
- [ ] Parallel downloads show clear progress
- [ ] No bottle corruption or failed installs
- [ ] 100+ packages tested successfully

### Phase 3 (Source Builds)
- [ ] Can build packages without bottles
- [ ] Ruby formulae work correctly
- [ ] All Homebrew core formulae compatible

### Phase 4 (Advanced)
- [ ] Better error messages than Homebrew
- [ ] Lock files work for reproducibility
- [ ] Users prefer bru for dependency inspection

---

## Timeline Summary

| Phase | Duration | Effort | Deliverable |
|-------|----------|--------|-------------|
| Phase 0 | 1-2 weeks | 40-80h | Project structure |
| Phase 1 | 2-3 weeks | 80-120h | Read-only commands |
| Phase 2 | 3-4 weeks | 120-160h | Bottle installs |
| **MVP DONE** | **8 weeks** | **240h** | **Usable package manager** |
| Phase 3 | 3-4 weeks | 120-160h | Source builds |
| Phase 4 | 3-4 weeks | 120-160h | Advanced features |
| Phase 5 | 3-4 weeks | 120-160h | Production release |
| **Total** | **20 weeks** | **720h** | **Full replacement** |

**Part-time (20h/week)**: 20 weeks for MVP, 36 weeks for full release
**Full-time (40h/week)**: 6 weeks for MVP, 18 weeks for full release

---

## Next Steps

1. **Initialize Project** (This week)
   - `cargo init --lib kombrucha-core`
   - Set up workspace
   - Create initial types

2. **First Feature** (Next week)
   - Implement `bru search`
   - Benchmark against `brew search`
   - Prove speed thesis

3. **Build Momentum**
   - Ship Phase 0-1 quickly
   - Get early feedback
   - Iterate on UX

Let's build this! 🚀
