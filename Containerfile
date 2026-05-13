# syntax=docker/dockerfile:1

ARG FEDORA_MINIMAL_IMAGE=registry.fedoraproject.org/fedora-minimal@sha256:ee37c006cc45336b22d0f939286ad06f0a98f9bdfcb7650be1470f2a50b8e1ca
ARG RUSTUP_TOOLCHAIN=stable
ARG RUSTUP_INIT_SHA256=6c30b75a75b28a96fd913a037c8581b580080b6ee9b8169a3c0feb1af7fe8caf
ARG DENO_VERSION=2.5.6
ARG DENO_SHA256=fd4f6abc1b6a134fa9a4dba56519f1631f44c88e04e4e3e9a8ff5975dfa66e1a

FROM ${FEDORA_MINIMAL_IMAGE} AS rust-base
ARG RUSTUP_TOOLCHAIN
ARG RUSTUP_INIT_SHA256
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
RUN microdnf --setopt=install_weak_deps=0 install -y ca-certificates curl gcc glibc-devel tini unzip \
    && microdnf clean all \
    && curl -fsSL https://sh.rustup.rs -o /tmp/rustup-init.sh \
    && echo "${RUSTUP_INIT_SHA256}  /tmp/rustup-init.sh" | sha256sum -c - \
    && sh /tmp/rustup-init.sh -y --profile minimal --default-toolchain "${RUSTUP_TOOLCHAIN}" \
    && rm /tmp/rustup-init.sh \
    && rustup target add wasm32-unknown-unknown \
    && chmod -R a+rx /usr/local/cargo /usr/local/rustup

FROM rust-base AS build
ARG DENO_VERSION
ARG DENO_SHA256
WORKDIR /app
RUN curl -fsSL "https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/deno-x86_64-unknown-linux-gnu.zip" -o /tmp/deno.zip \
    && echo "${DENO_SHA256}  /tmp/deno.zip" | sha256sum -c - \
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

FROM ${FEDORA_MINIMAL_IMAGE} AS runtime
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    RUSK_WEB_ADDR=0.0.0.0:8080 \
    RUSK_WEB_DIST=/app/web/dist \
    RUSK_RUN_TIMEOUT_MS=5000 \
    RUSK_REQUEST_TIMEOUT_MS=10000 \
    RUSK_MAX_REQUESTS=32 \
    RUSK_MAX_CONCURRENT_RUNS=2 \
    RUSK_RUN_RATE_LIMIT_PER_MINUTE=10
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
