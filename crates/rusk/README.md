# rusk

`rusk` is an MVP source-to-source transpiler for an indentation-based Rust dialect inspired by ML and Scala 3 layout syntax.

## Usage

```sh
rusk input.rsk -o output.rs
rusk transpile input.rsk --source-map output.map.json
rusk fmt input.rsk -o input.rsk --line-width 100
cat input.rsk | rusk
rusk run
rusk cargo test
rusk-lsp
```

## Supported MVP syntax

- indentation blocks for `struct`, `enum`, `trait`, `impl`, `mod`, `macro_rules!`, `fn`, `if`, `else`, `match`, loops, `unsafe`, and `async`
- optional opening braces on indented blocks, including inline multiline closures
- function bodies with `=`
- `if condition then expr else expr` expressions
- Rust-style inline struct literals, e.g. `Self{ id, name }`
- `do expr` for explicit semicolon/discard statements when inference is not enough
- Rust-style `#[...]` / `#![...]` attributes
- Scala-style generic brackets in type positions, e.g. `Result[T, E]` -> `Result<T, E>`
- method generic calls, e.g. `value.parse[i32]()` -> `value.parse::<i32>()`
- dotted path lowering for type paths and obvious item paths, e.g. `std.io.Read` -> `std::io::Read`, `Foo.new()` -> `Foo::new()`
- escape hatch: existing Rust `::` syntax is preserved
- macro definitions with indentation-based `macro_rules!` arms
- source formatter that preserves existing line break style; only line width is configurable
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

## Formatter

`rusk fmt` trims trailing whitespace and validates indentation without reflowing expressions, so both compact chains and already-broken chains keep their line break style. The only formatting option is `--line-width`.

```sh
rusk fmt src/main.rsk -o src/main.rsk
rusk fmt src/main.rsk --line-width 120
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

This is a compiler-front-end MVP, not a full Rust replacement yet. It intentionally avoids proc-macro DSL parsing, rust-analyzer proxying, Zed extension packaging, and full type-aware dot disambiguation.
