# rusk

`rusk` is an MVP source-to-source transpiler for an indentation-based Rust dialect inspired by ML and Scala 3 layout syntax.

## Usage

```sh
rusk input.rsml -o output.rs
rusk transpile input.rsml --source-map output.map.json
cat input.rsml | rusk
```

## Supported MVP syntax

- indentation blocks for `struct`, `enum`, `trait`, `impl`, `mod`, `fn`, `if`, `else`, `match`, loops, `unsafe`, and `async`
- function bodies with `=`
- `do expr` for semicolon/discard statements
- `#derive(...)` / `#!allow(...)` attribute shorthand
- Scala-style generic brackets in type positions, e.g. `Result[T, E]` -> `Result<T, E>`
- method generic calls, e.g. `value.parse[i32]()` -> `value.parse::<i32>()`
- dotted path lowering for type paths and obvious item paths, e.g. `std.io.Read` -> `std::io::Read`, `Foo.new()` -> `Foo::new()`
- escape hatch: existing Rust `::` syntax is preserved
- line-oriented JSON source map generation

## Example

```rsml
#derive(Debug, Clone)
pub struct User
    pub id: u64
    pub name: String

impl User
    pub fn new(id: u64, name: String) -> Self =
        Self
            id = id
            name = name

    pub fn display_name(&self) -> &str = &self.name
```

Generated Rust:

```rust
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
}
impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id: id,
            name: name,
        }
    }
    pub fn display_name(&self) -> &str {
        &self.name
    }
}
```

## Current limits

This is a compiler-front-end MVP, not a full Rust replacement yet. It intentionally avoids macro definitions, proc-macro DSL parsing, rust-analyzer proxying, Zed extension packaging, and full type-aware dot disambiguation.
