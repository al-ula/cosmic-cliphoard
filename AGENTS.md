# AGENTS.md

## Project Overview

- **Language**: Rust (Edition 2024)
- **Project Root**: `cosmic-cliphoard-master/`
- **License**: MPL-2.0

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `cliphoard` | Main binary (overlay, applet, tray, CLI) |
| `cliphoard-daemon` | D-Bus daemon, clipboard watcher |
| `cliphoard-decode` | MIME detection, content decoding |
| `cliphoard-schema` | Shared types, D-Bus interface, codecs |

## Build/Lint/Test Commands

All commands should be run from `cosmic-cliphoard-master/`:

```bash
# Build (debug)
just build

# Build (release)
just build-release

# Run clippy (pedantic)
just check

# Run all tests
just test

# Run single test
cargo test -p <crate> <test_name>
# Example: cargo test -p cliphoard-schema test_entry_id

# Run specific test binary
cargo test --test <test_name>

# Clean build artifacts
just clean
```

## Code Style

### File Headers
```rust
// SPDX-License-Identifier: MPL-2.0

//! Module documentation.
```

### Imports
- Group: std → external crates → local crates
- Use `cliphoard_schema::{}` for shared types

### Formatting
- Max line length: 100 characters
- Use 4-space indentation

### Naming
- `snake_case` for functions, variables, modules
- `PascalCase` for types, traits, structs
- `SCREAMING_SNAKE_CASE` for constants

### Error Handling
- Use `thiserror` for error types
- Return `Result<T, Box<dyn std::error::Error>>` for main functions
- Use `?` operator for error propagation

### Async
- Use `tokio` runtime
- `#[tokio::main]` for async main
- `Arc<RwLock<T>>` for shared mutable state

### D-Bus
- Use `zbus` v5 with tokio feature
- Interface: `com.github.al_ula.Cliphoard.Manager`
- Path: `/com/github/al_ula/Cliphoard`

## CLI Commands

```bash
cliphoard              # Layer-shell overlay (default)
cliphoard daemon       # D-Bus daemon
cliphoard applet       # COSMIC panel applet
cliphoard tray         # System tray
cliphoard list         # List entries
cliphoard paste <id>   # Paste by ID
cliphoard delete <id>  # Delete by ID
cliphoard clear        # Clear history
```

## Key Dependencies

- `zbus` 5 - D-Bus IPC
- `tokio` - Async runtime
- `thiserror` - Error handling
- `libcosmic` - COSMIC GUI framework
- `wayland-clipboard-listener` - Wayland clipboard
