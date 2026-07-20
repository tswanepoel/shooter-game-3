# Contributing to Shooter Game

Thanks for your interest in contributing! This guide covers everything you need to know to set up your environment, make changes, and submit a pull request.

## Table of Contents

- [Project Overview](#project-overview)
- [Environment Setup](#environment-setup)
- [Code Structure](#code-structure)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Message Conventions](#commit-message-conventions)
- [Pre-commit Hooks](#pre-commit-hooks)
- [Pull Request Process](#pull-request-process)
- [Troubleshooting](#troubleshooting)

## Project Overview

The shooter game is structured as a Rust workspace:

- **`game-sim`** — pure simulation rules
- **`game-net`** — multiplayer wire protocol placeholder
- **`game-server`** — native multiplayer host placeholder
- **`game-client`** — WebGPU client (solo)

## Environment Setup

Follow the [Getting Started](README.md#getting-started) instructions in the README to install prerequisites and run the project locally. In short:

```bash
rustup target add wasm32-unknown-unknown
npm install
npm run dev
```

## Code Structure

### `game-sim` crate
- Game logic and state management
- Uses [`glam`](https://crates.io/crates/glam) for math
- Ground-truth world constants (metres, Y-up, camera/grid quantities) live here; client consumes them

### `game-net` crate
- Placeholder for multiplayer wire types and codec

### `game-server` crate
- Placeholder native multiplayer host binary

### `game-client` crate
- WebGPU rendering logic via [`wgpu`](https://crates.io/crates/wgpu)
- JavaScript interop via [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen)
- Depends on `game-sim`
- Client-only debug visuals (e.g. floor grid) must not invent world quantities already defined in `game-sim`

### Web assets (`web/`)
- `index.html` — main HTML file with the canvas element
- `style.css` — basic styling for the game canvas
- `app.js` — WASM module loader and application initializer

### Art assets (`assets/`)
- **`assets/source/`** — authoring kits (models, textures, kit READMEs). Not served by Vite.
- **`assets/cooked/`** — cook outputs only (`manifest.json` + hashed packs under `packs/`). Vite `publicDir`. Generated; gitignored. Lands in `web/dist/` on production build.
- **`web/dist/`** — sole ship tree (Vite JS/CSS/WASM under `/assets/*` + cooked packs/manifest).
- **`npm run cook`** — thin cook (gather, hash, pack, manifest). Runs automatically before `dev` / `build` / `preview`. Stamp-cache when sources unchanged.
- Loaders address packs and asset ids from the manifest — not hard-coded source paths. Kit mesh/texture facts stay in source kit READMEs, not root docs.

## Development Workflow

1. Fork the repository and create a feature branch.
2. Make your changes.
3. Format your Rust code:
   ```bash
   cargo fmt --all
   ```
4. Run linting:
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```
5. Run the test suite:
   ```bash
   cargo test
   ```
6. Commit your changes (see [Commit Message Conventions](#commit-message-conventions) below — pre-commit hooks will run automatically).
7. Push your branch and open a pull request.

## Coding Standards

- All Rust code must be formatted with `cargo fmt --all` before committing.
- All Rust code must pass `cargo clippy --all-targets --all-features -- -D warnings` with no warnings.
- Keep changes focused: each pull request should represent a single, coherent unit of work.

## Commit Message Conventions

This project follows [Conventional Commits](https://www.conventionalcommits.org/) with a **required scope**. Commitlint rejects messages that omit the scope or use one outside the allow-list.

```
<type>(<scope>): <description>
```

### Types (required)

| Type | Use for |
|------|---------|
| `feat` | New user-facing capability |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | Code change that is not a feat or fix |
| `chore` | Tooling, CI, deps, scaffolding, housekeeping |

### Scopes (required — must be one of these)

| Scope | Paths / meaning |
|-------|-----------------|
| `sim` | `crates/game-sim` |
| `net` | `crates/game-net` |
| `server` | `crates/game-server` |
| `client` | `crates/game-client` |
| `web` | `web/` |
| `assets` | `assets/` |
| `ci` | `.github/` |
| `infra` | `infra/` |
| `docs` | `docs/` |
| `repo` | Root workspace, `package.json`, husky, commitlint, `.gitignore`, README/CONTRIBUTING at root |

### Rules

- Scope is **required** (empty scope fails commitlint)
- Scope must be from the table above
- Header (first line) ≤ 72 characters
- No trailing period on the subject
- One logical change per commit
- Prefer **one scope per commit**. If a change spans areas, split the commit. If you must keep it as one, pick the **primary** scope (the bulk of the diff)

### Examples

```bash
feat(client): initialize wgpu and clear to black
fix(web): show error when WebGPU is unavailable
chore(ci): add clippy to the workflow
chore(repo): require scoped conventional commits
docs(docs): define empty-scene acceptance criteria
docs(repo): clarify commit message conventions
```

## Git Hooks

Hooks are installed when you run `npm install` (`husky` via the `prepare` script).

| Hook | Checks |
|------|--------|
| **pre-commit** | `cargo fmt --check`, `cargo clippy … -D warnings`, `lint-staged` (Prettier on staged `web/` + root JS/TS) |
| **commit-msg** | commitlint (type, **required scope**, header length) |

You don't need to run these manually for a normal commit:

```bash
git commit -m "feat(client): add reload animation"
```

Commits without a valid scoped message are rejected, for example:

```bash
# fails — missing scope
git commit -m "feat: add reload animation"

# fails — unknown scope
git commit -m "feat(frontend): add reload animation"
```

## Pull Request Process

1. Ensure your branch is up to date with `main` and all tests, formatting, and linting checks pass.
2. Open a pull request with a clear description of the change and why it's needed.
3. Link any related issues.
4. Be responsive to review feedback — a maintainer will review and may request changes before merging.

## Troubleshooting

### WebGPU not supported
Make sure you're using a browser with WebGPU support: Chrome 113+, Edge 113+, or Firefox Nightly.

### WASM build issues
Confirm the correct Rust target is installed:
```bash
rustup target add wasm32-unknown-unknown
```

### Development server won't start
Try a clean reinstall:
```bash
npm install
npm run dev
```
