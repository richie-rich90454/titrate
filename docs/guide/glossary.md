# Glossary

This glossary defines key terms and concepts specific to the Titrate programming language.

## Core Types

### Result
A generic type `Result<T, E>` that represents either a successful value of type `T` or an error of type `E`. Results are the primary mechanism for error handling in Titrate, making error possibilities explicit in function signatures. Use `ok(value)` to create a successful result and `err(error)` to create an error result.

### Variant
A dynamic type that can hold values of any type. Use `Variant` when generics are not suitable for your use case, such as when you need to store heterogeneous values in a collection or when the type is genuinely unknown at compile time.

### Owned
A wrapper type `Owned<T>` that represents single-owner semantics for a value. When an `Owned` value is assigned to another variable, ownership transfers and the original variable can no longer be used. This enables automatic memory cleanup when the owner goes out of scope.

### Optional
A type `Optional<T>` that represents a value that may or may not be present. Unlike `null`, `Optional` makes the possibility of absence explicit in the type system, forcing callers to handle the empty case.

## Memory Management

### region
A scoped block that allocates memory with a bounded lifetime. All allocations within a region are automatically freed when the region ends. Regions are useful for batch processing, avoiding fragmentation, and predictable cleanup.

### unsafe
A block that suspends ownership and borrowing checks. Use `unsafe` for low-level operations that the compiler cannot verify, such as interfacing with C code through FFI or implementing data structures with raw pointers.

### borrow
A reference (`&T` or `&mut T`) to an owned value that allows temporary access without taking ownership. Immutable borrows (`&T`) allow reading, while mutable borrows (`&mut T`) allow reading and writing.

## Type System

### monomorphization
The compile-time process of generating specialized versions of generic code for each concrete type used in the program. This eliminates runtime overhead for generics—generic code runs at the same speed as hand-written specialized code.

### type inference
The compiler's ability to deduce the type of a variable from the value assigned to it. With `let` declarations, types are inferred automatically, reducing the need for explicit type annotations.

### algebraic data type
A type composed by combining other types through "sum" (one of several variants, like enums) or "product" (multiple fields together, like classes or tuples). Titrate's enums are sum types.

### pattern matching
The process of inspecting and destructuring a value based on its shape or type. Titrate's `switch` statements perform pattern matching on enum variants, extracting the data inside each variant.

## Syntax

### canonical form
The standard syntax for declaring functions using `fn` with `name: Type` parameter order and `: ReturnType` for the return type. For example: `fn add(a: int, b: int): int`.

### sugar form
Alternative syntax that is automatically desugared into canonical form during parsing. Titrate supports C-family syntax for familiarity during migration, but canonical form is recommended for all new code.

### import
A statement that brings types or functions from another module into the current scope. Use `import tt::module::Type;` to import a specific item, or `import tt::module::*;` for a glob import.

## Modules

### tt namespace
The standard namespace for all Titrate modules. Standard library modules live under `tt` (e.g., `tt::util::ArrayList`), and user modules also use this namespace by convention.

### public
An access modifier that makes a declaration visible outside its module. Functions, classes, and constants marked `public` can be imported from other modules.

### private
The default visibility for declarations. Items without an explicit `public` modifier are private and cannot be imported from other modules.

## Execution

### bytecode
The compiled representation of Titrate code that runs on the Titrate virtual machine. The compiler translates source code to bytecode instructions that the VM interprets.

### pipette
Titrate's build tool and package manager. Pipette manages compilation, dependencies, and execution for multi-file projects.

### trc
The Titrate compiler binary. Use `trc` to compile and run single-file programs, or use `pipette` for larger projects.

## Error Handling

### error propagation
The practice of passing errors up the call stack to a handler that can respond appropriately. Titrate's `?` operator propagates errors automatically—if a `Result` is an error, the function returns early with that error.

### throw
A statement that raises an unrecoverable error. Use `throw` for situations where continuing execution is not meaningful, such as index out of bounds or null pointer access.

### try/catch
Statements that handle thrown errors. `try` blocks contain code that might throw, and `catch` blocks receive and handle the error. Reserve this for unrecoverable errors; use `Result` for expected failures.

## Comparison with Other Languages

| Term | Titrate | Java | Rust |
|------|---------|------|------|
| Dynamic type | `Variant` | `Object` | `Box<dyn Any>` |
| Error handling | `Result<T, E>` | Exceptions | `Result<T, E>` |
| Null alternative | `Optional<T>` or `null` | `Optional<T>` | `Option<T>` |
| Memory safety | Ownership (advisory) | Garbage collection | Ownership (enforced) |
| Static methods | Top-level `fn` | `static` methods | No static, use modules |