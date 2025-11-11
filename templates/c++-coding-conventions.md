
## Coding Conventions

**General Principles:**

- Follow modern C++ best practices (C++23 standard)
- Use RAII principles for resource management
- Prefer smart pointers (`std::unique_ptr`, `std::shared_ptr`) over raw pointers
- Use const-correctness throughout the codebase
- This style provides extra safety and consistency throughout the codebase
- **Namespace**: All compiler code uses the `bbfm` namespace

**Functions and Methods:**

- All function input parameters should be const (e.g., `void SetTitle(const std::string& title)`)
- All functions that return class data without modification should be const (e.g., `std::string GetTitle() const`)
- Pass by const reference for complex types, by value for primitives
- Use trailing return types when it improves clarity
- For intentionally unused parameters, use the `UNREFERENCED_PARAMETER(param)` macro from `Common.h`

**Classes and Destructors:**

- All destructors should be virtual (even when deleted)
- All abstract/interface classes should have a protected virtual destructor
- Use the Rule of Zero when possible (let compiler generate special members)
- When implementing special members, follow the Rule of Five
- **File organization**: Each class should have a separate header (.h) and implementation (.cpp) file
  - Filename must match the class name exactly (e.g., `Driver` class → `Driver.h` and `Driver.cpp`)
  - Header files go in `include/` directory
  - Implementation files go in `src/` directory
  - Exception: Template classes may have implementation in header if needed
  - Exception: Tightly coupled class hierarchies (like AST nodes) may share a single header/implementation file pair
- **Implementation separation**: Method implementations should be in .cpp files, not inline in headers
  - Reduces recompilation of dependencies when implementation changes
  - Only constructors and trivial one-line getters may remain inline in headers if needed for performance
  - Prefer out-of-line implementations for better compilation times
- **Scope declaration order**: Always declare scopes in the order: `public`, `protected`, `private`
  - This makes the public interface immediately visible when reading class definitions
  - Example:

    ```cpp
    class MyClass {
    public:
        MyClass();
        virtual ~MyClass();
        void PublicMethod();

    protected:
        void ProtectedMethod();

    private:
        int privateData_;
    };
    ```

**Comparison and Conditional Expressions:**

- Always place constants on the left side of comparisons
- Use explicit `nullptr` comparisons instead of implicit boolean conversion
- Examples:
  - ✅ Correct: `if (nullptr == ast_)` or `if (0 == value)`
  - ❌ Incorrect: `if (!ast_)` or `if (ast_ == nullptr)`
- Reasoning: Prevents accidental assignment (`=`) instead of comparison (`==`); compiler will error on `nullptr = ast_` but may allow `ast_ = nullptr`
- Apply to all comparisons with literals, nullptr, and constants

**Naming Conventions:**

- Types (classes, structs, enums, typedefs): Upper PascalCase (e.g., `Episode`, `SharedObject`, `MediaType`)
- Functions/methods: Upper PascalCase (e.g., `GetTitle`, `SetDuration`, `ParseInput`)
- Variables and function parameters: camelCase (e.g., `bufferSize`, `episodeCount`, `userName`)
- Member variables: camelCase with underscore postfix (e.g., `dataSize_`, `title_`, `description_`)
- Constants: UPPER_SNAKE_CASE (e.g., `MAX_EPISODE_LENGTH`, `DEFAULT_TIMEOUT`)
- Remove redundant prefixes from class names (e.g., use `Model` instead of `P3Model`)
- **Include guards**: Use format `__BBFM_CLASS_NAME_H_INCL__` where CLASS_NAME matches the class declared in the file
  - Must start with `__BBFM_` prefix to identify project namespace
  - Single word class: `Driver` → `__BBFM_DRIVER_H_INCL__`
  - Multi-word class: `TestTools` → `__BBFM_TEST_TOOLS_H_INCL__`
  - Insert underscore between each word in PascalCase class names
  - Examples: `Driver` → `__BBFM_DRIVER_H_INCL__`, `SemanticAnalyzer` → `__BBFM_SEMANTIC_ANALYZER_H_INCL__`, `AST` → `__BBFM_AST_H_INCL__`

**Header File Structure:**

- All header files must use 8-byte alignment for types using `#pragma pack`
- Include alignment pragmas at the top (after include guard) and restore at the bottom (before closing include guard)
- Use cross-compiler compatible pragmas for MSVC, GCC, and Clang:

  ```cpp
  // At top of header (after include guard, before includes)
  #pragma pack(push, 8)

  // At bottom of header (before closing include guard)
  #pragma pack(pop)
  ```

**Comments and Documentation:**

- Always use modern C++ line comments (`//`) even for multi-line comments
- Document public APIs with clear Doxygen-style comments in header files
- Use traditional Doxygen syntax:
  - `///` for Doxygen comments
  - `\brief` for brief descriptions
  - `\param` for parameters
  - `\return` for return values
  - Example:

    ```cpp
    /// \brief Sets the episode title
    /// \param title The new title for the episode
    void SetTitle(const std::string& title);
    ```

- Implementation files (.cpp) should use inline `//` comments for logic explanation
- Keep comments concise and focused on "why" rather than "what"

**Documentation Tools:**

- Use Graphviz DOT for class diagrams and dependency diagrams:
  - Use `@dot...@enddot` blocks for custom graphs
  - Good for showing data flow, component relationships, and class structures
  - Use professional styling with white backgrounds and clear fonts
  - Example: `@dot digraph example { ... } @enddot`
- **UML Diagram Guidelines**: Treat String, Guid, Timestamp, Timespan as primitive types
  - These should appear as attributes in class diagrams, not as separate class boxes
  - Focus diagrams on domain model relationships, not implementation utility types
  - Keep diagrams clean by treating runtime utilities as built-in primitives

**Documentation Accuracy:**

- **CRITICAL: Always verify documentation against actual implementation**
- README.md must show real API patterns, not fictional functions
- Use actual struct member names and types from header files
- Integration examples must use real function signatures and member access patterns
- All type names must match implementation (e.g., `Guid` not `GUID`)
- Keep documentation synchronized with code changes
