
## Build Commands

### Setup

```bash
# Install Rust toolchain (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Update Rust to latest stable version
rustup update

# Install additional components (optional)
rustup component add rustfmt clippy
```

### Development

```bash
# Build the project (debug - use during development)
cargo build

# Run the application
cargo run

# Run with arguments
cargo run -- [args]

# Check code without building (faster than build)
cargo check

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all
```

### Build & Deploy

```bash
# Build for release (optimized - use for final testing/deployment only)
cargo build --release

# Run release build
cargo run --release

# Build with verbose output
cargo build --verbose

# Clean build artifacts
cargo clean
```

### Documentation

```bash
# Generate and open project documentation
cargo doc --open

# Generate documentation for dependencies too
cargo doc --no-deps --open
```

### Dependency Management

```bash
# Update dependencies to latest compatible versions
cargo update

# Add a new dependency
cargo add <crate_name>

# Check for outdated dependencies (requires cargo-outdated)
cargo outdated

# Audit dependencies for security vulnerabilities (requires cargo-audit)
cargo audit
```

**Important**: Always use debug builds (`cargo build`) during development. Debug builds compile faster and include debugging symbols. Only use release builds (`cargo build --release`) for final testing or deployment.
