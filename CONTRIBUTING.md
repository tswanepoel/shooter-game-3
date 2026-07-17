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

The shooter game is structured as a Rust workspace with two main crates:

- **`game-sim`** — server-side game logic and simulation
- **`game-client`** — WebGPU rendering and client-side logic

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
- Uses [`serde`](https://crates.io/crates/serde) for serialization
- Uses [`rapier3d`](https://crates.io/crates/rapier3d) for physics simulation

### `game-client` crate
- WebGPU rendering logic via [`wgpu`](https://crates.io/crates/wgpu)
- JavaScript interop via [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen)

### Web assets (`web/`)
- `index.html` — main HTML file with the canvas element
- `style.css` — basic styling for the game canvas
- `app.js` — WASM module loader and application initializer

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

This project follows the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>
```

- **Allowed types:** `feat`, `fix`, `docs`, `refactor`, `chore`
- Keep the first line under 72 characters
- Each commit should represent a single logical change — group related changes together rather than mixing unrelated edits in one commit

## Pre-commit Hooks

Pre-commit hooks are installed automatically when you clone the repository. On every commit, they automatically run:

- `cargo fmt`
- `cargo clippy`

You don't need to run them manually — they trigger as part of the normal commit flow:

```bash
git commit -m "feat(client): add reload animation"
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
