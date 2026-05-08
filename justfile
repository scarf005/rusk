default:
    @just --list

# Start the Rusk web demo from the repository root.
web:
    cd web && deno task dev

# Alias for `just web`.
dev: web

# Build the web demo from the repository root.
web-build:
    cd web && deno task build

# Preview the production web build from the repository root.
web-preview:
    cd web && deno task preview

# Rebuild only the WebAssembly bindings.
wasm:
    cd web && deno task wasmbuild
