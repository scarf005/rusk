# rusk

`rusk` is an MVP source-to-source transpiler for an indentation-based Rust dialect inspired by ML and Scala 3 layout syntax.

## Usage

```sh
rusk input.rsk -o output.rs
rusk transpile input.rsk --source-map output.map.json
cat input.rsk | rusk
rusk run
rusk cargo test
rusk-lsp
```

## Supported MVP syntax

- indentation blocks for `struct`, `enum`, `trait`, `impl`, `mod`, `fn`, `if`, `else`, `match`, loops, `unsafe`, and `async`
- function bodies with `=`
- `if condition then expr else expr` expressions
- Rust-style inline struct literals, e.g. `Self{ id, name }`
- `do expr` for explicit semicolon/discard statements when inference is not enough
- Rust-style `#[...]` / `#![...]` attributes
- Scala-style generic brackets in type positions, e.g. `Result[T, E]` -> `Result<T, E>`
- method generic calls, e.g. `value.parse[i32]()` -> `value.parse::<i32>()`
- dotted path lowering for type paths and obvious item paths, e.g. `std.io.Read` -> `std::io::Read`, `Foo.new()` -> `Foo::new()`
- escape hatch: existing Rust `::` syntax is preserved
- hierarchical JSON source map generation

## Example

```rsk
#[derive(Debug, Clone)]
pub struct User
    pub id: u64
    pub name: String

impl User
    pub fn new(id: u64, name: String) -> Self = Self{ id, name }

    pub fn display_name(&self) -> &str = &self.name

pub fn main() =
    let user = User.new(1, "Ada".to_string())
    println!("{}", user.display_name())
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
        Self{ id, name }
    }
    pub fn display_name(&self) -> &str {
        &self.name
    }
}
pub fn main() {
    let user = User::new(1, "Ada".to_string());
    println!("{}", user.display_name());
}
```

## Cargo wrapper

`rusk` can wrap common Cargo commands. It transpiles `.rsk` files under Cargo source roots (`src`, `examples`, `tests`, `benches`, and `build.rsk`) into temporary generated `.rs` files, runs Cargo, then removes the generated files.

```sh
rusk run        # cargo run
rusk check      # cargo check
rusk cargo test # cargo test
```

Existing non-generated `.rs` files are never overwritten.

## Language Server

`rusk-lsp` is a standard stdio Language Server Protocol server. It currently reports transpile diagnostics and document symbols for `.rsk` files.

```sh
cargo install --path crates/rusk --bin rusk-lsp
```

## Current limits

This is a compiler-front-end MVP, not a full Rust replacement yet. It intentionally avoids macro definitions, proc-macro DSL parsing, rust-analyzer proxying, Zed extension packaging, and full type-aware dot disambiguation.
