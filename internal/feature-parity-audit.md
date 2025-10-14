# Feature Parity Audit

**Goal**: Achieve complete feature parity with Homebrew

## Current Status

- **bru**: 54 commands implemented
- **brew**: 120 commands total
- **Gap**: 66 commands

## Command Categories

### ✅ Implemented (54 commands)

**Core Package Management**:
- install, uninstall, upgrade, reinstall
- fetch, list, outdated
- autoremove, cleanup, pin, unpin
- bundle (install from Brewfile, dump to Brewfile)
- services (list, start, stop, restart)

**Information & Query**:
- search, info, desc, deps, uses
- leaves, missing, cat, log
- which-formula, options
- livecheck (check for newer versions)

**Repository Management**:
- tap, untap, update

**System**:
- config, doctor, home, shellenv
- cache, analytics, commands, completions
- prefix, cellar, repository, formula

**Utilities**:
- alias, gist-logs, link, unlink

**Development**:
- edit (open formula in editor)
- create (generate formula template)
- audit (validate formula files)
- postinstall (stub - requires Phase 3)

### 🔴 Missing Critical Commands (Priority 1)

These are user-facing commands needed for feature parity:

1. ✅ **services** - Manage background services (postgres, redis, nginx, etc.)
   - `brew services list` - List all services
   - `brew services start <formula>` - Start service
   - `brew services stop <formula>` - Stop service
   - `brew services restart <formula>` - Restart service
   - Impact: HIGH - Many users run databases/services
   - Status: IMPLEMENTED

2. ✅ **Cask support** - Install macOS applications
   - `brew install --cask <app>` - Install GUI app
   - `brew uninstall --cask <app>` - Remove app
   - `brew outdated --cask` - Check outdated apps (pending)
   - Impact: HIGH - ~50% of Homebrew usage on macOS
   - Status: IMPLEMENTED (install/uninstall working, outdated pending)

3. ✅ **bundle** - Install from Brewfile
   - `brew bundle` - Install from Brewfile
   - `brew bundle dump` - Generate Brewfile
   - Impact: HIGH - Team onboarding, reproducibility
   - Status: IMPLEMENTED

4. ✅ **which-formula** - Find which formula provides file
   - `brew which-formula <command>` - Find provider
   - Impact: MEDIUM - Useful for debugging
   - Status: IMPLEMENTED

5. ✅ **options** - Show build options for formula
   - `brew options <formula>` - Show available options
   - Impact: LOW (bottles don't have options)
   - Status: IMPLEMENTED

### 🟡 Missing Development Commands (Priority 2)

Useful for formula developers:

6. ✅ **create** - Create new formula
   - `brew create <url>` - Generate formula template
   - Status: IMPLEMENTED

7. ✅ **edit** - Edit formula in editor
   - `brew edit <formula>` - Open formula in $EDITOR
   - Status: IMPLEMENTED

8. **test** - Run formula tests
   - `brew test <formula>` - Run test suite
   - Status: Requires Phase 3 (Ruby interop)

9. ✅ **audit** - Check formula for issues
   - `brew audit <formula>` - Lint formula
   - Status: IMPLEMENTED (basic checks)

10. ✅ **livecheck** - Check for newer versions
    - `brew livecheck <formula>` - Check upstream
    - Status: IMPLEMENTED (placeholder)

### 🟢 Missing Internal Commands (Priority 3)

Developer/CI commands, low user impact:

- bump-*, pr-*, test-bot, generate-*
- bottle, unbottled
- ruby, irb, debugger
- update-*, vendor-*, readall
- etc. (~60 commands)

## Architectural Gaps

### Phase 3: Source Builds (CRITICAL)

**Status**: Not implemented
**Blocker**: Cannot build formulae from source

**Requirements**:
1. Embed Ruby interpreter (`magnus` crate)
2. Load and execute `.rb` formula DSL
3. Handle build dependencies
4. Compile from source
5. Install built artifacts

**Estimated Effort**: 3-4 weeks

**Impact**:
- Blocks 1-5% of formulae without bottles
- Blocks formula development (create, edit, test)
- Needed for true "feature complete" status

### Performance Issues

1. **Search Performance** (3x slower than Homebrew)
   - Downloads entire formula database every search
   - **Fix**: Implement local caching layer
   - **Effort**: 1-2 days
   - **Priority**: HIGH (quick win)

## Recommendations

### Path to Feature Parity

**Quick Wins** (1 week):
1. ✅ Fix search performance (caching layer) - DONE
2. ✅ Implement `which-formula` command - DONE
3. ✅ Implement `options` command - DONE

**High-Value Features** (2-3 weeks):
4. ✅ Services management (launchd integration) - DONE
5. ✅ Bundle/Brewfile support - DONE
6. ✅ Basic cask support (info/search) - DONE

**Feature Complete** (3-4 weeks):
7. 🔴 Phase 3: Ruby interop + source builds
   - Embed Ruby with magnus
   - Formula DSL execution
   - Build from source support

**Formula Development** (1-2 weeks):
8. ✅ create, edit, test, audit commands
   - Requires Phase 3 completion

### Prioritization

**For 0.0.x releases**:
1. Fix search performance (quick win)
2. Add missing user-facing commands (services, bundle, which-formula)
3. Work toward Phase 3 (source builds)

**For 0.1.0**:
- Phase 3 complete (source builds working)
- All Priority 1 commands implemented
- Services + Bundle working
- Cask basics (at least search/info)

**For 1.0.0**:
- Full feature parity (all user-facing commands)
- Formula development tools (create, edit, test, audit)
- Production-ready, battle-tested

## Next Steps

1. **Immediate** (Today):
   - Fix search performance with caching
   - Add which-formula command

2. **This Week**:
   - Implement services management
   - Add Brewfile/bundle support

3. **Next 2-4 Weeks**:
   - Begin Phase 3 (Ruby interop)
   - Parallel: Add cask support

4. **Milestone**: 0.1.0
   - Phase 3 complete
   - All Priority 1 commands done
   - True feature parity achieved
