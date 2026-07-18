# Feature 010 - Asset cook

Delivered content is grouped by **demand cadence** and produced by one **cook** into packs the game addresses by id. Authoring kits are not the load path. JS/WASM stay their own bundle; art is not embedded in the Vite JS graph. Cook is a build gate (gather, hash, pack, manifest) — not a custom GPU-format pipeline. Prefer boring wire-friendly bytes (e.g. glTF + PNG); heavy transforms are optional later inside the same pack contract.

## Acceptance Criteria

- **Source → cooked → ship.** Kits live under `assets/source/`. Cook writes under `assets/cooked/` (packs + manifest); that is Vite’s `publicDir`. Sole ship tree is `web/dist/` (Vite chunks under `/assets/*` plus cooked packs). Kit facts stay with source READMEs.
- **One cook step** in `dev` / `build` (or a documented prerequisite). Same philosophy in both; stamp-cache when sources unchanged is fine. Vite owns page HMR; wasm-pack owns WASM; cook owns art packs.
- **Pack by cadence.** What loads together ships together. V1: a Kenney core pack covering the character kit for lineup / play. Blasters and projectiles join that pack (or a same-cadence sibling) when needed — not one URL per authoring file as the product model. Split packs only when demand splits.
- **Manifest** lists packs (id, URL, hash, size). Loaders use pack/asset ids, not hard-coded source paths. Lineup (008) loads via this path.
- **Thin cook (v1):** hashed cooked artifacts from source; optional light strip/normalize. Not required: custom mesh containers, KTX2/Basis, offline GPU re-encode, or banishing glTF/PNG from debug loaders.
- Release WASM still strips `debug-tools` (001 / 003). Debug may decode pack contents with ordinary loaders.
- Hashed pack URLs are long-cache friendly. Host Brotli/gzip where available; no CDN ownership required here.
- README / CONTRIBUTING: source vs cooked vs `web/dist`, cook command, cooked-only public root. No per-kit mesh facts in root docs.
- Out of scope: world streaming/residency beyond load-this-pack, hard CI byte budgets, proprietary containers as a goal, format purity over wire size, rewriting features 001–009.
