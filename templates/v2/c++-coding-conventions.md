
## C++ Coding Conventions

**General Principles:**

- Follow modern C++ best practices (C++23 standard preferred, C++17 minimum)
- Use RAII principles for resource management
- Prefer smart pointers (`std::unique_ptr`, `std::shared_ptr`) over raw pointers
- Apply const-correctness throughout the codebase
- Write self-documenting code with clear naming and structure
- Keep functions focused and modular
- Leverage the type system for compile-time safety
- Ensure platform portability (Linux, macOS, Windows)

**C++ Standard and Compatibility:**

- Use **C++23 standard** when possible for latest features
- Maintain **C++17 minimum** for broader compiler support
- Use standard library features over custom implementations
- Avoid compiler-specific extensions unless necessary
- Test on multiple compilers (GCC, Clang, MSVC)
- Use feature test macros for conditional compilation
- Handle platform differences through standard mechanisms

**Const Correctness:**

- All input parameters should be `const` when not modified
- Member functions that don't modify state should be `const`
- Use `const` references for complex types in parameters
- Apply `const` to return values when appropriate
- Examples:
  - ✅ Correct: `void SetTitle(const std::string& title);`
  - ✅ Correct: `std::string GetTitle() const;`
  - ✅ Correct: `const Data& GetData() const;`
  - ❌ Incorrect: `void SetTitle(std::string title);` (unnecessary copy)
- Const correctness improves maintainability and enables compiler optimization

**Comparison Conventions:**

- **Always place constants on the left side of comparisons** (constant-left style)
- Use explicit `nullptr` comparisons instead of implicit boolean conversion
- This prevents accidental assignment when `=` is used instead of `==`
- Examples:
  - ✅ Correct: `if (nullptr == ptr)`, `if (0 == value)`, `if (true == condition)`
  - ❌ Incorrect: `if (!ptr)`, `if (ptr == nullptr)`, `if (value == 0)`
- Apply to all comparisons including pointer checks, numeric values, and booleans
- Benefits: Compiler error if `=` is mistakenly used instead of `==`

**RAII and Resource Management:**

- Use RAII for all resource management (memory, files, locks, etc.)
- Prefer smart pointers over raw pointers:
  - `std::unique_ptr` for exclusive ownership
  - `std::shared_ptr` for shared ownership
  - `std::weak_ptr` to break circular dependencies
- Use standard containers instead of manual memory management
- Examples:
  ```cpp
  // Good: RAII with smart pointers
  auto data = std::make_unique<Data>();
  auto shared = std::make_shared<SharedObject>();

  // Good: RAII with containers
  std::vector<int> numbers;
  std::string text;

  // Avoid: Raw pointers requiring manual cleanup
  Data* data = new Data();  // Must remember to delete
  ```
- Let destructors handle cleanup automatically

**Classes and Destructors:**

- All destructors should be virtual (even when deleted)
- All abstract/interface classes should have a protected virtual destructor
- Use the Rule of Zero when possible (let compiler generate special members)
- When implementing special members, follow the Rule of Five
- Declare move constructor and move assignment operator when beneficial
- Examples:
  ```cpp
  // Rule of Zero: Compiler generates all special members
  class Simple
  {
  public:
      Simple() = default;
      std::string name;
      std::vector<int> data;
  };

  // Rule of Five: Custom resource management
  class Resource
  {
  public:
      Resource();
      ~Resource();
      Resource(const Resource& other);
      Resource& operator=(const Resource& other);
      Resource(Resource&& other) noexcept;
      Resource& operator=(Resource&& other) noexcept;
  };
  ```

**File Organization:**

- **Header files (.h)**: Class declarations, inline functions, templates
- **Implementation files (.cpp)**: Method implementations, non-template code
- Each class should have a separate header and implementation file
- Filename must match the class name exactly (e.g., `Driver` class → `Driver.h` and `Driver.cpp`)
- Header files go in `include/` directory
- Implementation files go in `src/` directory
- Exceptions:
  - Template classes may have implementation in header if needed
  - Tightly coupled class hierarchies (like AST nodes) may share files

**Implementation Separation:**

- Method implementations should be in .cpp files, not inline in headers
- Reduces recompilation of dependencies when implementation changes
- Only these may remain inline in headers:
  - Constructors (if trivial)
  - Destructors (if trivial)
  - One-line getters/setters for performance
  - Template functions (required)
- Prefer out-of-line implementations for better compilation times

**Class Structure and Scope Order:**

- Always declare scopes in the order: `public`, `protected`, `private`
- This makes the public interface immediately visible when reading class definitions
- Group related members together within each section
- Example:
  ```cpp
  class MyClass
  {
  public:
      // Constructors and destructor
      MyClass();
      virtual ~MyClass();

      // Public interface
      void PublicMethod();
      int GetValue() const;

  protected:
      // Protected interface for derived classes
      virtual void ProtectedMethod();

  private:
      // Private implementation details
      void PrivateHelper();
      int privateData_;
      std::string privateName_;
  };
  ```

**Naming Conventions:**

- **Types** (classes, structs, enums, typedefs): Upper PascalCase (e.g., `Episode`, `MediaType`)
- **Functions/methods**: Upper PascalCase (e.g., `GetTitle`, `SetDuration`, `ParseInput`)
- **Variables and function parameters**: camelCase (e.g., `bufferSize`, `episodeCount`)
- **Member variables**: camelCase with underscore postfix (e.g., `dataSize_`, `title_`)
- **Constants**: UPPER_SNAKE_CASE (e.g., `MAX_EPISODE_LENGTH`, `DEFAULT_TIMEOUT`)
- **Namespaces**: lowercase (e.g., `myproject`, `utils`)
- **Template parameters**: Single uppercase letter or PascalCase (e.g., `T`, `ValueType`)
- Remove redundant prefixes from class names (e.g., use `Model` instead of `P3Model`)

**Include Guards:**

- Use format `__PROJECT_CLASS_NAME_H_INCL__` where CLASS_NAME matches the class
- Must start with project-specific prefix to identify namespace
- Single word class: `Driver` → `__MYPROJECT_DRIVER_H_INCL__`
- Multi-word class: `TestTools` → `__MYPROJECT_TEST_TOOLS_H_INCL__`
- Insert underscore between each word in PascalCase class names
- Examples:
  ```cpp
  #ifndef __MYPROJECT_DRIVER_H_INCL__
  #define __MYPROJECT_DRIVER_H_INCL__

  // Class declaration

  #endif // __MYPROJECT_DRIVER_H_INCL__
  ```
- Alternative: Use `#pragma once` if all target compilers support it

**Header File Structure:**

- Include guard or `#pragma once` at top
- Includes (system headers first, then project headers)
- Forward declarations (to minimize includes)
- Type definitions and aliases
- Class declarations
- Inline function definitions
- Example:
  ```cpp
  #ifndef __MYPROJECT_CLASS_H_INCL__
  #define __MYPROJECT_CLASS_H_INCL__

  #include <string>
  #include <vector>

  #include "BaseClass.h"

  // Forward declarations
  class Helper;

  class MyClass : public BaseClass
  {
  public:
      // ... class definition
  };

  #endif // __MYPROJECT_CLASS_H_INCL__
  ```

**Alignment Pragmas:**

- All header files must use 8-byte alignment for types using `#pragma pack`
- Include alignment pragmas at the top and restore at the bottom
- Use cross-compiler compatible pragmas for MSVC, GCC, and Clang:
  ```cpp
  // At top of header (after include guard, before includes)
  #pragma pack(push, 8)

  // ... class declarations ...

  // At bottom of header (before closing include guard)
  #pragma pack(pop)
  ```

**Implementation File Organization:**

- Include corresponding header first
- Include system headers
- Include project headers
- Anonymous namespace for file-local helpers
- Class member function implementations
- Example:
  ```cpp
  #include "MyClass.h"

  #include <algorithm>
  #include <iostream>

  #include "Helper.h"

  namespace
  {
      // File-local helper functions
      void LocalHelper()
      {
          // ...
      }
  }

  // Class member implementations
  MyClass::MyClass()
  {
      // ...
  }
  ```

**Namespaces:**

- Use namespaces to organize code logically
- Avoid `using` directives in headers (e.g., `using namespace std;`)
- Use `using` declarations sparingly in implementation files
- Prefer explicit namespace qualification for clarity
- Use nested namespaces for hierarchical organization
- Examples:
  ```cpp
  namespace myproject
  {
      namespace utils
      {
          class Helper { };
      }

      class MainClass { };
  }

  // C++17 nested namespace syntax
  namespace myproject::utils
  {
      class Helper { };
  }
  ```

**Function and Method Design:**

- Keep functions short and focused on single responsibility
- Use early returns to reduce nesting depth
- Pass by const reference for complex types, by value for primitives
- Use trailing return types when it improves clarity (e.g., with `auto`)
- For intentionally unused parameters, use `[[maybe_unused]]` attribute or comment
- Examples:
  ```cpp
  // Good: Clear parameter passing
  void ProcessData(const std::vector<int>& data, int threshold);

  // Good: Trailing return type with auto
  auto GetValue() -> std::optional<int>;

  // Good: Unused parameter handling
  void Handler([[maybe_unused]] int eventType)
  {
      // Implementation doesn't use eventType
  }
  ```

**Type Definitions and Aliases:**

- Use `using` instead of `typedef` for type aliases
- Create meaningful aliases for complex types
- Document the purpose of type aliases
- Examples:
  ```cpp
  // Good: Clear type aliases
  using UserId = uint64_t;
  using ErrorCallback = std::function<void(const std::string&)>;
  using DataMap = std::unordered_map<std::string, std::shared_ptr<Data>>;

  // Avoid: Obscure typedef
  typedef unsigned long long int ull;
  ```

**Enums:**

- Prefer `enum class` over `enum` for type safety
- Use explicit underlying types when needed
- Prefix enum values with enum name for clarity (only if not using `enum class`)
- Examples:
  ```cpp
  // Best: enum class (scoped and type-safe)
  enum class Color : uint8_t
  {
      Red,
      Green,
      Blue
  };

  // Usage: Color::Red

  // Acceptable: Traditional enum with prefix
  enum MediaType
  {
      MEDIA_TYPE_AUDIO,
      MEDIA_TYPE_VIDEO,
      MEDIA_TYPE_SUBTITLE
  };
  ```

**Error Handling:**

- Use exceptions for exceptional conditions
- Use `std::optional` for values that may not exist
- Use `std::expected` (C++23) or similar for expected errors
- Never throw from destructors
- Document exceptions in function comments
- Examples:
  ```cpp
  // Good: Optional for nullable values
  std::optional<User> FindUser(const std::string& name);

  // Good: Exception for errors
  void LoadFile(const std::string& path)
  {
      if (path.empty())
      {
          throw std::invalid_argument("Path cannot be empty");
      }
      // ... load file
  }

  // Good: Error handling with optional
  auto user = FindUser("john");
  if (user.has_value())
  {
      ProcessUser(user.value());
  }
  ```

**Memory Management:**

- Prefer stack allocation over heap allocation when possible
- Use smart pointers for heap-allocated objects
- Use `std::make_unique` and `std::make_shared` for construction
- Avoid naked `new` and `delete`
- Use containers for collections of objects
- Examples:
  ```cpp
  // Good: Smart pointers
  auto data = std::make_unique<Data>();
  auto shared = std::make_shared<Config>();

  // Good: Stack allocation
  Data localData;
  std::array<int, 10> numbers;

  // Good: Containers
  std::vector<std::unique_ptr<Item>> items;
  ```

**Comments:**

- Use `//` for all comments (single-line and multi-line)
- Document public APIs with Doxygen-style comments in header files
- Use traditional Doxygen syntax:
  - `///` for Doxygen comments
  - `\brief` for brief descriptions
  - `\param` for parameters
  - `\return` for return values
- Implementation files should use inline `//` comments for logic explanation
- Comment the "why" not the "what"
- Examples:
  ```cpp
  /// \brief Sets the episode title
  /// \param title The new title for the episode
  void SetTitle(const std::string& title);

  // Implementation comment explaining reasoning
  // Use binary search because data is sorted
  auto it = std::lower_bound(data.begin(), data.end(), target);
  ```

**Code Formatting:**

- Use consistent indentation (4 spaces preferred)
- Braces: Opening brace on next line for functions and blocks
- Example:
  ```cpp
  // Function: opening brace on next line
  void MyClass::ProcessData(const std::vector<int>& data)
  {
      // Control structure: opening brace on next line
      if (nullptr == data_)
      {
          Initialize();
      }

      for (const auto& item : data)
      {
          ProcessItem(item);
      }
  }
  ```
- Line length: Keep under 120 characters when practical
- Use `.clang-format` configuration for automatic formatting

**Modern C++ Features:**

- Use `auto` for type deduction when type is obvious from context
- Use range-based for loops instead of iterators when possible
- Use structured bindings (C++17) for multiple return values
- Use `std::string_view` for non-owning string references
- Use `constexpr` for compile-time constants
- Examples:
  ```cpp
  // Good: auto for obvious types
  auto config = std::make_unique<Config>();
  auto it = container.find(key);

  // Good: Range-based for
  for (const auto& item : items)
  {
      ProcessItem(item);
  }

  // Good: Structured bindings
  auto [success, value] = TryParse(input);

  // Good: string_view
  void ProcessName(std::string_view name);

  // Good: constexpr
  constexpr int MAX_SIZE = 1024;
  ```

**Templates:**

- Keep template code in headers (required by C++ standard)
- Use concepts (C++20) to constrain template parameters
- Provide clear error messages for template failures
- Document template parameters and requirements
- Examples:
  ```cpp
  // C++20 concepts
  template<typename T>
  concept Drawable = requires(T obj)
  {
      obj.Draw();
  };

  template<Drawable T>
  void Render(const T& object)
  {
      object.Draw();
  }

  // Traditional template with static_assert
  template<typename T>
  class Container
  {
      static_assert(std::is_default_constructible_v<T>,
                    "T must be default constructible");
  };
  ```

**Lambda Expressions:**

- Use lambdas for short, local operations
- Capture by reference `[&]` for local scope, by value `[=]` when needed
- Be explicit with captures when clarity is important
- Use `mutable` when lambda needs to modify captured values
- Examples:
  ```cpp
  // Good: Short algorithm
  std::sort(items.begin(), items.end(),
            [](const Item& a, const Item& b)
            {
                return a.priority > b.priority;
            });

  // Good: Explicit captures
  int threshold = 10;
  auto filter = [threshold](int value)
  {
      return value > threshold;
  };

  // Good: Mutable lambda
  int counter = 0;
  auto increment = [counter]() mutable
  {
      return ++counter;
  };
  ```

**Standard Library Usage:**

- Prefer standard library over custom implementations
- Use algorithms from `<algorithm>` header
- Use standard containers (`vector`, `map`, `set`, etc.)
- Use `<string>` for string handling
- Use `<filesystem>` (C++17) for file operations
- Examples:
  ```cpp
  // Good: Standard algorithms
  std::sort(data.begin(), data.end());
  auto it = std::find_if(items.begin(), items.end(), predicate);

  // Good: Standard containers
  std::vector<int> numbers;
  std::unordered_map<std::string, Data> cache;

  // Good: Filesystem operations
  std::filesystem::path filePath = "/path/to/file";
  if (std::filesystem::exists(filePath))
  {
      // Process file
  }
  ```

**Const and Constexpr:**

- Use `const` for runtime constants
- Use `constexpr` for compile-time constants
- Use `consteval` (C++20) to force compile-time evaluation
- Mark functions `constexpr` when possible for compile-time optimization
- Examples:
  ```cpp
  // Runtime constant
  const int bufferSize = GetBufferSize();

  // Compile-time constant
  constexpr int MAX_USERS = 100;

  // Constexpr function
  constexpr int Square(int x)
  {
      return x * x;
  }

  // C++20 consteval (must be compile-time)
  consteval int Factorial(int n)
  {
      return (n <= 1) ? 1 : n * Factorial(n - 1);
  }
  ```

**Platform Portability:**

- Use standard C++ features when possible
- Handle platform differences through preprocessor or runtime checks
- Test on multiple platforms (Linux, macOS, Windows)
- Use standard integer types from `<cstdint>`
- Examples:
  ```cpp
  #ifdef _WIN32
      // Windows-specific code
      #include <windows.h>
  #else
      // POSIX code
      #include <unistd.h>
  #endif

  // Use standard fixed-size types
  uint32_t value32;
  int64_t offset;
  ```

**Compiler Warnings:**

- Build with strict warnings enabled:
  - GCC/Clang: `-Wall -Wextra -Wpedantic`
  - MSVC: `/W4`
- Treat warnings as errors in development builds
- Fix all warnings - don't suppress them unless absolutely necessary
- Document any warning suppressions with reasoning

**Testing Strategy:**

- Write unit tests for all public APIs
- Test edge cases: null pointers, empty containers, boundary values
- Use test frameworks (Google Test, Catch2, etc.)
- Mock dependencies for isolated testing
- Test on all target platforms
- Examples:
  ```cpp
  TEST(MyClassTest, ConstructorInitializesCorrectly)
  {
      MyClass obj;
      EXPECT_EQ(0, obj.GetValue());
  }

  TEST(MyClassTest, SetValueUpdatesCorrectly)
  {
      MyClass obj;
      obj.SetValue(42);
      EXPECT_EQ(42, obj.GetValue());
  }
  ```

**Documentation:**

- Document all public APIs in header files
- Include purpose, parameters, return values, and exceptions
- Use Doxygen format for API documentation
- Examples:
  ```cpp
  /// \brief Creates a new user account
  /// \param username The unique username for the account
  /// \param email The user's email address
  /// \return A unique pointer to the created User object
  /// \throws std::invalid_argument if username is empty
  std::unique_ptr<User> CreateUser(
      const std::string& username,
      const std::string& email);
  ```

**Documentation Tools:**

- Use Doxygen for API documentation generation
- Use Graphviz DOT for class diagrams and dependency diagrams
- Use `@dot...@enddot` blocks for custom graphs
- Keep diagrams clean and focused on domain relationships
- Treat standard types (String, etc.) as primitives in diagrams

**Documentation Accuracy:**

- **CRITICAL: Always verify documentation against actual implementation**
- README.md must show real API patterns, not fictional functions
- Use actual class names and member names from header files
- Integration examples must use real function signatures
- Keep documentation synchronized with code changes

**Code Review Checklist:**

- [ ] All public APIs have Doxygen documentation
- [ ] Const correctness applied throughout
- [ ] Constant-left comparisons used consistently
- [ ] Smart pointers used instead of raw pointers
- [ ] RAII principles applied for resource management
- [ ] Rule of Zero or Rule of Five followed correctly
- [ ] No memory leaks (verified with valgrind or similar)
- [ ] Code compiles without warnings on all platforms
- [ ] Unit tests pass
- [ ] Include guards or pragma once used correctly
- [ ] Namespaces used appropriately
- [ ] Modern C++ features used where beneficial
- [ ] Code formatted according to project standards

**Build System (CMake):**

- Use CMake 3.20+ for modern features
- Support multiple platforms (Linux, macOS, Windows)
- Support multiple compilers (GCC, Clang, MSVC)
- Generate both shared and static libraries
- Use CMake targets and properties
- Example CMakeLists.txt structure:
  ```cmake
  cmake_minimum_required(VERSION 3.20)
  project(MyProject VERSION 1.0.0 LANGUAGES CXX)

  set(CMAKE_CXX_STANDARD 23)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)

  # Library target
  add_library(mylib
      src/MyClass.cpp
      src/Helper.cpp
  )

  target_include_directories(mylib
      PUBLIC include
      PRIVATE src
  )

  # Executable target
  add_executable(myapp
      src/main.cpp
  )

  target_link_libraries(myapp PRIVATE mylib)
  ```
