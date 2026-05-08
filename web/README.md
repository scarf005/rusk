# Rusk Web Demo

A Vite + Deno + Preact demo for `rusk`. The Rust transpiler is compiled to WebAssembly with [`denoland/wasmbuild`](https://github.com/denoland/wasmbuild), then called from Preact Signals for instant browser-side output.

## Running

```sh
deno task dev
```

## Build

```sh
deno task build
```

The `wasmbuild` task emits inline browser-compatible bindings to `src/wasm/` before Vite starts or builds.
