### CMake Guidelines

- Use modern CMake practices (4.0+)
- **ALWAYS configure CMake with: `cmake -G Ninja -B _build`**
- **ALWAYS build with: `cmake --build _build`**
- Always use Ninja generator for builds (`cmake -G Ninja -B _build`)
- Use version.in file for version management instead of hardcoding versions
- Organize CMakeLists.txt files hierarchically
- Use target-based approach with `target_*` commands
- Set appropriate compiler flags and warnings
- Default to Debug build type for development
- Create cmake/ directory for package config templates (e.g., `cmake/p3-model-config.cmake.in`)
