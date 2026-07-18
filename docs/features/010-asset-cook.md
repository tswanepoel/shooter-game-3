# Feature 010 - Asset cook

Delivered content is grouped by **demand cadence** and produced by one **cook** into packs the game addresses by id. Authoring kits live under source; loaders consume cooked packs. JS/WASM stay their own bundle; art is separate from the Vite JS graph. Cook gathers, hashes, packs, and writes a manifest—wire-friendly bytes (glTF + PNG). Heavier transforms may join the same pack contract later.

## Acceptance criteria

- **Source → cooked → ship.** Kits live under `assets/source/`. Cook writes under `assets/cooked/` (packs + manifest); that is Vite’s `publicDir`. Sole ship tree is `web/dist/` (Vite chunks under `/assets/*` plus cooked packs). Kit facts stay with source READMEs.
- **One cook step** in `dev` / `build` (or a documented prerequisite). Same philosophy in both; stamp-cache when sources are unchanged is fine. Vite owns page HMR; wasm-pack owns WASM; cook owns art packs.
- **Pack by cadence.** What loads together ships together. V1: a Kenney core pack covering the **character and blaster** kits for lineup and play. Projectiles and other props join that pack (or a same-cadence sibling) when demand requires it. Split packs when demand splits.
- **Manifest** lists packs (id, URL, hash, size). Loaders use pack and asset ids. Lineup (008 / 011) loads through this path.
- **Thin cook (v1):** hashed cooked artifacts from source; optional light strip/normalize. Debug loaders decode ordinary glTF/PNG from pack payloads.
- Release WASM strips `debug-tools` (001 / 003). Debug builds decode pack contents with ordinary loaders.
- Hashed pack URLs are long-cache friendly. Host Brotli/gzip where available.
- README / CONTRIBUTING describe source vs cooked vs `web/dist`, the cook command, and cooked-only public root. Per-kit mesh and material facts stay in source kit READMEs.
