#!/bin/bash
#
# Zyra Programming Language Installer
# Linux Installation Script
#
# This script installs Zyra to your system by:
# 1. Building the release binary
# 2. Creating installation directory
# 3. Adding Zyra to PATH
#
# Run with sudo for system-wide installation,
# or run normally for user-level installation.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

header() { echo -e "\n${CYAN}=== $1 ===${NC}"; }
step() { echo -e "  ${GREEN}→${NC} $1"; }
warn() { echo -e "  ${YELLOW}⚠${NC} $1"; }
err() { echo -e "  ${RED}✗${NC} $1"; }

# Check if running as root
if [ "$EUID" -eq 0 ]; then
    IS_ROOT=true
    INSTALL_DIR="/usr/local"
else
    IS_ROOT=false
    INSTALL_DIR="$HOME/.local"
fi

BIN_DIR="$INSTALL_DIR/bin"
EXE_PATH="$BIN_DIR/zyra"

# Parse arguments
UNINSTALL=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --uninstall|-u)
            UNINSTALL=true
            shift
            ;;
        --prefix)
            INSTALL_DIR="$2"
            BIN_DIR="$INSTALL_DIR/bin"
            EXE_PATH="$BIN_DIR/zyra"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--uninstall] [--prefix DIR]"
            exit 1
            ;;
    esac
done

# Uninstall
if [ "$UNINSTALL" = true ]; then
    header "Uninstalling Zyra"
    
    if [ -f "$EXE_PATH" ]; then
        step "Removing $EXE_PATH..."
        rm -f "$EXE_PATH"
    else
        warn "Zyra not found at $EXE_PATH"
    fi
    
    echo -e "\n${GREEN}✓ Zyra has been uninstalled.${NC}"
    exit 0
fi

# Install
header "Zyra Programming Language Installer"
echo "  Version: 1.0.0"
echo "  Install Dir: $INSTALL_DIR"
echo "  Mode: $([ "$IS_ROOT" = true ] && echo 'System-wide' || echo 'User-level')"

# Check for Rust/Cargo
header "Checking Prerequisites"
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    step "Cargo found: $CARGO_VERSION"
else
    err "Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Find project root (directory containing Cargo.toml)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    err "Could not find Cargo.toml. Please run this script from the installer directory."
    exit 1
fi

# Build release binary
header "Building Zyra"
step "Building release binary..."
cd "$PROJECT_ROOT"
cargo build --release

if [ $? -ne 0 ]; then
    err "Build failed"
    exit 1
fi
step "Build successful!"

# Create install directory
header "Installing"
if [ ! -d "$BIN_DIR" ]; then
    step "Creating $BIN_DIR..."
    mkdir -p "$BIN_DIR"
fi

# Copy binary
step "Copying zyra to $EXE_PATH..."
cp "$PROJECT_ROOT/target/release/zyra" "$EXE_PATH"
chmod +x "$EXE_PATH"

# Check PATH
header "Configuring PATH"
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    step "Adding $BIN_DIR to PATH..."
    
    # Determine shell config file
    SHELL_CONFIG=""
    if [ -f "$HOME/.bashrc" ]; then
        SHELL_CONFIG="$HOME/.bashrc"
    elif [ -f "$HOME/.zshrc" ]; then
        SHELL_CONFIG="$HOME/.zshrc"
    elif [ -f "$HOME/.profile" ]; then
        SHELL_CONFIG="$HOME/.profile"
    fi
    
    if [ -n "$SHELL_CONFIG" ]; then
        # Check if already added
        if ! grep -q "export PATH.*$BIN_DIR" "$SHELL_CONFIG" 2>/dev/null; then
            echo "" >> "$SHELL_CONFIG"
            echo "# Zyra Programming Language" >> "$SHELL_CONFIG"
            echo "export PATH=\"\$PATH:$BIN_DIR\"" >> "$SHELL_CONFIG"
            step "Added to $SHELL_CONFIG"
        else
            step "Already configured in $SHELL_CONFIG"
        fi
    else
        warn "Could not find shell config file. Please add $BIN_DIR to your PATH manually."
    fi
else
    step "Already in PATH"
fi

# Verify
header "Verification"
export PATH="$PATH:$BIN_DIR"
if [ -x "$EXE_PATH" ]; then
    VERSION=$("$EXE_PATH" --version 2>/dev/null || echo "unknown")
    step "Installed: $VERSION"
else
    warn "Could not verify installation"
fi

echo ""
echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  ${GREEN}✓ Zyra has been installed!${CYAN}            ║${NC}"
echo -e "${CYAN}╠════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║  Restart your terminal, then run:      ║${NC}"
echo -e "${CYAN}║    ${NC}zyra --version${CYAN}                      ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
