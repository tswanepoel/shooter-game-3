#!/bin/bash

# Build script for WASM artifacts
set -e

echo "Building WASM artifacts..."

# Build the project for WASM target
cargo build --release --target wasm32-unknown-unknown

echo "WASM build completed successfully!"