
## Build Commands

### Setup

```bash
# Install CMake (if not already installed)
# macOS
brew install cmake

# Linux (Debian/Ubuntu)
sudo apt-get install cmake

# Linux (Fedora/RHEL)
sudo dnf install cmake

# Check CMake version
cmake --version
```

### Development

```bash
# Configure project (debug build - use during development)
cmake -B build -DCMAKE_BUILD_TYPE=Debug

# Build the project (debug)
cmake --build build

# Build with verbose output
cmake --build build --verbose

# Build specific target
cmake --build build --target target_name

# Run tests (if using CTest)
cd build && ctest

# Run tests with verbose output
cd build && ctest --verbose

# Run specific test
cd build && ctest -R test_name

# Clean build artifacts
cmake --build build --target clean

# Reconfigure from scratch
rm -rf build
cmake -B build -DCMAKE_BUILD_TYPE=Debug
```

### Build & Deploy

```bash
# Configure for release (optimized - use for final testing/deployment only)
cmake -B build -DCMAKE_BUILD_TYPE=Release

# Build release version
cmake --build build --config Release

# Install to system (requires proper CMAKE_INSTALL_PREFIX)
cmake --build build --target install

# Create distributable package (if configured)
cd build && cpack
```

### Advanced Options

```bash
# Configure with custom compiler
cmake -B build -DCMAKE_C_COMPILER=gcc -DCMAKE_CXX_COMPILER=g++

# Configure with custom install prefix
cmake -B build -DCMAKE_INSTALL_PREFIX=/path/to/install

# Configure with additional flags
cmake -B build -DCMAKE_CXX_FLAGS="-Wall -Wextra"

# Enable/disable specific features (example)
cmake -B build -DENABLE_FEATURE=ON

# Generate compile_commands.json for IDE/tools
cmake -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

# List available targets
cmake --build build --target help
```

### Static Analysis & Formatting

```bash
# Run clang-tidy (if configured)
cmake --build build --target clang-tidy

# Run clang-format to check formatting
find src include -name '*.cpp' -o -name '*.h' | xargs clang-format -n

# Apply clang-format
find src include -name '*.cpp' -o -name '*.h' | xargs clang-format -i

# Run cppcheck (if installed)
cppcheck --enable=all --project=build/compile_commands.json
```

### Multi-Configuration Generators

```bash
# For Visual Studio, Xcode, or Ninja Multi-Config
cmake -B build -G "Ninja Multi-Config"

# Build debug configuration
cmake --build build --config Debug

# Build release configuration
cmake --build build --config Release

# Build with multiple jobs (parallel)
cmake --build build --config Release -j 8
```

**Important**: Always use Debug builds (`-DCMAKE_BUILD_TYPE=Debug`) during development. Debug builds compile faster, include debugging symbols, and provide better error diagnostics. Only use Release builds (`-DCMAKE_BUILD_TYPE=Release`) for final testing or deployment.
