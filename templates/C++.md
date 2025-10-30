### Code Style & Standards

- Follow modern C++ best practices (C++23 standard)
- Use RAII principles for resource management
- Prefer smart pointers over raw pointers
- Use const-correctness
- All function input parameters should be const (e.g., `void SetTitle(const std::string& title)`)
- All functions that return class data without modification should be const (e.g., `std::string GetTitle() const`)
- All destructors should be virtual (even when deleted)
- All abstract/interface classes should have a protected virtual destructor
- All function input parameters should be const (e.g., `void Foo(const int value)`)
- Always use modern C++ line comments (//) even for multi-line comments
- This style provides extra safety and consistency throughout the codebase.
- Follow consistent naming conventions:
  - Types (classes, structs, enums): Upper PascalCase (e.g., `Episode`, `SharedObject`)
  - Functions/methods: Upper PascalCase (e.g., `GetTitle`, `SetDuration`)
  - Variables and function parameters: camelCase (e.g., `bufferSize`, `episodeCount`)
  - Member variables: camelCase with underscore postfix (e.g., `dataSize_`, `title_`)
  - Constants: UPPER_SNAKE_CASE (e.g., `MAX_EPISODE_LENGTH`)
- Remove redundant prefixes from class names (e.g., use `Model` instead of `P3Model`)

### Documentation

- Document public APIs with clear comments
- Use detailed Doxygen-style comments in header files with traditional form:
  - Use /// for Doxygen comments
  - Use \param for parameters, \return for return values, \brief for brief descriptions
  - Example: /// \brief Sets the episode title
  ///         \param title The new title for the episode
- Use Graphviz DOT for class diagrams and dependency diagrams:
  - Use @dot...@enddot blocks for custom graphs
  - Good for showing data flow, component relationships, and class structures
  - Use professional styling with white backgrounds and clear fonts
  - Example: @dot digraph example { ... } @enddot
- **UML Diagram Guidelines**: Treat String, Guid, Timestamp, Timespan as primitive types
  - These should appear as attributes in class diagrams, not as separate class boxes
  - Focus diagrams on domain model relationships, not implementation utility types
  - Keep diagrams clean by treating runtime utilities as built-in primitives
- Implementation files (.cpp) should use inline documentation with // comments for logic explanation
- Keep README.md updated with build instructions and usage examples
- Document any dependencies and requirements
- **CRITICAL: Always verify documentation against actual implementation**
  - README.md must show real API patterns, not fictional functions
  - Use actual struct member names and types from header files
  - Integration examples must use real function signatures and member access patterns
  - All type names must match implementation (e.g., `Guid` not `GUID`)
