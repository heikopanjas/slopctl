
## C Coding Conventions

**General Principles:**

- Follow C17 standard for broad compiler compatibility (including MSVC)
- Write self-documenting code with clear naming and structure
- Apply const-correctness throughout the codebase
- Use defensive programming with parameter validation
- Keep functions focused and modular
- Ensure platform portability (Linux, macOS, Windows)
- Prefer security over convenience in API design
- Write code that compiles with strict warnings enabled

**C Standard and Compatibility:**

- Use **C17 standard** exclusively for maximum compiler compatibility
- Avoid C23-specific features (not yet supported by MSVC)
- Do not use C++ code or C++-specific features
- Avoid platform-specific system calls when possible
- Test on all target platforms regularly (Linux, macOS, Windows)
- Use standard C library functions only
- Handle platform differences through preprocessor directives when necessary

**Const Correctness:**

- All input parameters should be `const` when not modified
- Apply `const` to pointer targets, not just pointers: `const char*` not `char* const`
- Use `const` to document intent and prevent accidental modification
- Examples:
  - ✅ Correct: `KString KStringCreate(const char* pStr, const size_t Size);`
  - ✅ Correct: `int KStringCompare(const KString a, const KString b);`
  - ❌ Incorrect: `KString KStringCreate(char* pStr, size_t Size);`
- Const correctness improves maintainability and compiler optimization

**Comparison Conventions:**

- **Always place constants on the left side of comparisons** (constant-left style)
- This prevents accidental assignment when `=` is used instead of `==`
- Examples:
  - ✅ Correct: `if (NULL == ptr)`, `if (0 == value)`, `if (true == condition)`
  - ❌ Incorrect: `if (ptr == NULL)`, `if (value == 0)`, `if (condition == true)`
- Apply to all comparisons including pointer checks, numeric values, and booleans
- Benefits: Compiler error if `=` is mistakenly used instead of `==`

**Parameter Naming Conventions:**

- **`Size`**: Count of bytes (for byte array parameters)
- **`cchSize`**: Count of characters (for character array parameters when distinct from bytes)
- **Pointer parameters**: PascalCase with `p` prefix (e.g., `pStr`, `pData`, `pBuffer`)
- **Value parameters**: PascalCase (e.g., `Encoding`, `Length`, `Index`)
- Use descriptive names that indicate purpose and units
- Be explicit about what size represents (bytes vs characters vs elements)

**Secure API Design:**

- **Require explicit length parameters** for all functions accepting `char*` pointers
- Never rely on null-terminated strings alone (avoid `strlen()` in library code)
- Provide explicit size to prevent buffer overflows
- Example secure API:
  ```c
  // Secure: Requires explicit size
  KString KStringCreate(const char* pStr, const size_t Size);

  // Less secure: Uses strlen() internally (provide for convenience only)
  KString KStringCreateFromCStr(const char* pStr);
  ```
- Validate all size parameters before use
- Check for arithmetic overflow in size calculations
- Use `size_t` for all size-related parameters and return values

**Function Naming:**

- Use prefix for all public API functions (e.g., `KString` prefix)
- Use PascalCase for public functions: `KStringCreate`, `KStringCompare`
- Use prefix + underscore for private functions: `KS_ValidatePointer`, `KS_Release`
- Action verbs should be clear and descriptive
- Common patterns:
  - Create/Destroy for resource management
  - Get/Set for property access
  - Convert for type transformations
  - Validate for checks

**Variable Naming:**

- **Local variables**: PascalCase (e.g., `MyVariable`, `StringLength`, `BufferSize`)
- **Function parameters**: PascalCase (e.g., `InputString`, `MaxLength`)
- **Pointer variables**: PascalCase with `p` prefix (e.g., `pData`, `pBuffer`, `pString`)
- **Type names**: PascalCase (e.g., `KString`, `KStringEncoding`)
- **Enum constants**: UPPER_SNAKE_CASE with prefix (e.g., `KSTRING_ENCODING_UTF8`)
- **Macro definitions**: UPPER_SNAKE_CASE with prefix (e.g., `KSTRING_MAX_SHORT_LENGTH`)
- **Static functions**: Prefix with project abbreviation (e.g., `KS_` for KString internals)

**Type Definitions:**

- Use `typedef` for struct types to avoid `struct` keyword in declarations
- Opaque types: Only define typedef in header, full struct in implementation
- Example:
  ```c
  // In header (opaque handle)
  typedef struct KString KString;

  // In implementation file
  struct KString {
      uint32_t size;
      // ... implementation details
  };
  ```
- Use descriptive type names in PascalCase
- Define enums with explicit values when they represent protocol/format specifications

**Enums:**

- Use explicit values for enums that map to external specifications
- Prefix enum constants with type name in UPPER_SNAKE_CASE
- Example:
  ```c
  typedef enum {
      KSTRING_ENCODING_UTF8     = 0,  // Default UTF-8 encoding
      KSTRING_ENCODING_UTF16LE  = 1,  // UTF-16 Little Endian
      KSTRING_ENCODING_UTF16BE  = 2,  // UTF-16 Big Endian
      KSTRING_ENCODING_ANSI     = 3   // ANSI/Windows-1252 (legacy)
  } KStringEncoding;
  ```
- Add comments for each enum value explaining its purpose
- Use `typedef enum` to avoid `enum` keyword in declarations

**Memory Management:**

- Use `calloc()` for all dynamic allocations (zero-initialization)
- Never use `malloc()` - always prefer `calloc()` for safety
- Check all allocation results for NULL before use
- Document ownership transfer clearly in function comments
- Functions that return pointers transfer ownership (caller must free)
- Functions that take `const` pointers do not take ownership
- Provide cleanup functions for resource types (e.g., `KStringDestroy`)
- Example:
  ```c
  // Allocate with zero-initialization
  char* pBuffer = (char*)calloc(Size, sizeof(char));
  if (NULL == pBuffer) {
      return InvalidResult();  // Handle allocation failure
  }
  ```

**Error Handling:**

- Return error indicators that can't be confused with valid values
- Use sentinel values for errors (e.g., `UINT32_MAX` for invalid size)
- Document error conditions clearly in function comments
- Use defensive programming: validate all parameters
- Check for NULL pointers before dereferencing
- Check for arithmetic overflow before operations
- Example:
  ```c
  // Validate input parameters
  if (NULL == pStr || 0 == Size) {
      return InvalidKString();  // Return sentinel value
  }

  // Check for overflow
  if (Size > MAX_VALID_SIZE) {
      return InvalidKString();
  }
  ```
- No exceptions - use return values for error reporting

**Function Structure:**

- Keep functions short and focused on single responsibility
- Use early returns to reduce nesting depth
- Validate parameters at function start
- Group related operations logically
- Example structure:
  ```c
  KString KStringCreate(const char* pStr, const size_t Size)
  {
      // 1. Validate parameters
      if (NULL == pStr || 0 == Size) {
          return InvalidKString();
      }

      // 2. Handle short string case
      if (Size <= KSTRING_MAX_SHORT_LENGTH) {
          return CreateShortString(pStr, Size);
      }

      // 3. Handle long string case
      return CreateLongString(pStr, Size);
  }
  ```

**Header Organization:**

- Include guards using `#ifndef`/`#define`/`#endif`
- Order: includes, macros, types, function declarations
- Example:
  ```c
  #ifndef KSTRING_H
  #define KSTRING_H

  #include <stddef.h>
  #include <stdint.h>
  #include <stdbool.h>

  // Macros and constants
  #define KSTRING_MAX_SHORT_LENGTH 12

  // Type definitions
  typedef struct KString KString;
  typedef enum { /* ... */ } KStringEncoding;

  // Public API declarations
  KString KStringCreate(const char* pStr, const size_t Size);
  void KStringDestroy(const KString kstr);

  #endif // KSTRING_H
  ```

**Implementation File Organization:**

- Order: includes, private macros, private types, private functions, public functions
- Group related functions together
- Use static inline for performance-critical helpers
- Example:
  ```c
  #include "KString.h"

  // Private macros
  #define KSTRING_SIZE_MASK 0x3FFFFFFF

  // Private helper functions
  static inline size_t GetSizeFromField(uint32_t SizeField) {
      return SizeField & KSTRING_SIZE_MASK;
  }

  // Public API implementations
  KString KStringCreate(const char* pStr, const size_t Size) {
      // ... implementation
  }
  ```

**Comments:**

- Use `//` for all comments (single-line and multi-line)
- Comment the "why" not the "what"
- Document complex algorithms and optimizations
- Add comments for bit manipulation and non-obvious logic
- Example:
  ```c
  // Extract 30-bit size from combined size/encoding field
  // Upper 2 bits store encoding, lower 30 bits store size
  static inline size_t GetSizeFromField(uint32_t SizeField)
  {
      return SizeField & KSTRING_SIZE_MASK;
  }
  ```
- Keep comments concise and focused
- Update comments when code changes

**Code Formatting:**

- Use `.clang-format` configuration for automatic formatting
- Indentation: 4 spaces (no tabs)
- Braces: Opening brace on next line for functions and blocks
- Example:
  ```c
  // Function: opening brace on next line
  KString KStringCreate(const char* pStr, const size_t Size)
  {
      // Control structure: opening brace on next line
      if (NULL == pStr)
      {
          return InvalidKString();
      }

      for (size_t i = 0; i < Size; i++)
      {
          // Process character
      }

      return Result;
  }
  ```
- Line length: Keep under 120 characters when practical
- Align related declarations for readability

**Bit Manipulation:**

- Use descriptive macro names for bit masks and shifts
- Document bit field layouts clearly
- Use helper functions for extracting/combining bit fields
- Example:
  ```c
  // Size field layout: 30 bits size + 2 bits encoding (32 bits total)
  #define KSTRING_SIZE_MASK         0x3FFFFFFF  // Lower 30 bits
  #define KSTRING_ENCODING_MASK     0xC0000000  // Upper 2 bits
  #define KSTRING_ENCODING_SHIFT    30

  // Extract components
  static inline size_t GetSizeFromField(uint32_t SizeField) {
      return SizeField & KSTRING_SIZE_MASK;
  }

  static inline KStringEncoding GetEncodingFromField(uint32_t SizeField) {
      return (KStringEncoding)((SizeField & KSTRING_ENCODING_MASK) >> KSTRING_ENCODING_SHIFT);
  }
  ```
- Use `static_assert` to verify size assumptions at compile time

**Platform Portability:**

- Use standard types from `<stdint.h>`: `uint32_t`, `uint64_t`, `size_t`
- Use `<stdbool.h>` for bool type instead of custom definitions
- Handle endianness differences when needed
- Use preprocessor for platform-specific code:
  ```c
  #ifdef _WIN32
      // Windows-specific code
  #else
      // Unix/POSIX code
  #endif
  ```
- Test on Linux, macOS, and Windows regularly
- Use CMake for cross-platform build configuration

**Compiler Warnings:**

- Build with strict warnings enabled:
  - GCC/Clang: `-Wall -Wextra -Wpedantic`
  - MSVC: `/W4`
- Treat warnings as errors in development builds
- Fix all warnings - don't suppress them unless absolutely necessary
- Document any warning suppressions with reasoning

**Static Assertions:**

- Use `static_assert` to verify compile-time assumptions
- Example:
  ```c
  #include <assert.h>

  // Verify structure size matches specification
  static_assert(sizeof(KString) == 16, "KString must be exactly 16 bytes");

  // Verify bit field sizes
  static_assert(sizeof(uint32_t) * 8 >= 32, "uint32_t must be at least 32 bits");
  ```
- Check sizes, alignments, and enum value ranges

**Inline Functions:**

- Use `static inline` for small, performance-critical helpers
- Define inline functions in implementation file, not header (unless needed by multiple files)
- Keep inline functions simple (1-3 lines typical)
- Example:
  ```c
  static inline bool KS_IsShortString(KString kstr)
  {
      return GetSizeFromField(kstr.size) <= KSTRING_MAX_SHORT_LENGTH;
  }
  ```

**Performance Considerations:**

- Pass small structs by value (≤16 bytes for register passing)
- Use `const` to enable compiler optimizations
- Avoid unnecessary pointer indirection
- Use inline functions for hot paths
- Consider cache locality in data structure design
- Document performance-critical sections
- Example:
  ```c
  // Pass by value for 16-byte struct (fits in registers)
  int KStringCompare(const KString a, const KString b)
  {
      // Fast path: compare sizes first
      size_t SizeA = GetSizeFromField(a.size);
      size_t SizeB = GetSizeFromField(b.size);

      if (SizeA != SizeB) {
          return (SizeA > SizeB) ? 1 : -1;
      }

      // ... rest of comparison
  }
  ```

**Testing Strategy:**

- Write test programs in separate `_examples/` directory
- Test all public API functions
- Include edge cases: NULL pointers, zero sizes, maximum sizes
- Test on all target platforms
- Use assertion macros for test validation
- Document expected behavior in test code

**Documentation:**

- Document all public API functions in header file
- Include purpose, parameters, return value, and notes
- Example:
  ```c
  /**
   * Creates a new KString from a byte array with explicit size.
   *
   * @param pStr Pointer to character data (not necessarily null-terminated)
   * @param Size Number of bytes in the string
   * @return New KString instance, or invalid KString on allocation failure
   *
   * @note Caller is responsible for calling KStringDestroy() when done
   * @note Uses UTF-8 encoding by default
   * @note Size must not exceed maximum supported size (2^30 - 1 bytes)
   */
  KString KStringCreate(const char* pStr, const size_t Size);
  ```
- Keep documentation concise but complete
- Update documentation when API changes

**Code Review Checklist:**

- [ ] All public functions have documentation comments
- [ ] Const correctness applied throughout
- [ ] Constant-left comparisons used consistently
- [ ] All size parameters use `size_t` type
- [ ] NULL pointer checks before all pointer dereferences
- [ ] Arithmetic overflow checks for size calculations
- [ ] Memory allocated with `calloc()`, checked for NULL
- [ ] All allocations have corresponding cleanup path
- [ ] Code compiles without warnings on all platforms
- [ ] Static assertions verify compile-time assumptions
- [ ] Function names follow naming conventions
- [ ] Comments explain "why" not "what"
- [ ] Code formatted according to `.clang-format`

**Build System (CMake):**

- Use CMake 3.30+ for modern features
- Support multiple platforms (Linux, macOS, Windows)
- Generate both shared and static libraries
- Example targets:
  - Linux: `libkstring.so`, `libkstring.a`
  - macOS: `libkstring.dylib`, `libkstring.a`
  - Windows: `kstring.dll`, `kstring.lib`
- Use Ninja generator for fast parallel builds
- Separate examples into `_examples/` subdirectory
- Build artifacts in `_build/` directory (gitignored)
