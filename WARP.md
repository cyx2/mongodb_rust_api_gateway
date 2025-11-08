# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

This is a Rust project repository, currently empty and ready for initialization.

## Getting Started

To initialize this as a Rust project, run:
```bash
cargo init
```

Or to create a library crate instead:
```bash
cargo init --lib
```

## Common Commands

### Building
```bash
cargo build              # Debug build
cargo build --release    # Optimized release build
```

### Running
```bash
cargo run                # Build and run the binary
cargo run -- [args]      # Pass arguments to the binary
```

### Testing
```bash
cargo test              # Run all tests
cargo test [test_name]  # Run specific test
cargo test -- --nocapture  # Show println! output from tests
```

### Linting and Formatting
```bash
cargo fmt               # Format code according to Rust style
cargo clippy            # Run Clippy linter
cargo clippy -- -D warnings  # Fail on warnings
```

### Dependencies
```bash
cargo add [crate]       # Add a dependency
cargo update            # Update dependencies
cargo tree              # View dependency tree
```

### Documentation
```bash
cargo doc --open        # Build and open documentation
```

## Project Structure

Standard Rust project structure:
- `Cargo.toml` - Project manifest with dependencies and metadata
- `src/main.rs` - Binary entry point (for executables)
- `src/lib.rs` - Library entry point (for libraries)
- `src/bin/` - Additional binary targets
- `tests/` - Integration tests
- `benches/` - Benchmarks
- `examples/` - Example code

## Development Notes

- The `.gitignore` is configured for Cargo projects, including:
  - `target/` directory (build artifacts)
  - `debug` directory
  - Backup files from rustfmt (`*.rs.bk`)
  - Mutation testing output (`mutants.out*/`)
  - MSVC debug files (`*.pdb`)
