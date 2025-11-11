# Kombrucha Library Examples

Comprehensive examples demonstrating how to use Kombrucha as a Rust library. These examples show real-world workflows for package management operations.

## Quick Start

Each example can be run with:

```bash
cargo run --example <example_name> [arguments]
```

## Examples

### 1. `query_formula` - Fetch Package Metadata

**Purpose**: Demonstrates querying the Homebrew API for formula metadata.

**What it shows**:
- Fetching formula information by name
- Reading versions (stable, head)
- Exploring dependencies (runtime and build)
- Checking bottle availability
- Accessing keg-only information

**Usage**:
```bash
cargo run --example query_formula                # Defaults to "ripgrep"
cargo run --example query_formula python          # Query specific formula
cargo run --example query_formula curl
```

**Output**:
```
Querying formula: ripgrep

Name:        ripgrep
Full name:   ripgrep
Description: Search tool like grep and The Silver Searcher
Homepage:    https://github.com/BurntSushi/ripgrep

Versions:
  Stable: 13.0.0
  Bottle available: true

Bottle rebuild: 0
Available platforms:
  - arm64_sequoia
  - x86_64_ventura
```

---

### 2. `list_installed` - List Installed Packages

**Purpose**: Inspect the local Cellar and show all installed packages.

**What it shows**:
- Reading the Cellar directory structure
- Listing all installed packages with versions
- Sorting versions semantically (newest first)
- Showing installation metadata (installed on request, dependencies)
- Multi-version package display

**Usage**:
```bash
cargo run --example list_installed
```

**Output**:
```
Homebrew Cellar Location: /opt/homebrew/Cellar

Installed packages (42):

curl (2)
  → 8.3.0 (installed on request) [deps: 1]
    8.2.1

python (1)
  → 3.13.0 (installed on request) [deps: 3]

ripgrep
  → 13.0.0 (installed on request)
```

---

### 3. `search_packages` - Search Homebrew

**Purpose**: Search across all formulae and casks using flexible matching.

**What it shows**:
- Full-text search across formula names and descriptions
- Searching cask tokens and names
- Case-insensitive matching
- Separate results for formulae vs casks
- Result summaries and counts

**Usage**:
```bash
cargo run --example search_packages                  # Interactive prompt
cargo run --example search_packages python           # Search for "python"
cargo run --example search_packages "cli tool"       # Multi-word queries
```

**Output**:
```
Searching for: 'python'
Fetching from API (this may take a moment on first run)...

Found 47 matches:

FORMULAE (43):
  python (3.13.0) - Interpreted, interactive, object-oriented programming language
  python-tk (3.13.0) - Tkinter wrapper for Python
  python-yq (3.4.1) - Command-line YAML processor (jq wrapper)
  ...

CASKS (4):
  pycharm-pro (2023.3) - JetBrains PyCharm Professional IDE
  python (3.11.6)
  ...

Summary: 43 formulae + 4 casks = 47 matches
```

**Performance Note**: First search may take 2-3 seconds (downloads full formula index). Subsequent searches are cached for 24 hours.

---

### 4. `dependency_tree` - Explore Dependencies

**Purpose**: Visualize a package's dependency graph and analyze relationships.

**What it shows**:
- Runtime vs build dependency distinction
- Dependency tree visualization
- Shared dependencies (appear in both categories)
- Dependency analysis and breakdown
- Keg-only information
- Bottle availability by platform

**Usage**:
```bash
cargo run --example dependency_tree                  # Defaults to "python"
cargo run --example dependency_tree curl             # Query specific formula
cargo run --example dependency_tree gcc
```

**Output**:
```
Dependency tree for: python

Package: python
Description: Interpreted, interactive, object-oriented programming language
Version: 3.13.0

RUNTIME DEPENDENCIES (8):
├── bzip2
├── libffi
├── mpdecimal
├── openssl@3
├── sqlite
├── xz
├── zlib
└── tcl-tk

BUILD DEPENDENCIES (3):
├── pkg-config
├── sphinx-doc
└── openssl@3

--- Dependency Analysis ---
Total unique dependencies: 10
Total dependency references: 12

Dependency breakdown:
  Runtime only: 7
  Build only: 2
  Both: 1

Bottle availability: Yes
  Rebuild: 0
  Platforms available: 4
    - arm64_sequoia
    - arm64_sonoma
    - x86_64_sonoma
    - x86_64_ventura
```

---

### 5. `bottle_installation` - Download and Install a Package

**Purpose**: Complete bottle-based installation workflow from download to activation.

**What it shows**:
- Fetching formula metadata
- Downloading bottles with checksum verification
- Extracting to the Cellar
- Generating installation receipts
- Creating symlinks for system accessibility
- Installation verification

**Usage**:
```bash
cargo run --example bottle_installation                # Defaults to "curl"
cargo run --example bottle_installation ripgrep        # Install specific package
```

**Output**:
```
Installing ripgrep from bottle...

Step 1: Fetching formula metadata...
✓ Found ripgrep
  Version: 13.0.0
  Dependencies: 0

Step 2: Downloading bottle...
✓ Downloaded to: /Users/nick/.cache/bru/downloads/ripgrep--13.0.0.arm64_sequoia.bottle.tar.gz
  Size: 5.2 MB

Step 3: Extracting to Cellar...
✓ Extracted to: /opt/homebrew/Cellar/ripgrep/13.0.0

Step 4: Creating installation receipt...
✓ Receipt created
  Homebrew version: bru/0.1.34

Step 5: Creating symlinks...
✓ Created 2 symlinks
    /opt/homebrew/bin/ripgrep
    /opt/homebrew/share/man/man1/ripgrep.1

✓ Installation complete!

Installed package details:
  Name:    ripgrep
  Version: 13.0.0
  Path:    /opt/homebrew/Cellar/ripgrep/13.0.0
  Prefix:  /opt/homebrew

Verifying installation...
✓ Verified in Cellar: 1 version(s) found
```

**Warning**: This example modifies your actual system Cellar. Use with caution or test in a container.

---

### 6. `check_upgrades` - Find Outdated Packages

**Purpose**: Check all installed packages against the API to find upgradeable versions.

**What it shows**:
- Listing all installed packages
- Comparing against API versions
- Semantic version comparison
- Categorizing results (upgradeable, up-to-date, not found)
- Handling tap packages (not in core API)
- Error handling for unavailable packages

**Usage**:
```bash
cargo run --example check_upgrades
```

**Output**:
```
Checking for package upgrades...

Step 1: Reading installed packages from Cellar...
✓ Found 42 package installations

Step 2: Checking 38 formulae for updates...
.......................................

═══════════════════════════════════════
UPGRADE REPORT
═══════════════════════════════════════

⬆️  UPGRADEABLE (3):
  curl 8.2.1 → 8.3.0
  python 3.12.0 → 3.13.0
  ripgrep 12.1.1 → 13.0.0

✓ UP TO DATE (31):
  bash 5.2.21
  coreutils 9.3
  ... and 29 more

⚠️  NOT FOUND IN API (4):
  my-custom-package
  ... and 3 more (possibly from taps)

═══════════════════════════════════════
Summary:
  Total installed: 38
  Upgradeable: 3
  Up to date: 31
  Not found: 4
  Errors: 0

💡 Tip: Use 'bru upgrade <formula>' to update specific packages
    or 'bru upgrade' to update all packages
```

---

## Common Workflows

### Workflow 1: Install a Package

```rust
use kombrucha::{BrewApi, download, extract, symlink};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api = BrewApi::new()?;
    let formula = api.fetch_formula("ripgrep").await?;
    
    let client = reqwest::Client::new();
    let bottle = download::download_bottle(&formula, None, &client).await?;
    let cellar = extract::extract_bottle(&bottle, "ripgrep", "13.0.0")?;
    symlink::link_formula("ripgrep", "13.0.0")?;
    symlink::optlink("ripgrep", "13.0.0")?;
    
    Ok(())
}
```

### Workflow 2: Check for Updates

```rust
use kombrucha::{cellar, BrewApi};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api = BrewApi::new()?;
    let installed = cellar::list_installed()?;
    
    for pkg in installed {
        let formula = api.fetch_formula(&pkg.name).await?;
        if let Some(latest) = &formula.versions.stable {
            if latest > &pkg.version {
                println!("Upgrade available: {} {} → {}", 
                    pkg.name, pkg.version, latest);
            }
        }
    }
    
    Ok(())
}
```

### Workflow 3: Search and Install

```rust
use kombrucha::BrewApi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api = BrewApi::new()?;
    let results = api.search("cli").await?;
    
    for formula in &results.formulae {
        println!("{}: {}", formula.name, 
            formula.desc.as_ref().unwrap_or(&"No description".to_string()));
    }
    
    Ok(())
}
```

---

## API Reference Summary

### Core Modules

| Module | Purpose |
|--------|---------|
| `api` | Query Homebrew JSON API for formula/cask metadata |
| `cellar` | Inspect local installed packages |
| `download` | Parallel bottle downloads from GHCR |
| `extract` | Decompress and extract bottles to Cellar |
| `symlink` | Create/remove system-wide symlinks |
| `platform` | Detect OS/arch for correct bottle selection |
| `receipt` | Installation metadata management |
| `tap` | Custom tap repository management |
| `cache` | Persistent disk caching of API data |

### Key Types

- **`BrewApi`**: Client for querying the Homebrew API
- **`Formula`**: Package metadata (versions, dependencies, bottles)
- **`InstalledPackage`**: Package in the local Cellar
- **`InstallReceipt`**: Installation metadata file
- **`Bottle`**: Precompiled binary archive information

### Performance Tips

1. **API queries are cached**: In-memory (1000 formulae) and disk (24 hours)
2. **Search is expensive**: Fetches all ~10k formulae. Use `fetch_formula()` for direct lookups
3. **Parallel downloads**: Up to 8 concurrent bottle downloads
4. **Symlink creation**: Parallelized with rayon for fast linking

---

## Error Handling

All examples use `anyhow::Result<T>` for error handling:

```rust
// Specific error types:
use kombrucha::BruError;

match api.fetch_formula("nonexistent") {
    Ok(f) => println!("{}", f.name),
    Err(BruError::FormulaNotFound(name)) => eprintln!("Not found: {}", name),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Testing Examples

All examples avoid modifying the system Cellar by default. To safely test bottle installation:

```bash
# Create a test environment with testcontainers
# (Example implementation in tests/ directory)
```

---

## Contributing

When adding new examples:

1. Keep examples focused (single workflow)
2. Include comprehensive comments
3. Add error handling and helpful output
4. Test with `cargo build --examples`
5. Document in this README with usage and output
6. Use realistic, safe package names for demos

---

## See Also

- **Full Library Documentation**: `cargo doc --open`
- **CLI Tool**: See `src/cli/` for command implementation
- **Tests**: See `tests/` for integration test patterns
