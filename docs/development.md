# Development Guide

This document provides detailed instructions for developing the shooter game.

## Project Overview

The shooter game is structured as a Rust workspace with two main crates:
- `game-sim`: Contains server-side game logic and simulation
- `game-client`: Contains WebGPU rendering and client-side logic

## Setting Up the Development Environment

### Prerequisites

- Rust (stable toolchain)
- Node.js (v16 or higher)
- wasm32-unknown-unknown target for Rust

### Installation

1. Install Rust dependencies:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Install Node.js dependencies:
   ```bash
   npm install
   ```

3. Run the development server:
   ```bash
   npm run dev
   ```

## Building for Production

To build the project for production:
```bash
npm run build
```

## Running Tests

### Rust Tests
```bash
cargo test
```

### Rust Linting
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Rust Formatting
```bash
cargo fmt --all
```

## Docker Builds

To build the WASM artifacts using Docker for reproducible builds:
```bash
docker build -f infra/docker/wasm-builder/Dockerfile -t shooter-game-wasm-builder infra/docker/wasm-builder
```

## Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. The hooks are automatically installed when you clone the repository. They run `cargo fmt` and `cargo clippy` before each commit.

To manually run them:
```bash
git commit -m "Your commit message"  # This will trigger the pre-commit hooks automatically
```

## Development Workflow

1. Make changes to the code
2. Run tests to ensure nothing is broken
3. Format your Rust code with `cargo fmt`
4. Run clippy with `cargo clippy`
5. Commit your changes following conventional commit principles (pre-commit hooks will run automatically)
   - Each commit should represent a single logical change
   - Follow the format: `<type>(<scope>): <description>`
   - Allowed types: feat, fix, docs, refactor, chore
   - Keep the first line under 72 characters

## Code Structure

### game-sim Crate
- Contains game logic and state management
- Uses glam for mathematical operations
- Uses serde for serialization

### game-client Crate
- Contains WebGPU rendering logic
- Uses wgpu for graphics rendering
- Uses wasm-bindgen for JavaScript interop

### Web Assets
- `web/index.html`: Main HTML file with canvas element
- `web/style.css`: Basic styling for the game canvas
- `web/app.js`: WASM module loader and application initializer

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and formatting
5. Submit a pull request

## Troubleshooting

### WebGPU Not Supported
If you encounter WebGPU not supported errors, make sure you're using a browser that supports WebGPU (Chrome 113+, Edge 113+, Firefox Nightly).

### WASM Build Issues
If you have issues with WASM builds, ensure you have the correct Rust target installed:
```bash
rustup target add wasm32-unknown-unknown
```

### Development Server Issues
If the development server fails to start, try:
```bash
npm install
npm run dev