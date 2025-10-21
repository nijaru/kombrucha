# Progress Indicator Implementation Plan

## Design Philosophy

Based on research of cargo, uv, rustup, and modern CLIs, we'll implement:

1. **Phase-based progress** - Clear phases with timing
2. **Terminal integration** - Native progress for Ghostty/Windows Terminal/iTerm2
3. **Parallel visibility** - Show concurrent operations
4. **Speed metrics** - Quantify everything
5. **Professional polish** - Clean, consistent output

---

## Architecture

### New Module: `src/progress.rs`

```rust
pub struct ProgressManager {
    terminal_caps: TerminalCapabilities,
    start_time: Instant,
    current_phase: Option<Phase>,
}

pub enum TerminalCapabilities {
    OSC94,      // Ghostty, Windows Terminal
    ITerm2,     // iTerm2 specific
    None,       // Fallback
}

impl ProgressManager {
    pub fn new() -> Self;
    pub fn start_phase(&mut self, name: &str, total: Option<usize>);
    pub fn update(&mut self, current: usize);
    pub fn finish_phase(&mut self);
    pub fn set_terminal_progress(&self, percent: u8);
    pub fn clear_terminal_progress(&self);
}
```

### Terminal Capability Detection

```rust
fn detect_terminal_capabilities() -> TerminalCapabilities {
    // Check TERM_PROGRAM environment variable
    match env::var("TERM_PROGRAM").ok().as_deref() {
        Some("iTerm.app") => TerminalCapabilities::ITerm2,
        Some("ghostty") => TerminalCapabilities::OSC94,
        Some("WezTerm") => TerminalCapabilities::OSC94,
        _ => {
            // Check if we're in Windows Terminal
            if env::var("WT_SESSION").is_ok() {
                TerminalCapabilities::OSC94
            } else {
                // Default to OSC 9;4 (most compatible)
                // Gracefully ignored by unsupported terminals
                TerminalCapabilities::OSC94
            }
        }
    }
}
```

---

## Implementation by Command

### 1. Update Command (HIGHEST PRIORITY)

**Current**: Silent git operations
**New**: Show progress for each tap

```
🔄 Updating Homebrew...

⟳ Updating homebrew/core (1/7)...
✓ homebrew/core updated in 2.1s

⟳ Updating homebrew/cask (2/7)...
✓ homebrew/cask updated in 1.8s

✓ Updated 7 taps in 5.3s
```

**Implementation**:
- Spinner for each tap update
- Sequential git operations (can't parallel)
- Timer for each tap
- Overall summary
- Terminal progress: 0-100% across all taps

### 2. Outdated Command

**Current**: Silent API fetching
**New**: Show progress checking packages

```
🔍 Checking for outdated packages...

[████████████████████░░░] 150/200 packages (1.2s)

✓ Checked 200 packages in 1.5s
  Found 5 outdated packages
```

**Implementation**:
- Progress bar for API fetching
- Count of packages checked
- Time elapsed
- Terminal progress: based on packages checked

### 3. Download (ENHANCE EXISTING)

**Current**: Individual progress bars (good!)
**New**: Add terminal integration + summary

```
📥 Downloading 5 packages...

  ✓ wget [████████] 2.1MB/2.1MB (2.5MB/s)
  ⟳ curl [████░░░░] 1.2MB/3.5MB (1.8MB/s)
  □ git (queued)
  □ python@3.13 (queued)
  □ rust (queued)

✓ Downloaded 5 packages in 3.2s (avg 2.1MB/s)
```

**Implementation**:
- MultiProgress already exists
- Add summary with timing
- Terminal progress: based on packages downloaded
- Show average speed

### 4. Install/Upgrade Command

**Current**: Sequential output
**New**: Overall progress + phase tracking

```
⬆ Upgrading 10 packages...

Phase 1: Downloading
  [████████░░] 8/10 packages (2.3 MB/s)
✓ Downloaded 10 packages in 4.2s

Phase 2: Installing
  [██████████] 10/10 complete
✓ Installed 10 packages in 2.1s

✨ Done! Total time: 6.3s
```

**Implementation**:
- Track download phase
- Track install phase
- Overall progress bar
- Terminal progress: 0-50% for download, 50-100% for install
- Total time summary

### 5. Extraction (NEW)

**Current**: Silent
**New**: Show extraction progress

```
📦 Extracting packages...
  [████████████████░░░] 4/5 complete
✓ Extracted 5 packages in 0.8s
```

**Implementation**:
- Progress bar for extraction
- Can be parallel
- Terminal progress included

---

## Terminal Progress Integration

### OSC 9;4 (Ghostty, Windows Terminal)

```rust
pub fn set_terminal_progress_osc94(progress: u8, state: ProgressState) {
    let state_code = match state {
        ProgressState::Off => 0,
        ProgressState::Indeterminate => 1,
        ProgressState::Normal => 2,
        ProgressState::Error => 3,
        ProgressState::Warning => 4,
    };
    print!("\x1b]9;4;{};{}\x1b\\", state_code, progress);
    let _ = std::io::stdout().flush();
}

pub fn clear_terminal_progress_osc94() {
    set_terminal_progress_osc94(0, ProgressState::Off);
}
```

### iTerm2 Support

```rust
pub fn set_terminal_progress_iterm2(progress: u8) {
    print!("\x1b]1337;SetProgress={}\x1b\\", progress);
    let _ = std::io::stdout().flush();
}

pub fn clear_terminal_progress_iterm2() {
    print!("\x1b]1337;SetProgress=-1\x1b\\");
    let _ = std::io::stdout().flush();
}
```

### Unified Interface

```rust
impl ProgressManager {
    pub fn set_terminal_progress(&self, percent: u8) {
        match self.terminal_caps {
            TerminalCapabilities::OSC94 => {
                set_terminal_progress_osc94(percent, ProgressState::Normal);
            }
            TerminalCapabilities::ITerm2 => {
                set_terminal_progress_iterm2(percent);
            }
            TerminalCapabilities::None => {
                // No-op, gracefully degrade
            }
        }
    }
}
```

---

## Timing & Metrics

### Phase Timing

```rust
pub struct PhaseTimer {
    start: Instant,
}

impl PhaseTimer {
    pub fn start() -> Self {
        PhaseTimer { start: Instant::now() }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn elapsed_s(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    pub fn format_elapsed(&self) -> String {
        let elapsed = self.elapsed_ms();
        if elapsed < 1000 {
            format!("{}ms", elapsed)
        } else {
            format!("{:.1}s", elapsed as f64 / 1000.0)
        }
    }
}
```

### Speed Calculation

```rust
pub struct SpeedTracker {
    bytes: u64,
    start: Instant,
}

impl SpeedTracker {
    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes += bytes;
    }

    pub fn avg_speed(&self) -> f64 {
        let elapsed = self.start.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.bytes as f64 / elapsed / 1_000_000.0 // MB/s
        } else {
            0.0
        }
    }

    pub fn format_speed(&self) -> String {
        format!("{:.1} MB/s", self.avg_speed())
    }
}
```

---

## Error Handling

### Progress on Error

When errors occur, clean up gracefully:

```rust
impl Drop for ProgressManager {
    fn drop(&mut self) {
        // Always clear terminal progress on drop
        self.clear_terminal_progress();
    }
}
```

### Error States

```rust
// Set terminal to error state
manager.set_terminal_progress_with_state(100, ProgressState::Error);
```

Shows red progress in supported terminals.

---

## Configuration

### Disable Progress

Respect `--quiet` flag and `NO_COLOR`:

```rust
pub fn should_show_progress() -> bool {
    // Disable if piping output
    !atty::isnt(atty::Stream::Stdout)
    // Disable if NO_COLOR set
    && env::var("NO_COLOR").is_err()
    // Disable if --quiet flag
    && !args.quiet
}
```

---

## Testing Strategy

### Manual Testing

Test in multiple terminals:
- [x] Ghostty - OSC 9;4 support
- [ ] iTerm2 - OSC 1337 support
- [ ] Terminal.app - Graceful degradation
- [ ] Alacritty - Graceful degradation
- [ ] VS Code terminal - Test compatibility

### Edge Cases

- [ ] Very fast operations (< 100ms)
- [ ] Very slow operations (> 60s)
- [ ] Interrupted operations (Ctrl+C)
- [ ] Piped output (should disable)
- [ ] Non-TTY output (CI, scripts)

---

## Implementation Order

### Phase 1: Foundation (30 min)
1. Create `src/progress.rs`
2. Implement `ProgressManager`
3. Add terminal capability detection
4. Add terminal progress functions

### Phase 2: Update Command (20 min)
1. Add spinner for tap updates
2. Add timing for each tap
3. Add terminal progress
4. Test with `bru update`

### Phase 3: Outdated Command (15 min)
1. Add progress bar for API fetching
2. Add timing
3. Add terminal progress
4. Test with many packages

### Phase 4: Download Enhancement (10 min)
1. Add terminal progress to existing bars
2. Add summary statistics
3. Add average speed calculation

### Phase 5: Install/Upgrade (20 min)
1. Add overall progress tracking
2. Add phase indicators
3. Add terminal progress
4. Add total time summary

### Phase 6: Testing & Polish (15 min)
1. Test in different terminals
2. Test error handling
3. Test edge cases
4. Update documentation

**Total Estimated Time**: ~2 hours

---

## Success Criteria

- ✅ No silent periods > 1 second
- ✅ Terminal progress works in Ghostty
- ✅ iTerm2 support functional
- ✅ Graceful degradation in other terminals
- ✅ All timings shown
- ✅ Parallel operations visible
- ✅ Professional appearance
- ✅ No crashes on Ctrl+C
- ✅ Works when piped (disables cleanly)

---

## Future Enhancements

### Post-MVP
- [ ] Configurable progress bar styles
- [ ] Progress log file for debugging
- [ ] Estimate total time based on history
- [ ] Show package sizes before download
- [ ] Retry indicators for failed downloads

### Advanced
- [ ] Rich protocol support (more detailed)
- [ ] Custom themes
- [ ] Progress persistence across runs
- [ ] Analytics on operation times
