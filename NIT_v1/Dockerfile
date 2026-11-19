FROM rust
WORKDIR /app
COPY . .
RUN cargo build --locked --bin nit_v1
CMD ["/app/target/debug/nit_v1"]
