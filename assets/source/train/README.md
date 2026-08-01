# Train kit (Kenney Train Kit)

Model and material facts for the train / track authoring kit. Engine commands and feature flags live in game documentation.

## Source

- **Pack:** Train Kit — Kenney ([kenney.nl/assets/train-kit](https://www.kenney.nl/assets/train-kit)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Authoring path:** `assets/source/train/`
- **Cooked delivery:** hashed packs under `assets/cooked/` (loaders address packs by id); map **a** consumes these via pack **`maps-a`**
- **Files:**
  - `models/spline-segment.glb` — two parallel metal rails (**090**)
  - `models/spline-track.glb` — wooden sleeper under the rails (**090**)
  - `models/train-locomotive-c.glb` — steam loco **c** (**091**)
  - `models/train-carriage-lumber.glb` — lumber load (**091**)
  - `models/lumber-cargo.glb` — cargo node stripped from the lumber carriage (**091** ground pile)
  - `models/train-carriage-flatbed.glb` — empty flatbed (**091**)
  - `models/train-carriage-tank.glb` — small tank car (**091**)
  - `textures/colormap.png` — shared albedo atlas

Prefab `railroad-*` pieces, passenger couplers (`train-connector`), and other rolling stock are not imported.

## Space, units, and facing

Project world axes (shared with character / blaster kits):

- **Y-up**, ground on **XZ**
- Authored kit units at **×1** are toy-scale relative to characters; map **a** scales rail and rolling stock separately (`rail.scale` **2.4**, `train.scale` **2.0**)
- Authored **track-forward** is **+Z** (along the rail; loco nose / carriage forward)
- Authored **gauge** spans **±X** (~1 kit unit overall for `spline-segment` → ~2.4 m at map **a** rail scale)

| Mesh | Approx. authored size (X × Y × Z) | Role |
|------|-----------------------------------|------|
| `spline-segment` | 1.0 × 0.075 × 0.25 | Metal rails; tile along track-forward |
| `spline-track` | 0.7 × 0.1 × 1.0 | Wooden sleeper; tile along track-forward |
| `train-locomotive-c` | ~1.4 × 1.7 × 2.8 | Steam loco; wheels sole at Y = 0 |
| `train-carriage-*` | ~1.1–1.4 × ~1.2–1.8 × 2.7 | Freight cars; wheels sole at Y = 0 |

`spline-segment` Y bands (authored): sole **0**, deck **0.05**, rail-head detail **0.075**. Wooden tracks seat on the **deck**, not the rail-head `max_y`. Seating uses scaled local bands (`T · R · S`).

Map **a** rail corridor (**090**) runs world **+X**. Instances use yaw **+π/2** so kit **+Z → world +X** and gauge sits on world **Z** about the centerline. `rail.stride` is world metres and must equal authored sleeper length × `rail.scale` (1.0 × 2.4 → **2.4**). Metal **segments** tile at half-stride on the gravel; wooden **tracks** tile at full stride with soles on the segment **deck**.

Map **a** stationed train (**091**): same yaw as the rail, own `train.scale`; consist packed front→back (east→west) with `unit_gap` between cars; midpoint at `mid_x`. Optional `ground_cargo` places the lumber pile beside a unit (unloaded story). Tune `train.seat_y`, `train.loco_z_nudge`, `train.unit_gap`, and `ground_cargo.*` on the map def.

Map **a** train collide (**092**): kit AABBs on those roots enter `MapWorld` (flatbed top = deck). Ground cargo `seat_y` buries ~half the pile so only the top layer protrudes — jump pad onto the mid flatbeds.

## Materials

- Shared material name **`colormap`**: glTF URI `Textures/colormap.png`; this repo stores `textures/colormap.png`
- Metallic factor is zero; materials are often **double-sided**
- Lit map present: texture × base color (white default), same kit path as blasters (**018** / **089** plate)
