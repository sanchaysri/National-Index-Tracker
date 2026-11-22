FROM rust
WORKDIR /app
RUN cargo install sqlx-cli --version 0.9.0 --no-default-features --features postgres,rustls
COPY . .
RUN cargo build --locked --bin nit_v1
CMD ["/app/target/debug/nit_v1"]
