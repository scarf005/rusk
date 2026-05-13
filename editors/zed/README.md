# Rusk Zed extension

Local Zed extension for `.rsk` and `.rk` files.

## Install the LSP

```sh
cargo install --path crates/rusk --bin rusk-lsp
```

`rusk-lsp` is a standard stdio Language Server Protocol server and can be used by Zed, Vim, VS Code, or any other LSP client.

## Install the Zed extension locally

1. Open Zed extensions.
2. Choose `Install Dev Extension`.
3. Select this directory: `editors/zed`.

The extension registers the `Rusk` language, starts `rusk-lsp`, and provides Tree-sitter highlighting for `.rsk` and `.rk` files.

If `rusk-lsp` is not on Zed's PATH, configure it in Zed settings:

```json
{
  "lsp": {
    "rusk-lsp": {
      "binary": {
        "path": "/absolute/path/to/rusk-lsp"
      }
    }
  }
}
```
