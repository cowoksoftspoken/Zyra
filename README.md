<div align="center">
  <img src="extensions/ZyraFileIcons/icons/zyra.svg" alt="Zyra Logo" width="180">
  <h1>Zyra Programming Language</h1>

  <p>
    <strong>A modern, memory-safe, and deterministic programming language built in Rust.</strong>
  </p>

  <p>
    <a href="https://github.com/cowoksoftspoken/Zyra/releases/latest">
      <img src="https://img.shields.io/badge/version-1.0.0-blue.svg?style=flat-square" alt="Version">
    </a>
    <a href="LICENSE">
      <img src="https://img.shields.io/badge/license-MIT-green.svg?style=flat-square" alt="License">
    </a>
    <a href="https://github.com/cowoksoftspoken/Zyra/actions">
      <img src="https://img.shields.io/github/actions/workflow/status/cowoksoftspoken/Zyra/rust.yml?branch=main&style=flat-square" alt="Build Status">
    </a>
  </p>

  <h4>
    <a href="#key-features">Key Features</a> •
    <a href="#quick-start">Quick Start</a> •
    <a href="#installation">Installation</a> •
    <a href="#documentation">Documentation</a>
  </h4>
</div>

---

## Why Zyra?

Zyra is designed to bring the safety and performance of systems programming to a higher-level, game-development-focused syntax. It combines a **custom compiler** and **lightweight virtual machine** with compile-time memory safety via **ownership, borrowing, and lifetime checking**.

> **"Fast like C, safe like Rust, simple like Python."**

This design enables fast, deterministic, and garbage-collection-free execution, making it perfect for real-time applications like games.

## Key Features

| Feature                       | Description                                                                     |
| :---------------------------- | :------------------------------------------------------------------------------ |
| **🛡️ Memory Safe**            | Ownership & Borrowing system prevents data races and segfaults at compile time. |
| **🚀 Zero Cost Abstractions** | Compiles to efficient bytecode for a fast custom VM execution.                  |
| **📦 Smart Modules**          | Clean namespace management with `import std::game`.                             |
| **🎮 Game Engine**            | Built-in 2D game engine in standard library (Window, Input, Graphics).          |
| **🔧 Zero Null**              | No null values—`Option` types used for safety everywhere.                       |
| **🛠️ Project Management**     | Built-in `zyra` CLI tool for `init`, `run`, and `build` workflows.              |

---

## Quick Start

### 1. Initialize a Project

Create a new project structure with `zyra.toml` automatically configred.

```bash
zyra init my_game
cd my_game
```

### 2. Run

The CLI automatically looks for `zyra.toml` to find your entry point.

```bash
zyra run
```

### 3. Compile

Compile your code to portable bytecode (`.zyc`).

```bash
zyra compile
```

> **Note**: Output directory is configurable in `zyra.toml`.

---

## Syntax Showcase

### Clean & Safe

Zyra uses type inference and strong typing to keep code clean but safe.

```rust
// Variables
let score = 0;              // Inferred as int
let mut speed = 5.5;        // Mutable float
let name: string = "Zyra";  // Explicit type

// Structs & Methods
struct Player {
    name: string,
    score: int,
}

impl Player {
    func new(name: string) -> Player {
        Player { name, score: 0 }
    }

    func level_up(&mut self) {
        self.score += 100;
    }
}
```

### Game Development Ready

Zyra comes with a built-in game framework (`std::game`) that handles windows, input, and rendering out of the box.

```rust
import std::game;
import std::time;

func main() {
    // Create an 800x600 window
    let win = Window(800, 600, "My Awesome Game");

    while game::is_open() {
        game::clear();

        // Handle Input
        if game::key_pressed("W") {
             println("Moving Up!");
        }

        // Draw entities using the standard library
        game::draw_rect(10, 10, 50, 50);
        game::draw_number(100, 100, 42, 2); // X, Y, Number, Scale

        game::display();
        time::sleep(16); // ~60 FPS cap
    }
}
```

---

## Documentation

### Project Configuration (`zyra.toml`)

Every Zyra project is managed by a simple `zyra.toml` file.

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

### Standard Library Modules

| Module          | Functionality                                                           |
| :-------------- | :---------------------------------------------------------------------- |
| **`std::io`**   | Standard Input/Output: `print`, `println`, `input`.                     |
| **`std::math`** | Math utilities: `abs`, `min`, `max`, `sqrt`, `random`, `clamp`.         |
| **`std::time`** | Time management: `now`, `sleep`, `delta_time`.                          |
| **`std::game`** | Core game engine: Window management, primitive drawing, input handling. |

---

## Installation

### Prerequisites

- **Rust 1.70+** must be installed on your system.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/cowoksoftspoken/Zyra.git
cd Zyra

# Build the release binary
cargo build --release
```

The binary will be located at `target/release/zyra`. You can add this to your PATH for global access.

---

## License

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.
