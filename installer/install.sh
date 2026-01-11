#!/bin/bash
#
# Zyra Programming Language Installer
# Linux Installation Script
#
# This script installs Zyra by:
# 1. Downloading prebuilt binary from GitHub repository (installer/bin/linux/zyra)
# 2. If download fails, cloning the repo and building from source
# 3. Setting up PATH
#
# Run with sudo for system-wide installation,
# or run normally for user-level installation.

set -e

# Version
VERSION="1.0.2"
GITHUB_REPO="cowoksoftspoken/Zyra"
GITHUB_RAW="https://raw.githubusercontent.com/${GITHUB_REPO}/main"

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
    SHARE_DIR="/usr/local/share/zyra"
else
    IS_ROOT=false
    INSTALL_DIR="$HOME/.local"
    SHARE_DIR="$HOME/.local/share/zyra"
fi

BIN_DIR="$INSTALL_DIR/bin"
EXE_PATH="$BIN_DIR/zyra"
ICONS_DIR="$SHARE_DIR/icons"

# Parse arguments
UNINSTALL=false
FORCE_BUILD=false
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
        --build|-b)
            FORCE_BUILD=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--uninstall] [--prefix DIR] [--build]"
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
    
    if [ -d "$SHARE_DIR" ]; then
        step "Removing $SHARE_DIR..."
        rm -rf "$SHARE_DIR"
    fi
    
    echo -e "\n${GREEN}✓ Zyra has been uninstalled.${NC}"
    exit 0
fi

# Install
header "Zyra Programming Language Installer"
echo "  Version: $VERSION"
echo "  Install Dir: $INSTALL_DIR"
echo "  Mode: $([ "$IS_ROOT" = true ] && echo 'System-wide' || echo 'User-level')"

# Create directories
header "Creating Directories"
mkdir -p "$BIN_DIR"
mkdir -p "$ICONS_DIR"
step "Created $BIN_DIR"
step "Created $ICONS_DIR"

# Try to get binary
BINARY_OBTAINED=false

if [ "$FORCE_BUILD" = false ]; then
    header "Downloading Prebuilt Binary"
    
    # Download from GitHub raw (installer/bin/linux/zyra)
    DOWNLOAD_URL="${GITHUB_RAW}/installer/bin/linux/zyra"
    step "Downloading from: $DOWNLOAD_URL"
    
    if curl -fsSL "$DOWNLOAD_URL" -o "$EXE_PATH.tmp" 2>/dev/null; then
        # Verify it's an ELF binary (not HTML error page)
        if file "$EXE_PATH.tmp" | grep -q "ELF"; then
            mv "$EXE_PATH.tmp" "$EXE_PATH"
            chmod +x "$EXE_PATH"
            step "Downloaded prebuilt binary successfully!"
            BINARY_OBTAINED=true
        else
            rm -f "$EXE_PATH.tmp"
            warn "Downloaded file is not a valid ELF binary"
        fi
    else
        rm -f "$EXE_PATH.tmp"
        warn "Could not download prebuilt binary"
    fi
fi

# If no binary obtained, clone and build
if [ "$BINARY_OBTAINED" = false ]; then
    header "Building from Source"
    
    # Check for Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        err "Cargo not found. Please install Rust from https://rustup.rs/"
        exit 1
    fi
    
    CARGO_VERSION=$(cargo --version)
    step "Cargo found: $CARGO_VERSION"
    
    # Check for git
    if ! command -v git &> /dev/null; then
        err "Git not found. Please install git."
        exit 1
    fi
    
    # Clone repository
    TEMP_DIR=$(mktemp -d)
    step "Cloning Zyra repository..."
    
    if git clone --depth 1 "https://github.com/${GITHUB_REPO}.git" "$TEMP_DIR/Zyra" 2>/dev/null; then
        cd "$TEMP_DIR/Zyra"
        
        step "Building release binary (this may take a few minutes)..."
        if cargo build --release; then
            cp "target/release/zyra" "$EXE_PATH"
            chmod +x "$EXE_PATH"
            step "Build successful!"
            BINARY_OBTAINED=true
            
            # Also copy icons
            if [ -d "extensions/ZyraFileIcons/icons" ]; then
                cp extensions/ZyraFileIcons/icons/*.png "$ICONS_DIR/" 2>/dev/null || true
                cp extensions/ZyraFileIcons/icons/*.ico "$ICONS_DIR/" 2>/dev/null || true
                step "Copied icons to $ICONS_DIR"
            fi
        else
            err "Build failed"
        fi
        
        cd - > /dev/null
        rm -rf "$TEMP_DIR"
    else
        err "Failed to clone repository"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
fi

if [ "$BINARY_OBTAINED" = false ]; then
    err "Could not obtain Zyra binary"
    exit 1
fi

# Download icons if not already present
header "Setting up Icons"
if [ ! -f "$ICONS_DIR/zyra.png" ] && [ ! -f "$ICONS_DIR/zyra.ico" ]; then
    step "Downloading icons..."
    curl -fsSL "${GITHUB_RAW}/extensions/ZyraFileIcons/icons/zyra.png" -o "$ICONS_DIR/zyra.png" 2>/dev/null || true
    curl -fsSL "${GITHUB_RAW}/extensions/ZyraFileIcons/icons/zyra.ico" -o "$ICONS_DIR/zyra.ico" 2>/dev/null || true
    
    if [ -f "$ICONS_DIR/zyra.png" ] || [ -f "$ICONS_DIR/zyra.ico" ]; then
        step "Icons downloaded to $ICONS_DIR"
    else
        warn "Could not download icons (non-critical)"
    fi
else
    step "Icons already present"
fi

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
    VERSION_OUTPUT=$("$EXE_PATH" --version 2>/dev/null || echo "unknown")
    step "Installed: $VERSION_OUTPUT"
else
    warn " Could not verify installation"
fi

echo ""
echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  ${GREEN}✓ Zyra has been installed!${CYAN}            ║${NC}"
echo -e "${CYAN}╠════════════════════════════════════════╣${NC}"
echo -e "${CYAN}║  Restart your terminal, then run:      ║${NC}"
echo -e "${CYAN}║    ${NC}zyra --version${CYAN}                      ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
