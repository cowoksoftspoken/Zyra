# Zyra Programming Language

![Zyra Logo](/extensions/ZyraFileIcons/icons/zyra.svg)

A modern, memory-safe, and deterministic programming language built in Rust.

**Version:** 1.0.0 (Stable Release)

## Overview

**Zyra** A modern, statically typed and deterministic programming language built in Rust. Zyra combines a custom compiler and lightweight virtual machine with compile-time memory safety via ownership, borrowing, and lifetime checking. This design enables fast, predictable, and garbage-collection-free execution.

## Key Features

- **🛡️ Memory Safe**: Ownership & Borrowing system prevents data races and segfaults at compile time.
- **🚀 Zero Cost Abstractions**: Compiles to efficient bytecode for a fast VM execution.
- **📦 Smart Module System**: Clean namespace management (`import std::game`).
- **🎮 Game Ready**: Built-in 2D game engine in standard library (Window, Input, Graphics).
- **🔧 Zero Null**: No null values - Option types used for safety.
- **🛠️ Project Management**: `zyra.toml` handling for consistent project builds.

---

## Quick Start

### 1. Initialize a Project

Create a new project structure with `zyra.toml`:

```bash
zyra init my_game
cd my_game
```

### 2. Run

Automatically finds the main entry file defined in `zyra.toml`:

```bash
zyra run
```

### 3. Compile

Compiles your code to bytecode (`.zyc`):

```bash
zyra compile
```

_Output directory is configurable in `zyra.toml`._

---

## Project Configuration (zyra.toml)

Every Zyra project can have a `zyra.toml` file to manage build settings:

```toml
[project]
name = "pong_game"
version = "1.0.0"
edition = "2025"
zyra = ">=1.0.0"

[build]
main_entry = "main.zr"   # Entry point file
output = "./dist/"       # Output directory for compiled files
```

---

## Syntax Examples

### Variables & Types

```zyra
let score = 0;              // Inferred as int
let mut speed = 5.5;        // Mutable float
let name: string = "Zyra";  // Explicit type
```

### Functions & Structs

```zyra
struct Player {
    name: string,
    score: int
}

impl Player {
    func new(name: string) -> Player {
        Player { name, score: 0 }
    }

    func level_up(&mut self) {
        self.score = self.score + 100;
    }
}
```

### Game Development (Standard Library)

Zyra comes with a built-in game framework:

```zyra
import std::game;
import std::time;

func main() {
    let win = Window(800, 600, "My Game");

    while game::is_open() {
        game::clear();

        if game::key_pressed("W") {
             println("Moving Up!");
        }

        // Draw entities
        game::draw_rect(10, 10, 50, 50);
        game::draw_number(100, 100, 42, 2); // X, Y, Number, Scale

        game::display();
        time::sleep(16);
    }
}
```

---

## Standard Library Modules

- **std::io**: `print`, `println`, `input`
- **std::math**: `abs`, `min`, `max`, `sqrt`, `random`, `clamp`
- **std::time**: `now`, `sleep`
- **std::game**: Window management, primitive drawing, input handling

---

## Installation

Requirements: Rust 1.70+

```bash
git clone https://github.com/cowoksoftspoken/Zyra.git
cd Zyra
cargo build --release
```

The binary is located at `target/release/zyra`.

---

## License

MIT License. See [LICENSE](LICENSE) for details.
