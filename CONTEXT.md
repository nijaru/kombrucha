# Command Migration - Active Work Context

## What We're Doing

Splitting the large `commands.rs` file (7,808 lines, 80+ commands) into individual module files under `src/commands/`. This addresses GitHub issue #3: https://github.com/nijaru/kombrucha/issues/3

## Why

- **Better organization**: Each command in its own file
- **Reduced cognitive load**: No need to scroll through 7,800+ lines
- **Better rust-analyzer performance**: Smaller files = faster IDE
- **Easier for contributors**: Clear, isolated files to work with
- **Idiomatic Rust**: Follows patterns from cargo and rustc

## Current Status

**Branch**: `feature/split-commands`

**Migrated Commands (4/80+)**:
- `search` → `src/commands/search.rs`
- `info` → `src/commands/info.rs`
- `deps` → `src/commands/deps.rs`
- `uses` → `src/commands/uses.rs`

**Remaining**: ~76 commands still in `src/commands_old.rs`

**Testing**: All tests passing (8/8), manual testing confirms identical behavior

## Architecture Decision

After initial exploration with traits, we settled on **simple module functions** (no traits, no structs unless needed for state). This is:
- Simpler and more maintainable
- Zero overhead (no extra dependencies)
- Idiomatic (how cargo/rustc do it)
- Achieves all the original goals

## Pattern to Follow

### 1. Create Command File

Example: `src/commands/list.rs`

```rust
use crate::api::BrewApi;
use crate::error::Result;
// ... other imports as needed

pub async fn list(
    _api: &BrewApi,
    show_versions: bool,
    json: bool,
    cask: bool,
    quiet: bool,
    columns: bool,
) -> Result<()> {
    // Copy implementation from commands_old.rs
    // Keep it identical - no changes to logic
}
```

### 2. Update `src/commands/mod.rs`

Add module declaration:
```rust
pub mod list;
```

### 3. Update `src/main.rs`

Change the match arm:
```rust
// Old:
commands::list(&api, versions, json, cask, quiet, columns).await?;

// New:
commands::list::list(&api, versions, json, cask, quiet, columns).await?;
```

### 4. Test

```bash
cargo build
cargo test
./target/debug/bru list  # Manual test
```

## Commands to Migrate (Priority Order)

### Priority 1: Core Commands (High Usage)
- [ ] `list` (line 622)
- [ ] `install` (line 1198)
- [ ] `upgrade` (line 1698)
- [ ] `uninstall` (line 2340)

### Priority 2: Installation-Related
- [ ] `fetch` (line 1113)
- [ ] `reinstall` (line 2135)
- [ ] `autoremove` (line 2455)
- [ ] `cleanup` (line 2847)
- [ ] `link` (line 3606)
- [ ] `unlink` (line 3669)

### Priority 3: Repository Management
- [ ] `tap` (line 2572)
- [ ] `untap` (line 2606)
- [ ] `tap_info` (line 2625)
- [ ] `update` (line 2713)

### Priority 4: Remaining Commands
- [ ] `outdated` (line 927)
- [ ] `cache` (line 3035)
- [ ] `config` (line 3146)
- [ ] `env` (line 3204)
- [ ] `doctor` (line 3226)
- [ ] `home` (line 3385)
- [ ] `leaves` (line 3419)
- [ ] `pin` (line 3524)
- [ ] `unpin` (line 3555)
- [ ] `desc` (line 3579)
- [ ] `commands` (line 3714)
- [ ] `missing` (line 3781)
- [ ] `analytics` (line 3845)
- [ ] `cat` (line 3889)
- [ ] `shellenv` (line 3932)
- [ ] `gist_logs` (line 4017)
- [ ] `alias` (line 4128)
- [ ] `log` (line 4208)
- [ ] `which_formula` (line 4323)
- [ ] `options` (line 4371)
- [ ] `bundle` (line 4400)
- [ ] `services` (line 4552)
- [ ] `edit` (line 4694)
- [ ] `create` (line 4800)
- [ ] `livecheck` (line 4896)
- [ ] `audit` (line 4926)
- [ ] `install_cask` (line 5035)
- [ ] `reinstall_cask` (line 5257)
- [ ] `cleanup_cask` (line 5290)
- [ ] `upgrade_cask` (line 5411)
- [ ] `uninstall_cask` (line 5489)
- [ ] `prefix` (line 5584)
- [ ] `cellar_cmd` (line 5623)
- [ ] `repository` (line 5655)
- [ ] `formula` (line 5707)
- [ ] `postinstall` (line 5754)
- [ ] `formulae` (line 5807)
- [ ] `casks` (line 5847)
- [ ] `unbottled` (line 5887)
- [ ] `docs` (line 5954)
- [ ] `tap_new` (line 5972)
- [ ] `migrate` (line 6024)
- [ ] `linkage` (line 6052)
- [ ] `readall` (line 6104)
- [ ] `extract` (line 6135)
- [ ] `unpack` (line 6169)
- [ ] `command_not_found_init` (line 6211)
- [ ] `man` (line 6260)
- [ ] `update_reset` (line 6279)
- [ ] `style` (line 6300)
- [ ] `test` (line 6344)
- [ ] `bottle` (line 6372)
- [ ] `tap_pin` (line 6410)
- [ ] `tap_unpin` (line 6431)
- [ ] `vendor_gems` (line 6452)
- [ ] `ruby` (line 6474)
- [ ] `irb` (line 6505)
- [ ] `prof` (line 6527)
- [ ] `tap_readme` (line 6552)
- [ ] `install_bundler_gems` (line 6576)
- [ ] `developer` (line 6598)
- [ ] `typecheck` (line 6635)
- [ ] `update_report` (line 6664)
- [ ] `update_python_resources` (line 6686)
- [ ] `determine_test_runners` (line 6717)
- [ ] `dispatch_build_bottle` (line 6745)
- [ ] `bump_formula_pr` (line 6776)
- [ ] `bump_cask_pr` (line 6819)
- [ ] `generate_formula_api` (line 6852)
- [ ] `generate_cask_api` (line 6880)
- [ ] `pr_pull` (line 6908)
- [ ] `pr_upload` (line 6932)
- [ ] `test_bot` (line 6960)
- [ ] `bump_revision` (line 6995)
- [ ] `pr_automerge` (line 7027)
- [ ] `contributions` (line 7054)
- [ ] `update_license_data` (line 7088)
- [ ] `formula_info` (line 7110)
- [ ] `tap_cmd` (line 7145)
- [ ] `install_formula_api` (line 7182)
- [ ] `uses_cask` (line 7204)
- [ ] `abv_cask` (line 7229)
- [ ] `setup` (line 7257)
- [ ] `fix_bottle_tags` (line 7279)
- [ ] `generate_man_completions` (line 7306)
- [ ] `bottle_merge` (line 7328)
- [ ] `install_bundler` (line 7356)
- [ ] `bump` (line 7378)
- [ ] `analytics_state` (line 7407)
- [ ] `sponsor` (line 7429)
- [ ] `command` (line 7457)
- [ ] `nodenv_sync` (line 7481)
- [ ] `pyenv_sync` (line 7505)
- [ ] `rbenv_sync` (line 7529)
- [ ] `setup_ruby` (line 7553)
- [ ] `tab` (line 7575)
- [ ] `unalias` (line 7607)

## Helper Functions to Handle

These are shared utilities in `commands_old.rs` that commands depend on:

- `check_brew_available()` (line 13)
- `is_tap_formula()` (line 23)
- `fallback_to_brew()` (line 40)
- `fallback_to_brew_with_reason()` (line 45)
- `cleanup_specific_version()` (line 85)
- `resolve_dependencies()` (line 1548) - **Already made public**
- `format_columns()` (line 579)

**Strategy**: Leave these in `commands_old.rs` for now, they're re-exported. After all commands are migrated, move them to `src/commands/utils.rs`.

## File Locations

- **Old commands**: `src/commands_old.rs` (7,808 lines)
- **New commands**: `src/commands/<name>.rs`
- **Module file**: `src/commands/mod.rs`
- **Main router**: `src/main.rs` (match statement starting at line 962)
- **Migration guide**: `docs/COMMAND_MIGRATION.md`

## Current File Structure

```
src/
├── commands/
│   ├── mod.rs          # Module declarations + re-exports old commands
│   ├── search.rs       # ✅ Migrated
│   ├── info.rs         # ✅ Migrated
│   ├── deps.rs         # ✅ Migrated
│   └── uses.rs         # ✅ Migrated
├── commands_old.rs     # 76+ commands remaining
└── main.rs             # Route commands (update match arms as you migrate)
```

## Migration Workflow

1. **Pick a command** from the priority list
2. **Find it** in `src/commands_old.rs` (use line numbers above)
3. **Copy the function** to `src/commands/<name>.rs`
4. **Update imports** (use `crate::error::Result` instead of `Result`)
5. **Add module** to `src/commands/mod.rs`: `pub mod <name>;`
6. **Update main.rs** match arm to call `commands::<name>::<name>(...)`
7. **Build & test**: `cargo build && cargo test && ./target/debug/bru <command> <args>`
8. **Repeat** for next command

## Automation Opportunity

Since this is mechanical work, you could:
- Batch process multiple commands at once
- Use regex/scripting to automate the repetitive parts
- Focus on getting it done quickly rather than perfectly (we'll compact later)

## After Migration Complete

1. **Compact**: Move helper functions to `src/commands/utils.rs`
2. **Delete**: Remove `src/commands_old.rs`
3. **Clean up**: Remove old command re-exports from `src/commands/mod.rs`
4. **Fix CI**: Address `.github/workflows/ci.yml` issues
5. **Open PR**: Create pull request with changes

## CI Issues

The CI workflow (`.github/workflows/ci.yml`) has been failing even with working code. This needs investigation after migration is complete. Possible issues:
- Timeout problems
- Missing dependencies in CI environment
- Test flakiness
- Workflow configuration errors

## Testing Strategy

- **Unit tests**: `cargo test --lib` (currently 8 tests, all passing)
- **Build**: `cargo build` should succeed with only warnings about unused functions in commands_old.rs
- **Manual**: Test each migrated command manually to ensure identical behavior
- **Integration tests**: Currently ignored, may need fixes

## Notes

- Keep logic **identical** during migration - no refactoring, no improvements
- Only change: move code to new file and update imports
- The warnings about "function X is never used" are expected for migrated commands
- All changes are backwards compatible - old and new commands coexist
- No version bump until this is merged

## GitHub Issue

Issue #3: https://github.com/nijaru/kombrucha/issues/3

Comment made explaining the approach - keep it updated as we progress.

## Quick Reference Commands

```bash
# Build
cargo build

# Test
cargo test --lib

# Test specific command
./target/debug/bru <command> <args>

# Check migration progress
grep "pub mod" src/commands/mod.rs | wc -l

# Find a command in old file
grep -n "^pub async fn <name>" src/commands_old.rs
```

## Current Branch Status

- Branch: `feature/split-commands`
- 4 commands migrated
- All tests passing
- Ready to continue migration
