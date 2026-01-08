# Zyra Installer

This directory contains installation scripts for the Zyra programming language.

## 🖥️ Windows GUI Installer (Recommended)

The easiest way to install Zyra on Windows.

### Build the Installer

1. Install [Inno Setup 6.x](https://jrsoftware.org/isdl.php)
2. Run `cargo build --release`
3. Open `ZyraSetup.iss` in Inno Setup Compiler
4. Press `Ctrl+F9` to build
5. Find `ZyraSetup-1.0.1.exe` in the `dist/` folder

### Features

- ✅ License agreement page
- ✅ Add to PATH option
- ✅ Desktop shortcut
- ✅ Start menu shortcuts
- ✅ Clean uninstaller

---

## Windows Command Line Installation

### Quick Install (Double-click)

1. Double-click `install.bat`
2. Follow the on-screen prompts

### PowerShell Install

```powershell
.\install.ps1           # User-level
.\install.ps1 -Uninstall # Uninstall
```

---

## Linux Installation

### Quick Install

```bash
chmod +x install.sh
./install.sh
```

### Options

```bash
sudo ./install.sh          # System-wide
./install.sh --prefix ~/   # Custom prefix
./install.sh --uninstall   # Uninstall
```

---

## Installation Locations

| Platform      | Mode  | Location                 |
| ------------- | ----- | ------------------------ |
| Windows (GUI) | Admin | `C:\Program Files\Zyra\` |
| Windows (CLI) | User  | `%LOCALAPPDATA%\Zyra\`   |
| Linux         | User  | `~/.local/bin`           |
| Linux         | Root  | `/usr/local/bin`         |

## Requirements

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
