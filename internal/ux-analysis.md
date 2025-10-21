# CLI UX Analysis & Recommendations

## Current State Analysis

### ✅ What We're Doing Well

**1. Progress Indicators**
```bash
⬇ Downloading bottles...
✓ tmux [████████████] 472.83 KiB (683.49 KiB/s)
✓ Downloaded 1 bottles in 806ms
```
- Clear visual feedback
- Speed metrics shown
- Timing information included
- Works great!

**2. Structured Output**
```bash
==> Formulae
rust (Safe, concurrent, practical language)
rust-analyzer (Experimental Rust compiler front-end for IDEs)
```
- Clear sections with headers
- Descriptions inline (better than brew's separate lines)
- Good information density

**3. Status Indicators**
```bash
✓ Installed 1 packages in 2.4s
openssl@3 ✓ installed (in search results)
```
- Clear success/failure states
- Inline status in search results (better than brew)
- Timing metrics everywhere

**4. Smart Features**
- Fuzzy search (handles typos)
- Result ranking (most relevant first)
- Platform fallback (works on newer macOS)
- Installed status in search

### ⚠️ Areas for Improvement

**1. Emoji Overuse**

**Current:**
```bash
🔍 Searching for: rust
✓ Found 137 results
📦 Installing 1 formulae...
⬇ Downloading bottles...
🔧 Installing packages...
```

**Issues:**
- Not all terminals/fonts support emojis
- Some users find them unprofessional
- Can look inconsistent across terminals
- Harder to grep/parse output

**Comparison with Modern CLIs:**

Cargo (Rust's package manager):
```bash
    Updating crates.io index
   Compiling rust v1.0.0
    Finished dev [unoptimized + debuginfo] target(s) in 2.3s
```
- Uses indentation, not emojis
- Clear hierarchy
- Professional appearance
- Works everywhere

UV (Python package manager):
```bash
Resolved 137 packages in 1.2s
Downloaded 5 packages in 800ms
Installed 5 packages in 1.4s
```
- No emojis at all
- Color for emphasis
- Clear, scannable

**Recommendation**: Add `--no-emoji` flag (or detect terminal capability)
```rust
pub fn should_use_emoji() -> bool {
    // Check NO_EMOJI env var
    if env::var("NO_EMOJI").is_ok() {
        return false;
    }

    // Check if terminal supports emojis
    // Most modern terminals do, but some don't
    true
}
```

**Alternative symbols without emojis:**
```bash
:: Searching for: rust
✓  Found 137 results          # ✓ is ASCII-ish
=> Installing 1 formulae...
-> Downloading bottles...
>> Installing packages...
```

**2. Color Usage**

**Current State:**
- Search: Cyan for package names, dimmed for descriptions ✅
- Install: Bold for emphasis, green for success ✅
- List: ALL cyan (might be too much)

**Issue with `bru list`:**
```bash
# Every package name is cyan
abseil
age
aom
```

**Better approach:**
```bash
# No color for plain list (like brew)
# OR use color sparingly for installed state
abseil      1.20240116.0
age         1.1.1
tmux        3.5a         ✓ current
rust        1.75.0       (outdated: 1.76.0 available)
```

**Recommendation**:
- `bru list`: No color (matches brew, pipe-friendly)
- `bru list --tree` or `bru list --verbose`: Use color for hierarchy
- Always honor `NO_COLOR` environment variable

**3. Output Consistency**

**Current inconsistency:**
- Search: "Found X results"
- Install: "Installed X packages"
- Download: "Downloaded X bottles"
- Update: "Updated X taps"

✅ This is actually good - specific terminology for each operation

**But check:**
- Singular vs plural handling
- "formulae" vs "packages" (we use both)

**Recommendation**: Be consistent
- "formulae" when specifically formulae
- "packages" as generic term
- "X formula" (singular) vs "X formulae" (plural)

**4. Error Messages**

**Current error:**
```bash
Error: Error: No bottle available for yarn. Tried platforms: arm64_sequoia, arm64_sonoma, arm64_ventura

Caused by:
    No bottle available for yarn. Tried platforms: arm64_sequoia, arm64_sonoma, arm64_ventura

Stack backtrace:
   0: __mh_execute_header
   ...
```

**Issues:**
- "Error: Error:" (duplicate)
- Stack trace not helpful to users
- No suggested fix

**Better approach:**
```bash
Error: No bottle available for 'yarn'

Tried platforms:
  • arm64_sequoia
  • arm64_sonoma
  • arm64_ventura
  • all

This package may need to be built from source.
Try: bru install --build-from-source yarn

For more details, run with --verbose
```

**Comparison with Cargo errors:**
```bash
error: package `foo` cannot be built because it requires rustc 1.70.0 or newer
       current version is 1.69.0

help: update rust with `rustup update`
```
- Clear error
- Explains why
- Suggests fix
- No stack trace unless --verbose

**5. Table Format for List**

**Current `bru list`:**
```
abseil
age
aom
```

**Better with `--verbose` or by default:**
```
NAME              VERSION        SIZE     INSTALLED
abseil            20240116.0     2.1 MB   2024-10-15
age               1.1.1          1.5 MB   2024-09-20
tmux              3.5a           149 KB   today
```

**Even better - cargo style:**
```
abseil 20240116.0        (2.1 MB)
age 1.1.1                (1.5 MB)
tmux 3.5a                (149 KB, installed today)
```

## Modern CLI Best Practices

### 1. **12 Factor CLI Apps**

✅ **We follow:**
- Store config in environment (NO_COLOR, HOMEBREW_PREFIX)
- Exit codes (0 = success, non-zero = error)
- Stream output (not buffered)

❌ **We should add:**
- Respect `--quiet` flag everywhere
- Structured output with `--json` for all commands
- `--help` shows examples

### 2. **GNU CLI Standards**

✅ **We follow:**
- Long form flags (`--verbose` not just `-v`)
- Help text available
- Consistent flag naming

### 3. **User-Friendly Output**

**Principles from `uv`, `cargo`, `git`:**

1. **Concise by default, verbose on demand**
   ```bash
   bru install rust          # Just "✓ Installed rust 1.76.0"
   bru install rust -v       # Show all details
   ```

2. **Timing for long operations**
   ✅ We do this! "in 2.4s"

3. **Show what's happening**
   ✅ We do this! Progress bars + status

4. **Actionable errors**
   ❌ Need to add suggested fixes

5. **No decorative output in scripts**
   - Check if stdout is TTY
   - Disable colors/progress if piped
   ✅ We use `atty` for this

### 4. **Information Hierarchy**

**Good example from our output:**
```bash
📦 Installing 1 formulae...           # Top level

🔍 Resolving dependencies...          # Phase
→ 1 formulae to install: tmux         # Detail

⬇ Downloading bottles...              # Phase
✓ tmux [████████] 472.83 KiB          # Detail

✓ Installed 1 packages in 2.4s        # Summary
```

Clear hierarchy: Operation → Phase → Details → Summary

**Could improve:**
- Use indentation instead of emojis
- More consistent symbols

## Specific Recommendations

### Priority 1: Fix Errors

1. **Remove duplicate "Error: Error:"**
2. **Hide stack traces by default**
3. **Add helpful suggestions**
4. **Show "Run with --verbose for details"**

### Priority 2: Add Flags

1. **`--no-emoji` flag** (or auto-detect)
2. **`--quiet` flag** for all commands
3. **`--verbose` flag** for debug output
4. **Honor `NO_COLOR` env var** everywhere

### Priority 3: Improve List

1. **Plain output by default** (like brew)
2. **Add `--tree` flag** for dependency trees
3. **Add `--long` or `-l` flag** for detailed table
4. **Add version/size info** in detailed mode

### Priority 4: Polish

1. **Consistent terminology** (formulae vs packages)
2. **Better error messages** with suggestions
3. **Show "Did you mean..." for typos** in commands
4. **Add examples to --help**

## Comparison Matrix

| Feature | Brew | Bru (Current) | Cargo | UV | Recommendation |
|---------|------|---------------|-------|----|----|
| **Emojis** | None | Heavy | None | None | Make optional |
| **Colors** | Minimal | Heavy | Moderate | Moderate | Reduce for list |
| **Progress** | Basic | Excellent ✓ | Excellent | Excellent | Keep current |
| **Timing** | No | Yes ✓ | Yes | Yes | Keep current |
| **Errors** | Basic | Stack traces ❌ | Excellent | Excellent | Improve |
| **JSON output** | Some | Some | N/A | Some | Add everywhere |
| **Help text** | Minimal | Good | Excellent | Excellent | Add examples |
| **Quiet mode** | Yes | No ❌ | Yes | Yes | Add |

## Examples of Great CLI UX

### Cargo (Rust)
```bash
$ cargo build
   Compiling foo v0.1.0
    Finished dev [unoptimized] target(s) in 0.5s
```
- Clear indentation hierarchy
- No emojis, works everywhere
- Professional look

### UV (Python)
```bash
$ uv pip install requests
Resolved 5 packages in 120ms
Downloaded 5 packages in 450ms
Installed 5 packages in 80ms
 + requests==2.31.0
 + urllib3==2.1.0
 + certifi==2023.11.17
 + charset-normalizer==3.3.2
 + idna==3.6
```
- Consistent timing format
- Shows what was installed
- Clean, scannable

### Git
```bash
$ git commit -m "fix"
[main abc1234] fix
 1 file changed, 2 insertions(+), 1 deletion(-)
```
- Minimal output
- Just the facts
- No decoration

## Action Items

### Immediate (Before 1.0)
- [ ] Fix "Error: Error:" duplication
- [ ] Hide stack traces (show with --verbose)
- [ ] Honor NO_COLOR environment variable
- [ ] Add --quiet flag
- [ ] Reduce color in `bru list` (match brew)

### Short Term
- [ ] Add --no-emoji flag
- [ ] Better error messages with suggestions
- [ ] Add --verbose flag for debug info
- [ ] Improve help text with examples
- [ ] Add --json to more commands

### Future
- [ ] Table format for list -l
- [ ] Dependency tree visualization
- [ ] "Did you mean..." for typos
- [ ] Detect terminal capabilities
- [ ] Smart column width detection

## Testing UX Changes

### Before/After Comparison

Test each change with:
```bash
# Test in different environments
bru list                    # Normal terminal
bru list | head             # Piped
NO_COLOR=1 bru list        # No color
bru list --quiet            # Quiet mode
bru list --json            # Machine readable
```

### User Testing

Ask questions:
1. Is the output clear?
2. Can you quickly find what you need?
3. Are errors helpful?
4. Does it work in your terminal?
5. Is timing information useful?

## Conclusion

**Overall: Our UX is good, but could be more professional**

Strengths:
- ✅ Progress indicators
- ✅ Timing metrics
- ✅ Smart features (fuzzy search, ranking)
- ✅ Installed status

Weaknesses:
- ❌ Too many emojis (not universal)
- ❌ Error messages need work
- ❌ Missing common flags (--quiet, --verbose)
- ❌ Color overuse in some commands

**Priority**: Fix errors, add flags, make emojis optional.
