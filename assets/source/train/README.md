# Train kit (Kenney Train Kit)

Model and material facts for the train / track authoring kit. Engine commands and feature flags live in game documentation.

## Source

- **Pack:** Train Kit — Kenney ([kenney.nl/assets/train-kit](https://www.kenney.nl/assets/train-kit)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Authoring path:** `assets/source/train/`
- **Cooked delivery:** hashed packs under `assets/cooked/` (loaders address packs by id); map **a** consumes these via pack **`maps-a`**
- **Files (090 atoms):**
  - `models/spline-segment.glb` — two parallel metal rails
  - `models/spline-track.glb` — wooden sleeper under the rails
  - `textures/colormap.png` — shared albedo atlas

Prefab `railroad-*` pieces and locomotives are not imported yet.

## Space, units, and facing

Project world axes (shared with character / blaster kits):

- **Y-up**, ground on **XZ**, **1 unit = 1 metre** at **×1** (no character-style root scale)
- Authored **track-forward** is **+Z** (along the rail)
- Authored **gauge** spans **±X** (~1 m overall for `spline-segment`)

| Mesh | Approx. size (X × Y × Z) | Role |
|------|--------------------------|------|
| `spline-segment` | 1.0 × 0.075 × 0.25 | Metal rails; tile along track-forward |
| `spline-track` | 0.7 × 0.1 × 1.0 | Wooden sleeper; tile along track-forward |

`spline-segment` Y bands (authored): sole **0**, deck **0.05**, rail-head detail **0.075**. Wooden tracks seat on the **deck**, not the rail-head `max_y`.

Map **a** rail corridor (**090**) runs world **+X**. Instances use yaw **+π/2** so kit **+Z → world +X** and gauge sits on world **Z** about the centerline. Metal **segments** tile at half-stride on the gravel; wooden **tracks** tile at full stride with soles on the segment **deck**.

## Materials

- Shared material name **`colormap`**: glTF URI `Textures/colormap.png`; this repo stores `textures/colormap.png`
- Metallic factor is zero; materials are often **double-sided**
- Lit map present: texture × base color (white default), same kit path as blasters (**018** / **089** plate)
