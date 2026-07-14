# Feature 001 - Scaffolding Implementation Checklist

## 1. Project Initialization
- [ ] Create root `Cargo.toml` with workspace members (`game-sim`, `game-client`)
- [ ] Create root `package.json` with Vite scripts and dependencies
- [ ] Create root `.gitignore` (Rust, Node, IDE files)
- [ ] Create root `README.md` with setup instructions

## 2. Game Sim Crate (Server-side Logic)
- [ ] Create `crates/game-sim/` directory structure
- [ ] Add `Cargo.toml` with dependencies: `glam`, `serde` (optional)
- [ ] Implement basic game logic structures in `src/lib.rs`
- [ ] Write unit tests in `tests/`
- [ ] Ensure crate compiles for standard Rust target

## 3. Game Client Crate (WebGPU Frontend)
- [ ] Create `crates/game-client/` directory structure
- [ ] Add `Cargo.toml` with dependencies: `wgpu`, `wasm-bindgen`, `js-sys`, `web-sys`
- [ ] Implement WebGPU initialization in `src/main.rs`
- [ ] Set up canvas context connected to DOM element
- [ ] Implement render loop with blank screen clear
- [ ] Add `console_error_panic_hook` for debugging

## 4. Web Frontend (Using `web/` directory)
- [ ] Create `web/index.html` with canvas element
- [ ] Create `web/style.css` for basic styling
- [ ] Create `web/app.js` to load WASM module and initialize application
- [ ] Configure `vite.config.ts` to handle WASM imports and dev server
- [ ] Ensure Vite development server runs with hot-reload

## 5. Build System & Tooling
- [ ] Create `infra/docker/wasm-builder/Dockerfile` for reproducible WASM builds
- [ ] Create `infra/docker/wasm-builder/build.sh` script
- [ ] Add `.dockerignore` files where appropriate
- [ ] Configure pre-commit hooks (via `lint-staged` or similar) for:
    - [ ] `cargo fmt --check`
    - [ ] `cargo clippy`

## 6. CI/CD Pipeline
- [ ] Create `.github/workflows/ci.yml` with jobs for:
    - [ ] Install Rust toolchain with `wasm32-unknown-unknown` target
    - [ ] Run `cargo fmt --check`
    - [ ] Run `cargo clippy`
    - [ ] Run `cargo test` (for both workspace crates)
    - [ ] Build WASM module (using Docker or native runner)

## 7. Documentation
- [ ] Update `README.md` with:
    - [ ] Local development instructions (`npm install`, `npm run dev`)
    - [ ] Docker build instructions
    - [ ] Project structure explanation
- [ ] Create `docs/development.md` with:
    - [ ] Detailed setup guide
    - [ ] Testing instructions
    - [ ] Building WASM artifacts guide
    - [ ] Contribution guidelines (pre-commit hooks, CI)

## Acceptance Criteria Verification
- [ ] Docker builds WASM artifacts successfully
- [ ] Workspace has separate crates for client and sim
- [ ] WebGPU initializes and renders a blank canvas in browser
- [ ] Sim logic uses `glam` and is separated from render logic
- [ ] Local dev server supports hot-reload via Vite
- [ ] Pre-commit hooks enforce formatting/linting
- [ ] CI runs tests and checks WASM build
- [ ] Documentation reflects the setup