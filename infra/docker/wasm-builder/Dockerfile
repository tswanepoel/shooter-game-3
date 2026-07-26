# Reproducible WASM build. Must be built from the repository root:
#
#   docker build -f infra/docker/wasm-builder/Dockerfile -t shooter-game-wasm-builder .
#
FROM rust:1-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --target wasm32-unknown-unknown --locked -p game-client -p game-sim -p game-net

# Slim image containing only the built WASM artifacts.
FROM alpine:3.20

RUN apk --no-cache add ca-certificates

WORKDIR /artifacts

COPY --from=builder /app/target/wasm32-unknown-unknown/release/*.wasm ./

CMD ["ls", "-la", "/artifacts"]
