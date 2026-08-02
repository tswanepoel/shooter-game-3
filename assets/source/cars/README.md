# Car kit (Kenney Car Kit)

Model and material facts for the yard vehicle authoring kit. Engine commands and feature flags live in game documentation.

## Source

- **Pack:** Car Kit — Kenney ([kenney.nl/assets/car-kit](https://www.kenney.nl/assets/car-kit)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Authoring path:** `assets/source/cars/`
- **Cooked delivery:** hashed packs under `assets/cooked/`; map **a** consumes these via pack **`maps-a`**
- **Files:**
  - `models/tractor.glb` — yard tractor (**094**)
  - `textures/colormap.png` — shared albedo atlas

Other car-kit vehicles / loose wheels are not imported.

## Space, units, and facing

Project world axes (shared with character / train kits):

- **Y-up**, ground on **XZ**
- Authored kit units at **×1** are near character scale (tractor body ~1.6 m tall); map **a** uses its own `train.tractor.scale` (not `train.scale`)
- Authored **forward** is **+Z**; wheels sit on **Y = 0**
- Approx. authored size (merged body + wheels): ~1.3 × 1.6 × 2.2

Map **a** yard tractor (**094**): pose knobs on `train.tractor` beside `ground_cargo` (south of the home rail, toward the lumber car). Own `car.colormap` batch — do not merge into the train lit batch. Collide/support uses a kit AABB on the same root (hood / body band).
