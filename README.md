# Shooter Game

A WebGPU-powered shooter game built with Rust and Vite.

## Table of Contents

- [Project Structure](#project-structure)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
- [Building for Production](#building-for-production)
- [Testing & Linting](#testing--linting)
- [Docker Builds](#docker-builds)
- [Development Workflow](#development-workflow)
- [Contributing](#contributing)

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
├── CONTRIBUTING.md      # Contribution guidelines
├── package.json
├── Cargo.toml
└── README.md
```

## Prerequisites

- Rust (stable toolchain)
- Node.js v16 or higher
- `wasm32-unknown-unknown` target for Rust
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) (builds the web WASM package under `pkg/`)
- A WebGPU-capable browser for running the game (see [Browser Support](#browser-support))

## Getting Started

1. Add the WASM target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

2. Install `wasm-pack` (if you do not already have it):
   ```bash
   cargo install wasm-pack
   ```

3. Install Node.js dependencies:
   ```bash
   npm install
   ```

4. Start the development server:
   ```bash
   npm run dev
   ```

Running `npm run dev` compiles the Rust WASM module and then starts the Vite dev server with hot-reload. See [Development Workflow](#development-workflow) for details.

### Browser Support

The game requires a browser with WebGPU support:
- Chrome 113+
- Edge 113+
- Firefox Nightly

## Building for Production

```bash
npm run build
```

## Testing & Linting

| Command | Purpose |
|---|---|
| `cargo test` | Run Rust tests |
| `cargo clippy --all-targets --all-features -- -D warnings` | Run Rust linting |
| `cargo fmt --all` | Format Rust code |
| `npm run format` | Format `web/` and root JS/TS with Prettier |

## Docker Builds

Build WASM artifacts with Docker from the **repository root** (the Dockerfile expects the full workspace as context):

```bash
docker build -f infra/docker/wasm-builder/Dockerfile -t shooter-game-wasm-builder .
```

The image packages the release `*.wasm` outputs under `/artifacts`.

## Development Workflow

`npm run dev` runs a two-step process:

1. Compiles the Rust WASM module
2. Starts the Vite development server with hot-reload

### Debug tools

One Cargo feature (`debug-tools`) gates the whole debug subsystem. npm only chooses which build uses it:

| Intent | Command | WASM |
|--------|---------|------|
| Iterate | `npm run dev` | `build-wasm` — **debug on** (default features) |
| Ship | `npm run build` | `build-wasm:release` — **debug off** (`--no-default-features`) |
| Smoke ship | `npm run build && npm run preview` | same stripped WASM as ship |

With debug on:

- Press **`` ` ``** (backtick) for the in-engine console; Esc closes it
- Commands: `help`, `grid [on|off|toggle]`, cvars such as `draw.grid`
- Host bridge: `window.__DEBUG__.exec("grid off")`

See [docs/features/003-debug-tools.md](docs/features/003-debug-tools.md).

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Setting up your local environment for contributions
- Coding standards and pre-commit hooks
- Commit message conventions (this project follows [Conventional Commits](https://www.conventionalcommits.org/))
- The pull request process
