#!/bin/sh
# Install script for bru (kombrucha)
# Usage: curl -fsSL https://raw.githubusercontent.com/nijaru/kombrucha/main/install.sh | sh

set -e

# Detect platform
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64|aarch64)
                PLATFORM="macos-arm64"
                ;;
            x86_64)
                PLATFORM="macos-x86_64"
                ;;
            *)
                echo "Error: Unsupported macOS architecture: $ARCH"
                exit 1
                ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64)
                PLATFORM="linux-x86_64"
                ;;
            *)
                echo "Error: Unsupported Linux architecture: $ARCH"
                echo "Supported: x86_64"
                echo "For other architectures, install via: cargo install kombrucha"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Error: Unsupported operating system: $OS"
        echo "Supported: macOS, Linux"
        echo "For other platforms, install via: cargo install kombrucha"
        exit 1
        ;;
esac

# Get latest release version
echo "Fetching latest release..."
LATEST_VERSION=$(curl -fsSL https://api.github.com/repos/nijaru/kombrucha/releases/latest | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')

if [ -z "$LATEST_VERSION" ]; then
    echo "Error: Could not fetch latest version"
    exit 1
fi

echo "Latest version: $LATEST_VERSION"

# Download binary
ASSET_NAME="bru-${PLATFORM}"
DOWNLOAD_URL="https://github.com/nijaru/kombrucha/releases/download/v${LATEST_VERSION}/${ASSET_NAME}.tar.gz"

echo "Downloading from: $DOWNLOAD_URL"
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

if ! curl -fsSL "$DOWNLOAD_URL" -o "${ASSET_NAME}.tar.gz"; then
    echo "Error: Failed to download release"
    echo "URL: $DOWNLOAD_URL"
    exit 1
fi

# Extract binary
tar xzf "${ASSET_NAME}.tar.gz"

# Determine install location
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
else
    mkdir -p "$HOME/.local/bin"
    INSTALL_DIR="$HOME/.local/bin"
fi

# Install binary
echo "Installing to: $INSTALL_DIR/bru"
mv bru "$INSTALL_DIR/bru"
chmod +x "$INSTALL_DIR/bru"

# Cleanup
cd - > /dev/null
rm -rf "$TMP_DIR"

# Check if in PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo ""
    echo "⚠️  Warning: $INSTALL_DIR is not in your PATH"
    echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
fi

echo "✅ Successfully installed bru $LATEST_VERSION"
echo ""
echo "Try it out:"
echo "    bru --version"
echo "    bru search rust"
echo ""
echo "For more info: https://github.com/nijaru/kombrucha"
