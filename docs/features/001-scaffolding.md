# Feature 001 - Scaffolding

The game will be built to run in the browser using WebGPU and Rust.

## Acceptance Criteria
- The solution is containerised. Use Docker for building WASM artifacts reproducibly.
- The app is created with a modular structure. Use separate Rust crates.
- Render logic runs inside the WASM module using `wgpu` and drives its own render loop.
- Sim logic is packaged separately so it can be reused server-side; use `glam` for math.
- Keep env-specific configuration separate from deployables.
- Running locally is a simple command that supports hot-reload. Use Vite.
- Visiting the game page successfully initializes WGPU and renders a blank canvas.
- Pre-commit hooks enforce formatting (`cargo fmt`) and linting (`clippy`).
- CI runs `cargo test` (unit tests) and checks WASM build compatibility.
- Relevant documentation is kept up to date with code changes.
