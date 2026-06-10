# Rust4D Documentation

Welcome to the Rust4D documentation. This is a 4D rendering engine written in Rust that displays real-time 3D cross-sections of 4D geometry.

## Documentation Guides

| Guide | Description | Audience |
|-------|-------------|----------|
| [Getting Started](./getting-started.md) | Installation, first steps, 4D concepts | New users |
| [User Guide](./user-guide.md) | Comprehensive feature reference | All users |
| [Developer Guide](./developer-guide.md) | Architecture, algorithms, contributing | Contributors |
| [The Mathematics of Rust4D](./4d-math.md) | Rotors, SkipY, slicing, the slice invariant, conventions | Contributors |
| [Shape Catalog](./shapes.md) | Built-in 4D primitives, construction math, scene syntax, verification | Users & contributors |

## Quick Start

```bash
# Clone and run
git clone https://github.com/sigilmakes/Rust4D
cd Rust4D
cargo run --release   # or: nix develop --command cargo run --release

# Run an example
cargo run --example 01_hello_tesseract --release
```

## Documentation Structure

```
docs/
├── README.md           <- You are here
├── getting-started.md  <- Start here if new
├── user-guide.md       <- Feature reference
└── developer-guide.md  <- For contributors
```

## Additional Resources

- [Architecture Overview](../ARCHITECTURE.md) - System design with diagrams
- [Examples](../examples/README.md) - Runnable code examples
- [Default Configuration](../config/default.toml) - Configuration reference
- [Default Scene](../scenes/default.ron) - Scene file format example

## Learning Path

1. **New to Rust4D?** Start with [Getting Started](./getting-started.md)
2. **Building something?** Reference the [User Guide](./user-guide.md)
3. **Want to contribute?** Read the [Developer Guide](./developer-guide.md)

## Getting Help

- Check the [Troubleshooting](./user-guide.md#troubleshooting) section
- Open an issue on GitHub
- Read the inline documentation: `cargo doc --open`

## Controls Quick Reference

| Input | Action |
|-------|--------|
| WASD | Move in XZ plane |
| Q / E | Move along W-axis (4th dimension) |
| Space / Shift | Move up/down |
| Mouse | Look around |
| Right-click drag | Rotate through W |
| Escape | Release cursor / Quit |
