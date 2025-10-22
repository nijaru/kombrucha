# Homebrew Compatibility Research

## Directory Structure

### Prefix Detection

**macOS**:
- Apple Silicon (M1/M2/M3): `/opt/homebrew`
- Intel: `/usr/local`

**Detection method**:
```rust
fn detect_homebrew_prefix() -> PathBuf {
    if let Ok(prefix) = std::env::var("HOMEBREW_PREFIX") {
        return PathBuf::from(prefix);
    }

    // Check architecture
    #[cfg(target_arch = "aarch64")]
    {
        PathBuf::from("/opt/homebrew")
    }
    #[cfg(target_arch = "x86_64")]
    {
        PathBuf::from("/usr/local")
    }
}
```

**Commands**:
```bash
$ brew --prefix
/opt/homebrew

$ brew --cellar
/opt/homebrew/Cellar
```

### Standard Paths

```
/opt/homebrew/                    # Prefix
├── bin/                          # Symlinks to executables
│   └── wget -> ../Cellar/wget/1.25.0/bin/wget
├── Cellar/                       # Actual installations
│   └── wget/
│       └── 1.25.0/               # Version-specific dir
│           ├── bin/              # Executables
│           ├── share/            # Docs, man pages
│           ├── INSTALL_RECEIPT.json
│           ├── sbom.spdx.json
│           ├── .bottle/          # Bottle metadata
│           └── .brew/            # Homebrew metadata
├── lib/                          # Symlinks to libraries
├── include/                      # Symlinks to headers
└── share/                        # Symlinks to shared data
```

## Install Receipt Format

**Location**: `/opt/homebrew/Cellar/<formula>/<version>/INSTALL_RECEIPT.json`

**Example** (wget 1.25.0):
```json
{
  "homebrew_version": "4.4.4-76-g40f4ab2",
  "used_options": [],
  "unused_options": [],
  "built_as_bottle": true,
  "poured_from_bottle": true,
  "loaded_from_api": true,
  "installed_as_dependency": false,
  "installed_on_request": true,
  "changed_files": [],
  "time": 1731462018,
  "source_modified_time": 1731274298,
  "compiler": "clang",
  "aliases": [],
  "runtime_dependencies": [
    {
      "full_name": "libunistring",
      "version": "1.3",
      "revision": 0,
      "pkg_version": "1.3",
      "declared_directly": true
    },
    {
      "full_name": "gettext",
      "version": "0.22.5",
      "revision": 0,
      "pkg_version": "0.22.5",
      "declared_directly": true
    }
  ],
  "source": {
    "path": "/opt/homebrew/Library/Taps/homebrew/homebrew-core/Formula/w/wget.rb",
    "tap": "homebrew/core",
    "tap_git_head": "abc123...",
    "spec": "stable",
    "versions": {
      "stable": "1.25.0",
      "version_scheme": 0
    }
  },
  "arch": "arm64",
  "built_on": {
    "os": "Macintosh",
    "os_version": "macOS 15.1",
    "cpu_family": "arm",
    "xcode": "16.1",
    "clt": "16.1.0.0.1.1729049160",
    "preferred_perl": "5.34"
  },
  "stdlib": "libc++"
}
```

**Key fields**:
- `poured_from_bottle`: true if from bottle, false if built from source
- `installed_on_request`: true if user asked for it, false if dependency
- `runtime_dependencies`: Full dependency tree with exact versions
- `time`: Unix timestamp of installation
- `source`: Where formula came from (tap, git head)
- `arch`: CPU architecture
- `built_on`: Build environment details

**For bru Phase 2**:
- Generate compatible receipts
- Must include all fields for `brew list` compatibility
- Use `homebrew_version` = "bru/<version>" to identify bru installs?

## Bottle Format

### Bottle Metadata (from API)

```json
{
  "bottle": {
    "stable": {
      "rebuild": 0,
      "root_url": "https://ghcr.io/v2/homebrew/core/wget",
      "files": {
        "arm64_sequoia": {
          "cellar": "/opt/homebrew/Cellar",
          "url": "https://ghcr.io/v2/homebrew/core/wget/blobs/sha256:a93dd95c...",
          "sha256": "a93dd95c5d63036e026b526e000d33fae7fb44d9a8fda5afc89bff112438c6b3"
        },
        "arm64_sonoma": {
          "cellar": "/opt/homebrew/Cellar",
          "url": "https://ghcr.io/v2/homebrew/core/wget/blobs/sha256:...",
          "sha256": "..."
        }
      }
    }
  }
}
```

**Platform detection**:
- macOS version: `sw_vers -productVersion` → "15.1" (Sequoia)
- Architecture: `uname -m` → "arm64"
- Map to bottle key: `arm64_sequoia`

**Bottle keys** (current):
- `arm64_sequoia` (macOS 15)
- `arm64_sonoma` (macOS 14)
- `arm64_ventura` (macOS 13)
- `arm64_monterey` (macOS 12)
- `arm64_big_sur` (macOS 11)
- `monterey` (Intel macOS 12)
- `big_sur` (Intel macOS 11)
- `x86_64_linux` (Linux Intel)
- `arm64_linux` (Linux ARM)

### Bottle File Format

**Bottle is a `.tar.gz` archive**:
```
wget--1.25.0.arm64_sequoia.bottle.tar.gz
  └── wget/
      └── 1.25.0/
          ├── bin/
          │   └── wget
          ├── share/
          │   └── ...
          ├── INSTALL_RECEIPT.json
          └── ...
```

**Note**: Archive contains `<formula>/<version>/` structure, not just files!

**Download and extraction**:
1. Fetch bottle URL from API
2. Download .tar.gz to cache
3. Verify SHA256 checksum
4. Extract to `/opt/homebrew/Cellar/`
5. Create symlinks in `/opt/homebrew/bin/`, etc.

## Symlink Management

### Creation

**Strategy**: Relative symlinks for portability

```bash
# Example: Link wget binary
cd /opt/homebrew/bin
ln -s ../Cellar/wget/1.25.0/bin/wget wget
```

**In Rust**:
```rust
use std::os::unix::fs::symlink;
use std::path::Path;

fn link_formula(cellar_path: &Path, prefix: &Path) -> Result<()> {
    let bin_dir = cellar_path.join("bin");

    for entry in std::fs::read_dir(bin_dir)? {
        let entry = entry?;
        let name = entry.file_name();

        // Create relative symlink
        let target = format!("../Cellar/{}/{}/bin/{}",
            formula_name, version, name.to_str().unwrap());
        let link = prefix.join("bin").join(&name);

        symlink(&target, &link)?;
    }

    Ok(())
}
```

### Conflict Handling

If symlink already exists:
- Check if it points to same formula (different version)
- If same formula: Ask user to upgrade or force overwrite
- If different formula: Error (conflict)

**Example**: Both `node` and `nodejs` might provide `node` binary

## Cache Management

**Homebrew cache**: `~/Library/Caches/Homebrew/`

**Contents**:
```
~/Library/Caches/Homebrew/
├── downloads/                    # Downloaded bottles/sources
│   └── <sha256>--wget--1.25.0.arm64_sequoia.bottle.tar.gz
├── api/                          # API response cache
│   ├── formula.jws.json
│   └── cask.jws.json
└── Cask/                         # Cask installers
```

**For bru**:
```
~/.cache/bru/                     # XDG-compliant
├── downloads/
│   └── wget--1.25.0.arm64_sequoia.bottle.tar.gz
├── api/
│   ├── formulae.json             # All formulae (cached 1h)
│   ├── casks.json                # All casks (cached 1h)
│   └── formula/
│       └── wget.json             # Individual formula (cached 1d)
└── tmp/                          # Temporary files during install
```

**Cache TTL**:
- All formulae/casks: 1 hour (changes infrequently)
- Individual formula: 1 day (stable)
- Bottles: Forever (immutable, identified by SHA256)

## Checksums & Security

### SHA256 Verification

**Process**:
1. Download bottle to cache
2. Compute SHA256 of downloaded file
3. Compare with `sha256` from API
4. If mismatch: Delete file, error out

**Implementation**:
```rust
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{self, Read};

fn verify_checksum(path: &Path, expected: &str) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = format!("{:x}", hasher.finalize());
    Ok(result == expected)
}
```

**Homebrew uses SHA256** (not SHA512, not MD5)

### Signature Verification

**Current Homebrew**: Does NOT use GPG signatures for bottles

**Why**: Bottles served from GitHub Container Registry (GHCR) with HTTPS
- HTTPS provides transport security
- SHA256 checksum provides integrity
- GHCR is trusted infrastructure

**For bru**: Same approach
- Verify SHA256 checksums
- Use HTTPS for all downloads
- Trust GitHub infrastructure

## Keg-Only Formulae

### What are keg-only formulae?

Formulae installed to Cellar but NOT symlinked to prefix

**Examples**:
- `openssl@3` (conflicts with system OpenSSL)
- `python@3.12` (specific version, not default)
- `icu4c` (system provides own version)

### Detection

**In API response**:
```json
{
  "name": "openssl@3",
  "keg_only": true,
  "keg_only_reason": {
    "reason": "shadowed by macOS",
    "explanation": "macOS provides LibreSSL"
  }
}
```

### Handling

**Installation**:
1. Extract to Cellar as normal
2. DO NOT create symlinks in prefix
3. Mark as keg-only in receipt

**Dependencies**:
- Other formulae can still depend on keg-only formulae
- Build tools (compilers, linkers) find them via:
  - `PKG_CONFIG_PATH` env var
  - Explicit paths in build scripts
  - Homebrew provides `--prefix openssl@3` to get path

**For bru Phase 2**:
- Respect `keg_only` flag from API
- Skip symlink creation for keg-only
- Provide `bru --prefix <formula>` to get cellar path

## Compatibility Strategy

### Phase 1 (Read-only)
- [x] Auto-detect Homebrew prefix
- [x] Read formula info from API
- [x] Display compatible output

### Phase 2 (Installation)
- [ ] Install to same Cellar as Homebrew
- [ ] Generate compatible INSTALL_RECEIPT.json
- [ ] Create symlinks matching Homebrew's style
- [ ] Verify bottles with SHA256
- [ ] Handle keg-only formulae

### Phase 3 (Full Compatibility)
- [ ] Support custom prefix
- [ ] Handle edge cases (conflicts, upgrades)
- [ ] Generate SBOM (sbom.spdx.json)
- [ ] Support taps

## Testing Compatibility

**Validation checklist**:
```bash
# Install with bru
bru install wget

# Verify brew can see it
brew list | grep wget          # Should appear
brew info wget                 # Should show as installed

# Verify files are correct
diff -r /opt/homebrew/Cellar/wget/1.25.0/ \
        /tmp/homebrew-installed-wget/

# Verify symlinks work
which wget                     # Should be /opt/homebrew/bin/wget
wget --version                 # Should work

# Verify brew can uninstall
brew uninstall wget            # Should work
```

**Integration test**:
```rust
#[test]
fn test_bru_installed_formula_visible_to_brew() {
    // Install with bru
    Command::new("./target/release/bru")
        .args(&["install", "tree"])
        .output()
        .expect("bru install failed");

    // Check with brew
    let output = Command::new("brew")
        .args(&["list", "tree"])
        .output()
        .expect("brew list failed");

    assert!(output.status.success());
}
```

## References

- Homebrew source: https://github.com/Homebrew/brew
- Formula API: https://formulae.brew.sh/api/
- Bottle format: https://docs.brew.sh/Bottles
- Installation: https://docs.brew.sh/Installation

---

**Status**: Researched, documented for Phase 2 implementation
**Next**: Implement prefix detection, update API types with bottle/keg info
