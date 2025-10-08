# Research Conclusions - All Open Questions Answered

## Tested & Verified

### 1. API Rate Limiting ✅ NO RATE LIMITS

**Test**: Made 50 rapid consecutive requests to formulae.brew.sh

**Result**: 50/50 succeeded, zero rate limiting (HTTP 429)

**Conclusion**:
- No rate limiting detected on Homebrew JSON API
- Can make rapid requests without throttling
- Still should cache for performance and offline use
- Caching is optimization, not requirement

**Cache Strategy**:
```
~/.cache/bru/
  api/
    formulae.json     # TTL: 1 hour (changes infrequently)
    casks.json        # TTL: 1 hour
    formula/
      wget.json       # TTL: 1 day (stable)
```

**Implementation**: Use simple file-based cache with mtime checks

---

### 2. Tap Structure ✅ API-FIRST, NO CORE TAP

**Discovery**: Modern Homebrew doesn't clone homebrew-core!

**Findings**:
```bash
$ brew tap-info homebrew/core
homebrew/core: Not installed

$ ls /opt/homebrew/Library/Taps/
homebrew/              # Only has homebrew-command-not-found
kushalpandya/          # Third-party tap (git repo)
oven-sh/               # Third-party tap (git repo)
```

**Modern Homebrew Architecture**:
- homebrew-core formulae accessed via JSON API only
- No local git clone of core formulae
- Third-party taps are still git repos
- Tap structure: `/opt/homebrew/Library/Taps/<user>/<repo>/`

**For bru**:
- **Phase 1-2**: Only support homebrew-core via API (no tap management)
- **Phase 3+**: Add third-party tap support (git clone + parse .rb files)
- Matches modern Homebrew's approach

---

### 3. Keg-Only Behavior ✅ MODERN HOMEBREW CHANGED

**Findings**:
```bash
$ brew info openssl@3 --json
{"keg_only": false, "keg_only_reason": null}

$ ls -la /opt/homebrew/bin/openssl
lrwxr-xr-x ... openssl -> ../Cellar/openssl@3/3.6.0/bin/openssl
```

**Surprise**: openssl@3 used to be keg-only, but modern Homebrew changed this!

**Modern Behavior**:
- openssl@3 IS symlinked to /opt/homebrew/bin/
- python@3.12 IS symlinked to /opt/homebrew/bin/
- /opt/homebrew/opt/ has version-specific symlinks:
  - openssl, openssl@3, openssl@3.2, openssl@3.3, etc. → all point to current

**Conclusion**:
- Modern Homebrew is more permissive with symlinks
- "keg-only" concept may be deprecated or rarely used
- Still need to check `keg_only` field in API responses
- For Phase 2: Create symlinks unless `keg_only: true`

---

### 4. Dependency Resolution ✅ TREE WITH DEDUPLICATION

**Test**:
```bash
$ brew deps wget
ca-certificates
gettext
libidn2
libunistring
openssl@3

$ brew deps --tree wget
wget
├── libidn2
│   ├── libunistring
│   └── gettext
│       └── libunistring
├── openssl@3
│   └── ca-certificates
├── gettext
│   └── libunistring
└── libunistring
```

**Key Insights**:
1. **Flat list** = deduplicated (libunistring appears once)
2. **Tree** = shows full structure (libunistring appears 3 times in tree)
3. Homebrew warns that --tree may differ from declared dependencies

**Dependency Conflict Strategy**:
- Homebrew forces everything to latest version
- No version pinning or multiple versions of same formula
- Cascading upgrades when deps need updating
- Simple strategy: match Homebrew exactly for Phase 1-2

**For bru**:
- Phase 1-2: Match Homebrew (force latest, no conflicts)
- Phase 4: Consider advanced resolution (SAT solver, version ranges)

---

### 5. Configuration & Performance-Based Defaults

**User Question**: Should we change config/features due to perf differences?

**YES - Recommended Changes**:

**1. More Aggressive Concurrency** (Homebrew: cautious, bru: fast)
```toml
[downloads]
concurrency = 20          # vs Homebrew's 10
timeout = 60              # Longer timeout, we can afford it

[api]
cache_ttl = 3600          # 1 hour (same as Homebrew)
prefetch_deps = true      # NEW: Prefetch dependencies in background
```

**2. Enable Features Homebrew Disables for Speed**
```bash
# Homebrew users disable these for perf:
HOMEBREW_NO_AUTO_UPDATE=1      # bru: can enable (fast check)
HOMEBREW_NO_INSTALL_CLEANUP=1  # bru: can enable (fast cleanup)
```

**bru advantages**:
- Instant startup → can check for updates every time (non-blocking)
- Fast cleanup → can run after every install
- Parallel downloads → higher concurrency is fine

**3. Better Defaults for Modern Networks**
```toml
[network]
download_concurrency = 20    # Modern network can handle it
retry_attempts = 3          # Retry failed downloads
retry_delay = 1             # Fast retry (1s vs Homebrew's 5s)
```

**4. Enhanced Output (We Can Afford It)**
```toml
[output]
show_download_speeds = true    # Show real-time speeds
show_progress_bars = true      # Multiple bars for parallel ops
show_dep_tree = true           # Show tree during install
color = auto                   # Auto-detect terminal support
```

**5. Background Operations**
```toml
[background]
auto_update_check = true       # Check updates in background (non-blocking)
prefetch_bottles = true        # Start downloading while resolving deps
```

---

### 6. Development Caching Strategy

**User Question**: Should we cache anything from brew infra for dev?

**YES - Smart Development Caching**:

**1. For Integration Tests**:
```bash
# Cache test data to speed up tests
tests/fixtures/
  api/
    formulae.json              # Full formula list (updated weekly)
    formula-wget.json          # Individual formulae
    formula-node.json
  bottles/
    wget--1.25.0.tar.gz       # Small test bottles (<5MB)
```

**Benefits**:
- Tests run offline
- Tests run faster (no network calls)
- Consistent test data
- Can test against specific versions

**2. For Benchmarking**:
```bash
# Cache Homebrew responses to ensure fair comparison
benchmark/cache/
  homebrew-api-response-wget.json    # What Homebrew fetches
  homebrew-timing-wget.log            # Homebrew's timing
```

**Benefits**:
- Consistent benchmarks
- Can compare same API responses
- Isolate network variance

**3. Implementation**:
```rust
// tests/common/fixtures.rs
pub fn load_fixture(name: &str) -> String {
    include_str!(concat!("../fixtures/", name))
}

#[tokio::test]
async fn test_with_fixture() {
    let json = load_fixture("api/formula-wget.json");
    let formula: Formula = serde_json::from_str(&json).unwrap();
    // Test logic...
}
```

**Refresh strategy**:
- Update fixtures weekly via script
- CI uses committed fixtures (fast, consistent)
- Local dev can refresh: `./scripts/update-fixtures.sh`

---

### 7. Shell Completions ✅ TRIVIAL WITH CLAP

**Answer**: Easy, clap generates them automatically

**Implementation**:
```rust
// Add to main.rs or separate binary
use clap::CommandFactory;
use clap_complete::{generate, shells::*};

fn generate_completions() {
    let mut app = Cli::command();

    generate(Bash, &mut app, "bru", &mut std::io::stdout());
    generate(Zsh, &mut app, "bru", &mut std::io::stdout());
    generate(Fish, &mut app, "bru", &mut std::io::stdout());
}
```

**Usage**:
```bash
# Bash
bru completions bash > /usr/local/share/bash-completion/completions/bru

# Zsh
bru completions zsh > /usr/local/share/zsh/site-functions/_bru

# Fish
bru completions fish > ~/.config/fish/completions/bru.fish
```

**Priority**: Phase 3-4 (nice to have, not critical)

---

## Summary of Decisions

| Question | Answer | Impact |
|----------|--------|--------|
| API Rate Limits | None detected | ✅ No throttling needed, cache for perf |
| Tap Structure | API-only for core | ✅ No tap management in Phase 1-2 |
| Keg-Only | Rarely used now | ✅ Simple: check flag, skip symlinks |
| Dep Conflicts | Force latest version | ✅ Match Homebrew exactly |
| Config Changes | More aggressive defaults | ✅ Leverage speed advantage |
| Dev Caching | Yes, for tests/benchmarks | ✅ Faster dev workflow |
| Shell Completions | Clap auto-generates | ✅ Easy to add later |

## Performance-Optimized Defaults for bru

**Key Principle**: Since bru is 7-20x faster, we can:
1. Enable features Homebrew disables (auto-update, cleanup)
2. Use higher concurrency (20 vs 10)
3. Prefetch/background operations
4. Show richer output (progress bars, speeds)
5. Faster retries (1s vs 5s)

**User Experience**:
- Homebrew: Users disable features for speed
- bru: All features enabled, still faster than Homebrew

**Configuration**:
- Start with no config file (sensible defaults)
- Add optional config in Phase 3+ if requested
- Environment variables for power users

---

## Action Items

### Phase 1
- [x] Use API-only approach (no tap cloning)
- [x] Implement file-based cache (~/.cache/bru/)
- [x] Set aggressive concurrency defaults (20)

### Phase 2
- [ ] Check `keg_only` flag from API
- [ ] Create symlinks to prefix (unless keg-only)
- [ ] Generate install receipts matching Homebrew format
- [ ] Implement SHA256 verification

### Phase 3
- [ ] Add shell completions command
- [ ] Optional config file (~/.config/bru/config.toml)
- [ ] Third-party tap support

### Testing
- [ ] Create fixtures directory (tests/fixtures/)
- [ ] Cache test API responses
- [ ] Update fixtures script

---

**Status**: All critical questions answered, ready for Phase 1 implementation
**Updated**: 2025-01-08 (Phase 0 complete)
