#!/bin/bash
# Build WASM artifacts from a mounted workspace (used inside the builder image).
set -euo pipefail

echo "Building WASM artifacts..."
cargo build --release --target wasm32-unknown-unknown --locked
echo "WASM build completed successfully!"
