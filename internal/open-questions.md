# Open Questions & Research Needed

This document tracks things we need to investigate, decide, or research before implementing.

## 🔴 Critical (Need answers before Phase 1)

### 1. Homebrew Prefix & Cellar Paths

**Question**: Where does Homebrew install things? Where should bru install things?

**Why it matters**:
- Must be compatible with existing Homebrew installations
- Users might have both `brew` and `bru` installed
- Installation paths are hardcoded in bottles/receipts

**Need to research**:
- `/opt/homebrew` (Apple Silicon) vs `/usr/local` (Intel)
- Cellar structure: `/opt/homebrew/Cellar/wget/1.21.4/`
- Symlink targets: `/opt/homebrew/bin/wget` → `../Cellar/wget/1.21.4/bin/wget`
- Prefix detection: How to auto-detect user's Homebrew prefix?
- Can we install to same location as Homebrew? (Probably yes)
- What if user doesn't have Homebrew installed? (Create standard prefix)

**Action**:
- [ ] Read Homebrew prefix detection code
- [ ] Test on both Intel and Apple Silicon
- [ ] Document in `internal/homebrew-compatibility.md`

### 2. API Rate Limiting & Caching

**Question**: Will we hit rate limits? Should we cache API responses?

**Why it matters**:
- formulae.brew.sh might rate limit aggressive usage
- Users might run `bru search` repeatedly
- Integration tests hit API many times

**Need to research**:
- Does formulae.brew.sh have rate limits?
- Should we cache API responses locally?
- Where to store cache? (`~/.cache/bru/` or `~/.bru/cache/`)
- Cache expiration strategy?
- Cache invalidation (after `bru update`)?

**Observations**:
- API responses are large (~7MB for all formulae)
- Responses are stable (change only on formula updates)
- `brew` must cache these somehow

**Proposed solution**:
```
~/.cache/bru/
  api/
    formulae.json (cached, 1 hour TTL)
    casks.json (cached, 1 hour TTL)
    formula/wget.json (cached, 1 day TTL)
```

**Action**:
- [ ] Check if Homebrew API has rate limits
- [ ] Implement simple file-based cache
- [ ] Add `--no-cache` flag for testing

### 3. Bottle Checksums & Security

**Question**: How do we verify bottle integrity? What checksums does Homebrew use?

**Why it matters**:
- Security: Don't want to install compromised bottles
- Reliability: Detect corrupted downloads
- Compatibility: Must match Homebrew's verification

**Need to research**:
- What hash algorithm? (SHA256? SHA512?)
- Where are checksums stored? (In API response? In bottle?)
- How does Homebrew verify bottles?
- Do we need GPG signature verification?
- What happens on checksum mismatch?

**Action**:
- [ ] Read Homebrew bottle verification code
- [ ] Test bottle download and checksum verification
- [ ] Implement in Phase 2

### 4. Configuration System

**Question**: Does bru need configuration? Where should it live?

**Why it matters**:
- Users might want to customize behavior
- Need to store API cache location
- Need to store Homebrew prefix override

**Possible config locations**:
- `~/.config/bru/config.toml` (XDG spec)
- `~/.bru.toml` (simple)
- `/opt/homebrew/etc/bru/config` (Homebrew-style)
- Environment variables only (HOMEBREW_* style)

**Possible settings**:
```toml
[api]
cache_dir = "~/.cache/bru"
cache_ttl = 3600  # seconds

[install]
prefix = "/opt/homebrew"  # auto-detect if not set
cellar = "/opt/homebrew/Cellar"
auto_update = false

[network]
concurrency = 10  # parallel downloads
timeout = 30  # seconds

[output]
color = true
verbose = false
```

**Decision**: Start simple, add config as needed
- Phase 0-2: No config file, use sensible defaults
- Phase 3+: Add config file if users request it

**Action**:
- [ ] Defer until Phase 2
- [ ] For now: auto-detect Homebrew prefix, use defaults

## 🟡 Important (Need answers before Phase 2)

### 5. Installation Receipts Format

**Question**: What format are Homebrew install receipts? Are they compatible with bru?

**Need to research**:
- Receipt location: `/opt/homebrew/Cellar/wget/1.21.4/INSTALL_RECEIPT.json`?
- Receipt format: JSON? What fields?
- Does `brew list` parse these receipts?
- Can bru-installed packages appear in `brew list`?

**Why it matters**:
- Users might want to use both `brew` and `bru`
- Receipts track what's installed
- `brew uninstall` needs to work on bru-installed packages

**Action**:
- [ ] Examine receipt format in Phase 2
- [ ] Ensure compatibility

### 6. Tap Management

**Question**: How do taps work? Do we need to support custom taps in Phase 1-2?

**Current understanding**:
- Taps are Git repos with formulae
- Core: homebrew/core (built-in)
- Others: `brew tap user/repo` clones to `/opt/homebrew/Library/Taps/`

**For Phase 1-2**:
- Only support homebrew-core (default)
- Tap management deferred to Phase 3+

**Action**:
- [ ] Defer tap support until Phase 3
- [ ] Document in roadmap

### 7. Keg-Only Formulae Handling

**Question**: How do keg-only formulae work? How should bru handle them?

**What we know**:
- Some formulae are keg-only (not symlinked to prefix)
- Example: `openssl@3` is keg-only to avoid conflicts with system OpenSSL
- Other formulae depend on keg-only formulae

**Questions**:
- How does Homebrew track keg-only status?
- How do dependent formulae find keg-only deps?
- Do we need special handling in Phase 2?

**Action**:
- [ ] Research in Phase 1
- [ ] Test with openssl@3 and dependents

### 8. Dependency Conflict Resolution

**Question**: What happens when two packages need different versions of a dep?

**Homebrew's approach** (from research):
- Forces everything to latest version
- No version flexibility
- Upgrades cascade to dependents

**Should bru be different?**
- Option A: Match Homebrew exactly (simple, compatible)
- Option B: Allow version flexibility (complex, better)

**Decision for Phase 1-2**: Match Homebrew (Option A)
- Defer advanced dep resolution to Phase 4
- Focus on compatibility first

**Action**:
- [ ] Document decision in implementation-roadmap.md
- [ ] Consider Phase 4 enhancement

## 🟢 Nice to Have (Can research anytime)

### 9. Shell Completions

**Question**: Should bru provide shell completions for bash/zsh/fish?

**Why it matters**: Better UX

**Implementation**: clap can generate completions automatically

**Action**:
- [ ] Add in Phase 3 or later
- [ ] Easy to implement with clap

### 10. Progress Bars for Downloads

**Question**: What progress bar library should we use? How to show parallel downloads?

**Options**:
- `indicatif` (already in Cargo.toml) - great for this
- Show multiple progress bars for concurrent downloads
- Show overall progress

**Action**:
- [ ] Implement in Phase 2 with download manager
- [ ] `indicatif` is perfect for this

### 11. Homebrew API Alternatives

**Question**: Are there other package metadata sources besides formulae.brew.sh?

**Possible alternatives**:
- Clone homebrew-core repo locally (slow, disk space)
- Use CDN/mirror (faster for some regions)
- Build our own index (overkill)

**Decision**: Stick with formulae.brew.sh API
- It's fast, reliable, official
- Has CDN backing
- Caching solves speed issues

**Action**:
- [ ] No action needed, use official API

### 12. Metrics & Telemetry

**Question**: Should bru collect usage metrics? (Homebrew does with HOMEBREW_NO_ANALYTICS)

**Privacy considerations**:
- Homebrew collects anonymous usage data
- We set `HOMEBREW_NO_ANALYTICS=1` in our dotfiles optimization
- Users value privacy

**Decision**: No telemetry for now
- Focus on building great software
- Users can opt-in later if desired

**Action**:
- [ ] No telemetry in MVP
- [ ] Reconsider in Phase 4+

## Research Methodology

When researching answers:

1. **Check Homebrew source code**:
   - https://github.com/Homebrew/brew
   - Focus on: `Library/Homebrew/`
   - Key files: `formula.rb`, `installer.rb`, `cellar.rb`

2. **Test with real Homebrew**:
   ```bash
   brew install wget --verbose
   brew info wget --json
   ls -la /opt/homebrew/Cellar/wget/
   ```

3. **Read Homebrew docs**:
   - https://docs.brew.sh/
   - https://docs.brew.sh/Formula-Cookbook
   - https://docs.brew.sh/Installation

4. **Test edge cases**:
   - What if Homebrew isn't installed?
   - What if prefix is custom?
   - What if network is slow/offline?

5. **Document findings**:
   - Create new docs in `internal/` as needed
   - Update `CLAUDE.md` with important decisions
   - Add to roadmap if implementation is needed

## Tracking

- 🔴 Red = Blocking / Critical
- 🟡 Yellow = Important but not blocking
- 🟢 Green = Nice to have / Low priority

Update this document as we answer questions and make decisions.

---

**Last updated**: Phase 0 complete
**Next review**: Start of Phase 1
