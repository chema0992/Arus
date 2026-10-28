# Arus

<img width="1280" height="640" alt="logo2" src="https://github.com/user-attachments/assets/6cfaa956-3daf-484c-80a3-35a3a3223af1" />

> **Easy to write. Powerful performance.**

Arus is a systems programming language currently under design, aiming for readable and writable syntax while delivering native-code performance. Its core design elements are a proprietary intermediate representation called **AIR (Arus Intermediate Representation)** and a permission-based pointer model.

> [!WARNING]
> **Arus is currently in the design and development phase.** The syntax and command examples below illustrate the current design direction and intended implementation. Implementation status and detailed rules may change. A stable 1.0 release is not yet assumed.

## Design Goals

- **Easy to write:** Minimize unnecessary syntactic noise and prioritize readable code.
- **Native compilation:** The ultimate goal is native machine-code generation via LLVM.
- **Explicit memory permissions:** Express read/write permissions of pointers in the type system and check invalid accesses at compile time.
- **Independent intermediate representation:** Keep AIR as a clear boundary between the frontend and backend.
- **Gradual self-hosting:** Bootstrap the initial compiler in Rust, then rewrite the compiler in Arus once the language is sufficiently mature.

## Compilation Pipeline

```text
Arus Source
↓
Lexer / Parser
↓
AST
↓
AIR
│ Type and permission information
│ Memory-access and aliasing checks
↓
LLVM IR
↓
LLVM Backend
↓
Native Machine Code
```

Roles of each stage:

- **AST:** Represents the syntactic structure of the source code. Type and permission information must be preservable for later analysis stages.
- **AIR:** Expresses the program with simpler and more explicit operations than the AST. Performs language-level checks of Arus type and memory-permission rules before generating LLVM IR.
- **LLVM IR / Backend:** Converts verified AIR into LLVM IR, then uses LLVM for optimization and native code generation.

The purpose of AIR is not to replicate LLVM IR. It provides an intermediate representation that is easy to parse and analyze while preserving Arus semantics and permission information.

### AIR Syntax Direction

To reduce parsing cost and implementation complexity, AIR currently prioritizes a simple text format in which each instruction appears on its own line and operands are separated by spaces.

```text
%1 = add 100 60
%2 = mul %1 2
%3 = sub %2 10
return %3
```

The code above is only an illustrative example of the design direction. The final instruction set, type notation, block/function syntax, and AIR file format have not yet been finalized.

## Execution Modes

Two execution modes are planned:

```
| Mode      | Behavior                        | Primary Purpose              |
| --------- | ------------------------------- | ---------------------------- |
| `runtime` | Immediate execution via interpreter | Fast testing and development |
| `compile` | Compile then run native code    | Default execution and deployment |
```

Compile mode is intended to be the default; runtime mode is provided as a convenience for development. The CLI examples below represent the target interface and do not guarantee current implementation.

```sh
arus run test.arus
arus build main.arus
```

## Program Structure

Arus sources are designed to explicitly declare one of two program forms.

### `script`

Top-level statements in the file are executed in order.

```arus
script

println("Hello, Arus!")
let value: i32 = 10
println(value)
```

### `main`

Programs that use an explicit entry-point function `main()`.

```arus
main

fn main() {
println("Hello, Arus!")
}
```

`script` and `main` represent distinct program structures. Detailed validity rules will be finalized during development.

## Syntax Preview

The following examples summarize the current direction of the language syntax.

### Variables and Types

```arus
let count: i32 = 10
mut total: i32 = 20

let inferred = 30
mut updated = 40
```

- `let` denotes an immutable binding; `mut` denotes a mutable binding.
- Omitting the type enables type inference.
- Semicolons are not required in the base syntax.

### Functions and Return

```arus
fn add(a: i32, b: i32) -> i32 {
return a + b
}
```

Returns are explicit via `return`. Implicit return of the last expression is not planned.

### Conditionals and Loops

```arus
if count > 10 {
println("large")
} else if count > 5 {
println("medium")
} else {
println("small")
}

for i in 0..10 {
println(i)
}
```

Parentheses around conditions are not required; the condition must evaluate to a boolean. Details such as whether range endpoints are inclusive remain to be finalized.

### Strings and Interpolation

```arus
print("value = {count}")
print(f"value = {count}")
```

In ordinary strings `{count}` is treated literally; interpolation occurs only inside `f"..."` strings.

### Arrays

```arus
let fixed: [i32; 3] = [10, 20, 30]
mut values: [i32] = [1, 2, 3]

values.push(4)
println(values[0])
```

`[T; N]` denotes a fixed-size array; `[T]` denotes a dynamic array. Full array behavior and type-inference rules are still under design.

### Structs and Enums

```arus
struct User {
name: str
age: i32
}

let user = User {
name: "Tom"
age: 20
}

enum Color {
Red
Green
Blue
}
```

Preference is currently given to `struct` over inheritance-oriented `class`, and to ordinary enums over class-style enums.

### `match`

```arus
match color {
Red => println("red")
Green => println("green")
Blue => println("blue")
}
```

A syntax using branch patterns and the arrow `=>` is under consideration. Exhaustiveness checking and return-value rules are not yet decided.

## Permission-Based Pointers

One of Arus’s core design decisions is to make read/write permissions explicit in pointer types.

```
| Notation | Meaning              |
| -------- | -------------------- |
| `T r*`   | Read permission      |
| `T w*`   | Write permission     |
| `T rw*`  | Read and write permission |
```

Example of a read-only pointer:

```arus
let value: i32 = 5
let pointer: i32 r* = &value

let copy = *pointer
println(copy)
```

Writes through an `r*` pointer are disallowed:

```arus
*pointer = 10 // compile error: write through read-only pointer
```

Mutability of a `let` or `mut` binding is independent of pointer permissions. The default inferred permission for pointers is currently planned to be read (`r`). Pointer creation conditions, aliasing restrictions, lifetimes, and deallocation rules require further detailed design.

## Standard Input / Output

Standard I/O is intended to be available as built-ins without explicit imports, similar to Rust and Python.

```arus

print("No newline")
println("With newline")

let text = input("Input: ")
let number = i32(input("Number: "))
```

- `print()` does not append a newline.
- `println()` appends a newline after output.
- `input()` returns a `str`.
- Type-name conversion syntax such as `i32(value)` is preferred. Concrete handling of conversion failures is still undecided.

## Development Roadmap

1. Finalize basic syntax and type rules of Arus
2. Design AST and AIR data structures together with the text format
3. Implement AIR type, permission, and memory analyzers
4. Convert AIR to LLVM IR and generate native code
5. Build a runtime interpreter and developer tooling
6. Pursue self-hosting once the language stabilizes

## Project Status

Arus is still in an experimental design phase. The syntax and commands appearing in this document are not a final specification or a complete feature list. As implementation progresses, examples, build instructions, testing methods, and contribution guidelines will be added.

## License

See the LICENSE document.
```