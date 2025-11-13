# Command Migration Guide

This document describes the modular command structure and provides guidance for migrating remaining commands.

## Overview

Issue #3 proposed better separation of concerns for commands by splitting the large `commands.rs` file into individual files in `commands/`. This follows the idiomatic pattern used in cargo and rustc.

## Current Status

**Migrated Commands (4/80+):**
- `search` → `src/commands/search.rs`
- `info` → `src/commands/info.rs`
- `deps` → `src/commands/deps.rs`
- `uses` → `src/commands/uses.rs`

**Remaining Commands:**
- All other commands are still in `src/commands_old.rs`
- These are re-exported through `src/commands/mod.rs` for backwards compatibility

## Architecture

### Simple Module-Based Approach

Each command is a plain function in its own file. No traits, no structs (unless needed for complex state), just straightforward Rust modules.

#### Example: Search Command

```rust
// src/commands/search.rs
use crate::api::BrewApi;
use crate::error::Result;

pub async fn search(api: &BrewApi, query: &str, formula_only: bool, cask_only: bool) -> Result<()> {
    // Implementation
}
```

#### Usage in main.rs

```rust
match cli.command {
    Some(Commands::Search { query, formula, cask }) => {
        commands::search::search(&api, &query, formula, cask).await?;
    }
}
```

## Migration Process

### Step 1: Create New Command File

Create `src/commands/<command_name>.rs` with the command function.

### Step 2: Add to Module Declaration

Update `src/commands/mod.rs`:

```rust
pub mod <command_name>;
```

### Step 3: Update main.rs

Replace the old function call with the new module path:

```rust
// Old (from commands_old.rs):
commands::search(&api, &query, formula, cask).await?;

// New (from commands/search.rs):
commands::search::search(&api, &query, formula, cask).await?;
```

### Step 4: Test

```bash
# Build and test
cargo build
cargo test

# Manually test the command
./target/debug/bru <command> <args>
```

## Benefits

1. **Better Organization**: Each command in its own file
2. **Reduced Cognitive Load**: No 7800+ line file to navigate
3. **Better IDE Performance**: Smaller files = faster rust-analyzer
4. **Easier for Contributors**: Clear, isolated files
5. **Idiomatic Rust**: Follows patterns from cargo, rustc
6. **No Overhead**: No traits, no extra dependencies, just modules

## Helper Functions

Some commands use shared helper functions from `commands_old.rs`:
- `resolve_dependencies()` - Used by `deps` command
- `check_brew_available()` - Used by various commands
- `fallback_to_brew()` - Used by install commands
- `cleanup_specific_version()` - Used by upgrade commands
- `format_columns()` - Used by list commands

These should be moved to a separate `src/commands/utils.rs` module as more commands are migrated.

## Next Steps

### Priority 1: Core Commands (High Usage)
- [ ] `list` - Most frequently used read command
- [ ] `install` - Primary write command
- [ ] `upgrade` - Common maintenance command
- [ ] `uninstall` - Common cleanup command

### Priority 2: Installation-Related
- [ ] `fetch`
- [ ] `reinstall`
- [ ] `autoremove`
- [ ] `cleanup`
- [ ] `link`
- [ ] `unlink`

### Priority 3: Repository Management
- [ ] `tap`
- [ ] `untap`
- [ ] `tap_info`
- [ ] `update`

### Priority 4: Remaining Commands
- [ ] All other commands in `commands_old.rs`

## Completion

Once all commands are migrated:
1. Delete `src/commands_old.rs`
2. Remove the `#[path = "../commands_old.rs"]` import from `src/commands/mod.rs`
3. Move helper functions to `src/commands/utils.rs`
4. Update documentation

## Testing Strategy

For each migrated command:
1. Build succeeds
2. Unit tests pass (if any)
3. Manual testing shows identical behavior to old implementation
4. Integration tests pass

## Questions?

See the issue discussion: https://github.com/nijaru/kombrucha/issues/3
