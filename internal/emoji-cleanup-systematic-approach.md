# Systematic Emoji Cleanup Approach

## Problem
Multiple automated attempts to remove all emojis from the codebase failed due to:
- Complex format string patterns
- Need to remove both emoji AND corresponding `{}` placeholder
- 30+ different emoji types across 200+ usages

## Better System: Incremental Verified Changes

###Step 1: Categorize by pattern type (grep analysis)
```bash
# Pattern 1: Simple replacement (emoji as only argument)
println!("{}", "emoji".bold()); → println!("");

# Pattern 2: Emoji as first arg (most common)
println!("{} Text {}", "emoji".bold(), other) → println!("Text {}", other)

# Pattern 3: Emoji in middle
println!("Text {} more {}", arg1, "emoji".bold(), arg2) → println!("Text {} more {}", arg1, arg2)

# Pattern 4: Multi-line format strings
println!("Text {}",
    "emoji".bold()) → println!("Text")
```

### Step 2: Count each pattern type
```bash
# Pattern 1 count
grep -c 'println!("{}", "[^"]*emoji' src/commands.rs

# Pattern 2 count
grep -c 'println!("{} [^"]*", "emoji"' src/commands.rs

# etc...
```

### Step 3: Fix one pattern type at a time
1. Create regex for JUST that pattern
2. Apply to test file
3. Run `cargo build`
4. If success: commit with message "refactor: remove emoji pattern X"
5. If failure: revert and refine regex
6. Move to next pattern

### Step 4: Handle edge cases individually
For any emoji that doesn't match patterns:
1. Find with `grep -n`
2. Manually fix in editor
3. Test build
4. Commit

## Execution Plan

### Phase 1: Replace error symbol (safe, proven)
- [x] Replace `❌` with `✗` across all files
- [x] Test build
- [x] Commit

### Phase 2: Remove Homebrew arrows (safe, proven)
- [x] Replace `==> Formulae` with `Formulae`
- [x] Replace `==> Casks` with `Casks`
- [x] Test build
- [x] Commit

### Phase 3: Simple emoji patterns (low risk)
Pattern: `println!("{}",  "emoji".method());`
- [ ] Find all instances
- [ ] Replace with `println!("");`
- [ ] Test build
- [ ] Commit

### Phase 4: First-position emojis (medium risk)
Pattern: `println!("{} Text {}", "emoji".method(), other_args)`
- [ ] Create specific regex
- [ ] Apply to one emoji type (e.g., 📦)
- [ ] Test build
- [ ] If works, apply to all emoji types
- [ ] Commit

### Phase 5: Complex patterns (manual)
- [ ] Find remaining with grep
- [ ] Fix each manually
- [ ] Test after each
- [ ] Commit

## Testing Strategy
After EACH change:
```bash
cargo build 2>&1 | tee /tmp/build.log
grep -E "(error|Finished)" /tmp/build.log
```

If errors: immediately `git restore src/commands.rs`

## Success Criteria
- Zero emojis in output: `grep -c '"[🎯🔍...]"' src/commands.rs` returns 0
- Build succeeds: `cargo build` exits 0
- All tests pass: `cargo test` exits 0

## Current Status
- ✗ symbol: ✅ Replaced
- ==> arrows: ✅ Removed
- Emoji cleanup: ⏸️ Paused - need to execute this plan

## Next Session
Start at Phase 3 with this documented approach.
