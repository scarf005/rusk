# syntax=docker/dockerfile:1

ARG FEDORA_MINIMAL_VERSION=42
ARG RUSTUP_TOOLCHAIN=stable
ARG DENO_VERSION=2.5.6

FROM registry.fedoraproject.org/fedora-minimal:${FEDORA_MINIMAL_VERSION} AS rust-base
ARG RUSTUP_TOOLCHAIN
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN microdnf --setopt=install_weak_deps=0 install -y ca-certificates curl gcc glibc-devel tini unzip \
    && microdnf clean all \
    && curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "${RUSTUP_TOOLCHAIN}" \
    && rustup target add wasm32-unknown-unknown \
    && chmod -R a+rx /usr/local/cargo /usr/local/rustup

FROM rust-base AS build
ARG DENO_VERSION
WORKDIR /app
RUN curl -fsSL "https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/deno-x86_64-unknown-linux-gnu.zip" -o /tmp/deno.zip \
    && unzip /tmp/deno.zip -d /usr/local/bin \
    && rm /tmp/deno.zip
COPY . .
RUN cargo build --release -p rusk --bin rusk --bin rusk-web
RUN cd web && VITE_RUN_API=1 deno task build

FROM rust-base AS runtime-rust
RUN rm -rf \
    /usr/local/cargo/registry \
    /usr/local/rustup/downloads \
    /usr/local/rustup/tmp \
    /usr/local/rustup/toolchains/*/lib/rustlib/wasm32-unknown-unknown \
    /usr/local/rustup/toolchains/*/share/doc \
    /usr/local/rustup/toolchains/*/share/man

FROM registry.fedoraproject.org/fedora-minimal:${FEDORA_MINIMAL_VERSION} AS runtime
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    RUSK_WEB_ADDR=0.0.0.0:8080 \
    RUSK_WEB_DIST=/app/web/dist \
    RUSK_RUN_TIMEOUT_MS=5000 \
    RUSK_REQUEST_TIMEOUT_MS=10000 \
    RUSK_MAX_CONNECTIONS=32 \
    RUSK_MAX_CONCURRENT_RUNS=2
RUN microdnf --setopt=install_weak_deps=0 install -y ca-certificates gcc glibc-devel tini \
    && microdnf clean all
COPY --from=runtime-rust /usr/local/cargo /usr/local/cargo
COPY --from=runtime-rust /usr/local/rustup /usr/local/rustup
WORKDIR /app
COPY --from=build /app/target/release/rusk /usr/local/bin/rusk
COPY --from=build /app/target/release/rusk-web /usr/local/bin/rusk-web
COPY --from=build /app/web/dist /app/web/dist
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/rusk-web"]
