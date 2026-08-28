FROM rust:1.96-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY services/ services/
RUN cargo build --release --bin catalog-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/catalog-service /usr/local/bin/
EXPOSE 8000
CMD ["catalog-service"]
