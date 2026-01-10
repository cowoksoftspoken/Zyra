# Zyra v1.0.2 Release Notes

## 🎉 What's New

### Result/Option Types & Error Handling

- **New**: `Ok(value)`, `Error(value)` for Result type
- **New**: `Some(value)`, `None` for Option type
- **New**: `unwrap()`, `unwrap_or(default)` methods
- **New**: `is_ok()`, `is_err()`, `is_some()`, `is_none()` checks
- **Fixed**: `parse_int()` and `parse_float()` now return proper Option types

### Scope-Based Variable Naming

- **Fixed**: Variables with same name in different `if/else` blocks now work correctly
- Each branch gets unique scope ID for proper isolation

### Reference Auto-Deref

- **New**: `&String` can now be passed where `String` is expected (automatic dereferencing)
- Enables borrowing in string functions: `string::contains(&op, "+")`

### LinkedList Error Handling

- **Fixed**: `list_get()` and `list_set()` now return `Err()` for:
  - Invalid list ID
  - Index out of bounds

### Cross-Platform Compatibility

- **Fixed**: Window icon API now Windows-only (fixes Wayland build errors)
- **Fixed**: minifb dependency compatible with Linux Wayland (Hyprland, Sway, etc.)

---

## 📦 Installation

### Windows

Download `ZyraSetup-1.0.2.exe` and run the installer.

### Linux

```bash
curl -sSL https://github.com/cowoksoftspoken/Zyra/releases/download/v1.0.2/install.sh | bash
```

### Build from Source

```bash
git clone https://github.com/cowoksoftspoken/Zyra.git
cd Zyra
cargo build --release
```

---

## 🔧 Breaking Changes

None

## 📝 Full Changelog

https://github.com/cowoksoftspoken/Zyra/compare/v1.0.1...v1.0.2
