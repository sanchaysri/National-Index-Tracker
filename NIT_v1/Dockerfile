FROM rust
WORKDIR /app
RUN cargo install sqlx-cli --version 0.9.0 --no-default-features --features postgres,rustls
COPY . .

# Reuse downloaded crates and compiled dependencies across Watch rebuilds.
RUN --mount=type=cache,id=nit-v1-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=nit-v1-cargo-target,target=/app/target,sharing=locked \
    cargo build --locked --bin nit_v1 \
    && install -m 755 /app/target/debug/nit_v1 /usr/local/bin/nit_v1

CMD ["/usr/local/bin/nit_v1"]
