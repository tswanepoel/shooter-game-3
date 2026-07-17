# Multi-stage Docker build for reproducible WASM builds
FROM rust:1.70 as builder

# Install wasm32-unknown-unknown target
RUN rustup target add wasm32-unknown-unknown

# Install additional build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    libssl-dev \
    libvulkan-dev \
    vulkan-tools \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY . .

# Build the project for WASM target
RUN cargo build --release --target wasm32-unknown-unknown

# Create final image with just the necessary files
FROM alpine:latest

RUN apk --no-cache add ca-certificates
WORKDIR /root/

# Copy the built WASM files
COPY --from=builder /app/target/wasm32-unknown-unknown/release/*.wasm .

# Copy build script
COPY infra/docker/wasm-builder/build.sh .

RUN chmod +x ./build.sh

CMD ["./build.sh"]