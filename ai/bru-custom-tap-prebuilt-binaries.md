# Bru: Custom Taps with Prebuilt Binaries

## Summary

Bru currently falls back to `brew` for custom tap packages even when they have prebuilt binaries, treating them as "requires building from source". This results in slower upgrades for packages that actually have fast prebuilt binary installs.

## Observed Behavior

```bash
❯ bru upgrade
Found 4 outdated packages: fmt, sy, openjph, vercel-cli
Upgrading fmt 11.2.0 -> 12.1.0
Upgrading openjph 0.24.4 -> 0.24.5
Upgrading vercel-cli 48.7.1 -> 48.8.0
Downloading 3 bottles...
✓ fmt [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━] 276.42 KiB/276.42 KiB (0s)
✓ openjph [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━] 154.11 KiB/154.11 KiB (0s)
✓ vercel-cli [━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━] 40.74 MiB/40.74 MiB (0s)

Upgrading 1 tap packages via brew...
  nijaru/tap/sy requires building from source (no bottle available)
  Falling back to brew upgrade...
==> Upgrading nijaru/tap/sy
  0.0.55 -> 0.0.56
🍺  /opt/homebrew/Cellar/sy/0.0.56: 5 files, 13.3MB, built in 1 second
```

**What actually happened:**
- ✅ fmt, openjph, vercel-cli: Upgraded via bru (fast)
- ⚠️ sy: Delegated to brew despite having prebuilt binary (unnecessary fallback)
- The "built in 1 second" message is misleading - it's extracting a prebuilt tarball, not compiling

## Technical Details

### The sy Formula

**Location:** `nijaru/homebrew-tap/Formula/sy.rb`

**Current structure:**
```ruby
class Sy < Formula
  desc "Modern rsync alternative - Fast, parallel file synchronization"
  homepage "https://github.com/nijaru/sy"
  version "0.0.56"
  license "MIT"
  head "https://github.com/nijaru/sy.git", branch: "main"

  on_macos do
    on_arm do
      url "https://github.com/nijaru/sy/releases/download/v0.0.56/sy-aarch64-apple-darwin.tar.gz"
      sha256 "6e293ef7dce59cba0121590a592eebc1b4c1fd72aa940f42db15204e4f91476c"
    end

    on_intel do
      # No prebuilt binary for Intel yet - build from source
      url "https://github.com/nijaru/sy/archive/refs/tags/v0.0.56.tar.gz"
      sha256 "0a316ca776e465a9bb95500c1fab9c45124a5bb5f7f503b39447cc4989b47c30"
      depends_on "rust" => :build
    end
  end

  def install
    if File.exist?("sy") && File.exist?("sy-remote")
      # Prebuilt binaries (Apple Silicon)
      bin.install "sy"
      bin.install "sy-remote"
    else
      # Build from source (Intel Mac)
      system "cargo", "install", *std_cargo_args
      bin.install "target/release/sy-remote" if File.exist?("target/release/sy-remote")
    end
  end

  test do
    system bin/"sy", "--version"
    assert_match "Modern rsync alternative", shell_output("#{bin}/sy --help")

    (testpath/"source").mkpath
    (testpath/"source/test.txt").write("test content")
    system bin/"sy", testpath/"source", testpath/"dest"
    assert_predicate testpath/"dest/test.txt", :exist?
    assert_equal "test content", (testpath/"dest/test.txt").read
  end
end
```

**Key characteristics:**
- ✅ Has prebuilt binary for Apple Silicon (ARM)
- ✅ Fast install (1 second to extract tarball)
- ✅ 13.3MB total (both binaries)
- ⚠️ Uses platform-specific `on_macos/on_arm` blocks instead of bottle DSL
- ⚠️ Not using Homebrew's official "bottle" format

### Why Bru Misses This

Bru likely checks for bottles using Homebrew's bottle DSL:

```ruby
# What Homebrew bottles look like
bottle do
  sha256 cellar: :any_skip_relocation, arm64_sonoma: "abc123..."
  sha256 cellar: :any_skip_relocation, arm64_ventura: "def456..."
  sha256 cellar: :any_skip_relocation, arm64_monterey: "789xyz..."
end
```

But `sy` uses direct `url` blocks in platform-specific sections, which:
- Works perfectly with Homebrew
- Provides prebuilt binaries
- Is NOT detected as a "bottle" by bru's heuristics

## Verification

```bash
❯ which sy && file $(which sy) && ls -lh $(which sy)
/opt/homebrew/bin/sy
/opt/homebrew/bin/sy: Mach-O 64-bit executable arm64
lrwxr-xr-x  1 nick  admin  26B Oct 31 17:38 /opt/homebrew/bin/sy -> ../Cellar/sy/0.0.56/bin/sy

❯ sy --version
sy 0.0.56
```

**Confirmed:** It's a prebuilt ARM binary, not built from source.

## Performance Impact

| Package | Handler | Install Time | Type |
|---------|---------|--------------|------|
| fmt | bru | ~1s | Official bottle |
| sy | brew (fallback) | ~1s | Custom tap prebuilt binary |
| sy (if built from source) | brew | ~30-60s | Rust compilation |

**Current state:** No real performance difference (both ~1s)
**Potential improvement:** Bru could handle custom tap prebuilt binaries directly for consistency

## Possible Solutions

### Option 1: Enhance Bru's Formula Parsing

**Goal:** Detect prebuilt binaries in custom tap formulas

**Implementation ideas:**
1. Parse `on_macos/on_arm/on_intel` blocks
2. Check if `url` points to prebuilt tarball vs source archive
3. Heuristics:
   - URL contains platform/arch in name (e.g., `aarch64-apple-darwin`)
   - `install` method checks for existing binaries (`File.exist?("sy")`)
   - No `depends_on "rust" => :build` for current platform
   - URL points to `/releases/download/` (GitHub releases) not `/archive/`

**Example detection logic:**
```ruby
def has_prebuilt_binary?(formula)
  # Check for platform-specific prebuilt URLs
  if formula.on_macos && formula.on_arm
    url = formula.on_macos.on_arm.url
    return true if url.include?("aarch64-apple-darwin") ||
                   url.include?("arm64") ||
                   url.include?("/releases/download/")
  end

  # Check install method for binary installation
  install_code = formula.install.source
  return true if install_code.include?("bin.install") &&
                 install_code.include?("File.exist?")

  false
end
```

### Option 2: Use Homebrew Bottle Format for sy

**Goal:** Convert sy to official bottle format

**Changes to sy formula:**
```ruby
class Sy < Formula
  desc "Modern rsync alternative - Fast, parallel file synchronization"
  homepage "https://github.com/nijaru/sy"
  url "https://github.com/nijaru/sy/archive/refs/tags/v0.0.56.tar.gz"
  sha256 "0a316ca776e465a9bb95500c1fab9c45124a5bb5f7f503b39447cc4989b47c30"
  license "MIT"

  bottle do
    root_url "https://github.com/nijaru/sy/releases/download/v0.0.56"
    sha256 cellar: :any_skip_relocation, arm64_sonoma: "6e293ef7dce59cba0121590a592eebc1b4c1fd72aa940f42db15204e4f91476c"
    # Add more platforms as needed
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    bin.install "target/release/sy-remote"
  end

  test do
    # ... same tests
  end
end
```

**Pros:**
- Standard Homebrew format
- Bru would automatically detect it
- Better compatibility with Homebrew ecosystem

**Cons:**
- Requires rebuilding bottles with Homebrew's specific format
- More complex release process
- `root_url` for custom taps is less common

### Option 3: Document as Expected Behavior

**Goal:** Clarify that custom taps with non-standard prebuilt binaries are expected to fall back to brew

**Changes:**
- Update bru documentation
- Maybe add a message: "Custom tap with prebuilt binary (handled by brew)"
- Keep current behavior

## Recommendation

**For bru:** Option 1 (Enhanced formula parsing)

**Why:**
1. Many custom taps use this pattern (platform-specific `url` blocks)
2. Provides better UX consistency
3. Maintains bru's speed advantage for all prebuilt binaries
4. Low risk - still falls back to brew if detection is uncertain

**Implementation priority:**
1. Parse `on_macos/on_arm` blocks with platform-specific URLs
2. Detect GitHub releases URLs (`/releases/download/` vs `/archive/`)
3. Check for binary installation patterns in `install` method
4. Fall back to brew for any uncertainty

## Additional Examples

Other projects that might use similar patterns:
- Custom CLI tools with cross-compilation
- Projects that release prebuilt binaries but aren't in Homebrew core
- Third-party taps with selective platform support

## Test Case for Bru

```ruby
# Test formula structure
on_macos do
  on_arm do
    url "https://github.com/owner/repo/releases/download/v1.0.0/tool-aarch64-apple-darwin.tar.gz"
    sha256 "abc123..."
  end
  on_intel do
    url "https://github.com/owner/repo/archive/refs/tags/v1.0.0.tar.gz"
    sha256 "def456..."
    depends_on "rust" => :build
  end
end

def install
  if File.exist?("tool")
    bin.install "tool"
  else
    system "cargo", "install", *std_cargo_args
  end
end
```

**Expected behavior:**
- On ARM Mac: Bru should handle directly (prebuilt binary)
- On Intel Mac: Fall back to brew (build from source)

## Contact

For questions about this specific case:
- **Project:** sy (modern rsync alternative)
- **Tap:** nijaru/homebrew-tap
- **Repository:** https://github.com/nijaru/sy
- **Issue:** Custom tap prebuilt binaries not detected by bru

---

**Note:** This document is meant to help improve bru's detection of prebuilt binaries in custom taps. The current fallback behavior is safe and correct, just not optimal for user experience consistency.
