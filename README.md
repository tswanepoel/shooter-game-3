# Shooter Game

A WebGPU-powered shooter game built with Rust and Vite.

## Project Structure

```
.
├── crates/
│   ├── game-client/     # WebGPU frontend using Rust and wgpu
│   └── game-sim/        # Server-side game logic using Rust and glam
├── web/                 # Web frontend assets (HTML, CSS, JS)
├── assets/              # Game assets (models, textures)
├── infra/docker/        # Docker configuration for WASM builds
├── docs/                # Documentation
├── package.json
├── Cargo.toml
└── README.md
```

## Getting Started

### Prerequisites

- Rust (stable toolchain)
- Node.js (v16 or higher)
- wasm32-unknown-unknown target for Rust

### Local Development

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

4. Build for production:
   ```bash
   npm run build
   ```

### Running Tests

- Run Rust tests: `cargo test`
- Run Rust clippy: `cargo clippy --all-targets --all-features -- -D warnings`
- Run Rust formatting: `cargo fmt --all`

### Docker Builds

To build the WASM artifacts using Docker for reproducible builds:
```bash
docker build -f infra/docker/wasm-builder/Dockerfile -t shooter-game-wasm-builder infra/docker/wasm-builder
```

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. The hooks are automatically installed when you clone the repository. They run `cargo fmt` and `cargo clippy` before each commit. To manually run them:
```bash
git commit -m "Your commit message"  # This will trigger the pre-commit hooks automatically
```

Commit messages should follow the [Conventional Commits specification](https://www.conventionalcommits.org/), which includes proper grouping of changes into logical units. This ensures that commits represent single, coherent changes and makes the project history more maintainable.

## Development Workflow

The project uses a two-step development workflow:
1. Rust WASM compilation (handled automatically by the dev script)
2. Vite development server with hot-reload

When you run `npm run dev`, it will:
- Build the Rust WASM module
- Start the Vite development server

## Documentation

See [docs/development.md](docs/development.md) for detailed development instructions.

## Commit Message Conventions

This project follows conventional commit principles. Please ensure your commit messages follow the format: `<type>(<scope>): <description>` with allowed types including feat, fix, docs, refactor, and chore. Each commit should represent a single logical change to maintain a clean project history.