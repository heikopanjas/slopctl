
## Build Commands

### Setup

```bash
# Install Make (usually pre-installed on Unix-like systems)
# macOS (Xcode Command Line Tools)
xcode-select --install

# Linux (Debian/Ubuntu)
sudo apt-get install build-essential

# Linux (Fedora/RHEL)
sudo dnf groupinstall "Development Tools"

# Check Make version
make --version
```

### Development

```bash
# Build the project (debug - use during development)
make

# Build with debug flags
make DEBUG=1

# Build with verbose output
make VERBOSE=1

# Build specific target
make target_name

# Run tests (if Makefile has test target)
make test

# Run specific test
make test-name

# Clean build artifacts
make clean

# Clean everything including dependencies
make distclean

# Rebuild from scratch
make clean && make
```

### Build & Deploy

```bash
# Build release version (optimized - use for final testing/deployment only)
make RELEASE=1

# Build with optimizations
make CFLAGS="-O3 -march=native"

# Install to system (requires proper PREFIX)
make install

# Install to custom location
make PREFIX=/path/to/install install

# Uninstall
make uninstall
```

### Advanced Options

```bash
# Build with custom compiler
make CC=gcc CXX=g++

# Build with custom flags
make CFLAGS="-Wall -Wextra -Werror"

# Build with specific number of parallel jobs
make -j8

# Build with automatic parallelism
make -j$(nproc)

# Dry run (show commands without executing)
make -n

# Print debug information
make --debug

# Show all defined variables
make -p

# Continue on errors
make -k
```

### Static Analysis & Formatting

```bash
# Run clang-tidy (if configured in Makefile)
make clang-tidy

# Run clang-format to check formatting
find src include -name '*.c' -o -name '*.h' | xargs clang-format -n

# Apply clang-format
find src include -name '*.c' -o -name '*.h' | xargs clang-format -i

# Run cppcheck (if installed)
make cppcheck

# Generate code coverage (if configured)
make coverage

# Run static analysis (if configured)
make analyze
```

### Dependency Management

```bash
# Generate dependency files
make depend

# Update dependencies
make deps

# Show dependencies for specific target
make -d target_name

# Check prerequisites
make -q target_name

# Rebuild only if dependencies changed
make -B target_name
```

### Common Makefile Targets

```bash
# Standard targets (if implemented)
make all          # Build everything
make build        # Alias for default target
make clean        # Remove build artifacts
make distclean    # Remove all generated files
make install      # Install built files
make uninstall    # Remove installed files
make test         # Run test suite
make check        # Alias for test
make dist         # Create distribution tarball
make help         # Show available targets
```

**Important**: Always use debug builds during development. Debug builds compile faster and include debugging symbols. Only use optimized release builds for final testing or deployment. Most Makefiles default to debug mode unless `RELEASE=1` or similar is specified.
