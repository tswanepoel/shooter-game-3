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
├── web/                 # Web frontend source (HTML, CSS, JS)
│   └── dist/            # Sole ship tree (Vite build + cooked packs)
├── assets/
│   ├── source/          # Authoring kits (not served)
│   └── cooked/          # Cook outputs (Vite publicDir → lands in web/dist)
├── tools/               # Build helpers (e.g. asset cook)
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

Running `npm run dev` cooks art packs, compiles the Rust WASM module, then starts the Vite dev server with hot-reload. See [Development Workflow](#development-workflow) for details.

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

`npm run dev` / `npm run build` run:

1. **Cook** art — `npm run cook` gathers `assets/source/` into hashed packs under `assets/cooked/` (manifest + packs). Skips when sources unchanged.
2. Compiles the Rust WASM module (`wasm-pack`)
3. Starts Vite (dev) or emits the production bundle under `web/dist/` (build)

Vite serves cook outputs from `assets/cooked/` (not authoring kits). Production copies them into `web/dist/` alongside Vite’s hashed `/assets/*` chunks. JS/WASM stay their own bundle; art is not embedded in the Vite JS graph. Kit facts live next to the source kits (e.g. `assets/source/characters/README.md`).

### Input session

**Click the canvas once** to enter the in-game input session (browser pointer lock). That owns look and game keys until the browser ejects (Esc, blur, leave tab). Click again to resume. Game modes (including flycam) do not lock or unlock the pointer.

### Player self

The default view mounts on the player **self**: a standing Kenney body with a held blaster (`character-a` + `blaster-a` until loadout exists). The camera sits at a local **eye offset** from the feet origin and looks along self facing. Production and dev both load this path; kit facts stay in the source kit READMEs.

### Debug tools

With `npm run dev`, press **`` ` ``** (backtick) for the in-game developer console. Production builds (`npm run build`) omit debug tools.

| Input / command | Action |
|---|---|
| **Click canvas** | Enter / resume input session |
| **`` ` ``** | Toggle developer console |
| **F9** / `screenshot` | Capture frame to `debug/shots/` |
| **F8** / `flycam` / `remount` | Debug flycam (WASD + mouse look, Q/E up/down, Shift sprint); remount restores the self eye view |
| `lineup` / `draw.lineup` | Toggle blaster lineup (held Kenney row: scale, paint, grip, muzzle markers) |

## Contributing

Contributions are welcome. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Setting up your local environment for contributions
- Coding standards and pre-commit hooks
- Commit message conventions (this project follows [Conventional Commits](https://www.conventionalcommits.org/))
- The pull request process
