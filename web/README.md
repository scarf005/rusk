# Rusk Web Demo

A Vite + Deno + Preact demo for `rusk` and `ruk`. The Rust/Ruk/Rusk converters are compiled to WebAssembly with [`denoland/wasmbuild`](https://github.com/denoland/wasmbuild), then called from Preact Signals for instant browser-side output.

## Running

```sh
deno task dev
```

## Build

```sh
deno task build
```

The `wasmbuild` task emits inline browser-compatible bindings to `src/wasm/` before Vite starts or builds.

## Self-host with Podman

```sh
podman compose up --build
```

The image build uses Deno to produce static files, then serves them with `rusk-web`; the runtime image contains `rustc` and `rusk`, but no Deno binary is needed.
