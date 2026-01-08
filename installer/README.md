# Zyra Installer

This directory contains installation scripts for the Zyra programming language.

## Windows Installation

### Quick Install (Double-click)

1. Double-click `install.bat`
2. Follow the on-screen prompts

### Command Line Install

```powershell
# User-level installation
.\install.ps1

# System-wide installation (run as Administrator)
.\install.ps1
```

### Uninstall

```powershell
.\install.ps1 -Uninstall
```

## Linux Installation

### Quick Install

```bash
chmod +x install.sh
./install.sh
```

### System-wide Install (requires sudo)

```bash
sudo ./install.sh
```

### Custom Prefix

```bash
./install.sh --prefix ~/.local
```

### Uninstall

```bash
./install.sh --uninstall
```

## What Gets Installed

- `zyra` - The Zyra compiler and runtime
- PATH environment variable update

## Installation Locations

### Windows

| Mode        | Location                    |
| ----------- | --------------------------- |
| User-level  | `%LOCALAPPDATA%\Zyra\bin`   |
| System-wide | `C:\Program Files\Zyra\bin` |

### Linux

| Mode        | Location         |
| ----------- | ---------------- |
| User-level  | `~/.local/bin`   |
| System-wide | `/usr/local/bin` |

## Requirements

- [Rust](https://rustup.rs/) (for building from source)

## After Installation

Restart your terminal, then verify:

```
zyra --version
```

## Quick Start

```bash
# Create a new project
zyra init my_game

# Run your project
cd my_game
zyra run
```
