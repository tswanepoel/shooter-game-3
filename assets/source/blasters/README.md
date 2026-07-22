# Blaster kit (Kenney Blaster Kit)

Model and material facts for the Blaster authoring kit. Engine commands, debug UI, and feature flags live in game documentation and the root project README.

## Source

- **Pack:** Blaster Kit — Kenney ([kenney.nl](https://www.kenney.nl/assets/blaster-kit)), CC0
- **Authoring path:** `assets/source/blasters/`
- **Cooked delivery:** hashed packs under `assets/cooked/` (loaders address packs by id)
- **Files:**
  - `models/blaster-{a…r}.glb` — weapon meshes and child parts (magazine, scope, …); image bytes live in a shared PNG
  - `textures/colormap.png` — shared albedo atlas for the blaster letter set (and other kit props in the same source folder)

Letter pairing with characters is one-to-one when both are shown together: `blaster-a` with `character-a`, through `r`.

## Space, units, and facing

Project world axes (shared with the character kit):

- **Y-up**, ground on **XZ**, face / forward **+Z**
- After each kit’s own scale, **1 unit = 1 metre**

| Quantity | Character kit | Blaster kit |
|----------|---------------|-------------|
| Authored units → metres | **×1/1.5** (2.7 kit → 1.8 m standing) | **×1** (authored size is final size) |
| Forward in world | face **+Z** | muzzle along **+Z** when held for presentation |
| Up in world | **+Y** | top of weapon **+Y** when held for presentation |

Blaster length is on the order of **~0.8** authored units at **×1**.

When a blaster is placed through a chain that already applies the character **×1/1.5** scale to positions, the blaster mesh uses an additional uniform scale of **1.5** relative to that chain so its authored size remains **×1** in metres. Hand-socket placement rides the character scale chain; grip **G** and muzzles are blaster-local under `held_blaster` (feature 037).

## Materials and textures

- Shared material name **`colormap`**: base color texture references `Textures/colormap.png` in the glTF; this repository stores that file as `textures/colormap.png`
- Metallic factor is zero; materials are often **double-sided**
- Flat debug presentation matches characters: **texture × base color factor** (white default), display-referred sampling on a Unorm canvas
- Cooked packs expose the shared atlas as a first-class asset alongside each `blaster-{letter}` mesh

## Hierarchy

- Root node is typically `blaster-{letter}`, with optional children (`magazine`, `scope`, …) under bind local TRS
- Full node tree draws under one placement
- Files contain bind geometry, materials, and the static node hierarchy

## Model axes

Authored blaster local frame:

| Axis | Meaning |
|------|---------|
| **−Z** | Muzzle (exit end of the barrel) |
| **+Z** | Stock / rear |
| **+Y** | Top of the weapon |
| **+X** | Side of the weapon |

## Held presentation with a character

Feature **037** owns the arm → hand → blaster contract. Two named frames and one composition path:

| Frame | Owner | Space | Role |
|-------|--------|--------|------|
| **Hand socket** `H` | character / `arm-right` | arm-local | Where the fist is and how the palm faces (shared by all weapons). |
| **Weapon grip** `G` | blaster letter | blaster-local | Where the handle / mesh origin sits relative to that hand. |

Character hold and limb layout: `assets/source/characters/README.md`.

### Hand socket `H_hold`

Logic node on **`arm-right`** (not a GLB bone). Under armed hold and aim:

- **Rotation:** cancel kit clip **`holding-right`** (**+90° about local X**) then **180° about Y** so mesh **−Z (muzzle) → character +Z** with mesh **+Y** up.
- **Translation:** identity on `H` for this kit; fist placement is carried by per-letter **`G`** (mesh origin on the socket). A later **`H_loco`** socket is optional if in-hand angle under sprint needs its own authoring.

### Scale

Character body: **×1/1.5** from kit units to metres.  
Blaster mesh: **×1** from authored units to metres.  
Relative factor **`S_blaster`** when positions already follow the character scale: **1.5** (`BLASTER_UNITS_TO_M / CHAR_UNITS_TO_M`).

### Composition (authoritative)

```
kit_to_world     = placement · (character scale 1/1.5) · feet on y = 0
hand_kit         = arm_right_kit · H_hold
held_blaster     = kit_to_world · hand_kit · inv(G) · S_blaster
muzzle_world     = held_blaster · muzzle_local
```

- **`arm_right_kit`:** posed `arm-right` after bind / hold / aim / loco (same matrix that draws the arm mesh).
- **`H_hold`:** shared hand socket (above).
- **`G`:** per-letter weapon grip (table below).
- Self present, debug lineup held pair, muzzle markers, and present muzzle FX all use this path.

### Weapon grip `G`

Per-letter translation of the mesh origin relative to the hand socket, in **blaster-local** units (pre `S_blaster`). Identity `G` would put the blaster origin on the socket with axes already matching after `H`. Values preserve the historical hold look (migrated from the former arm-attachment grip table).

| Blaster | Grip `G` translation (x, y, z) |
|---------|--------------------------------|
| `blaster-a` | `(0, -0.34, 1.14)` |
| `blaster-b` | `(0, -0.30, 1.00)` |
| `blaster-c` | `(0, -0.20, 1.11)` |
| `blaster-d` | `(0, -0.18, 1.11)` |
| `blaster-e` | `(0, -0.22, 2.34)` |
| `blaster-f` | `(0, -0.19, 1.39)` |
| `blaster-g` | `(0, -0.22, 1.27)` |
| `blaster-h` | `(0, -0.24, 1.25)` |
| `blaster-i` | `(0, -0.22, 0.93)` |
| `blaster-j` | `(0, -0.15, 1.20)` |
| `blaster-k` | `(0, -0.20, 1.09)` |
| `blaster-l` | `(0, -0.20, 1.16)` |
| `blaster-m` | `(0, -0.26, 1.18)` |
| `blaster-n` | `(0, -0.22, 0.99)` |
| `blaster-o` | `(0, -0.19, 1.06)` |
| `blaster-p` | `(0, -0.14, 1.21)` |
| `blaster-q` | `(0, -0.19, 1.28)` |
| `blaster-r` | `(0, -0.10, 1.18)` |

### Muzzle points

Barrel exits in **blaster-local** units. World position is always `held_blaster · muzzle_local`. Values preserve the historical lineup markers (migrated from the former arm-attachment muzzle table).

Debug lineup draws a magenta ball at **each** listed point (feature 012).

| Blaster | Muzzle points (x, y, z) |
|---------|-------------------------|
| `blaster-a` | `(0, 0.053333, -0.373333)` |
| `blaster-b` | `(0, 0.013333, -0.26)` |
| `blaster-c` | `(0, 0.02, -0.24)` |
| `blaster-d` | `(0, 0.056667, -0.456667)` |
| `blaster-e` | `(-0.046667, 0.026667, 0)` |
| `blaster-f` | `(0, 0.046667, -0.653333)` |
| `blaster-g` | `(0, 0.08, -0.353333)` |
| `blaster-h` | `(0, 0.026667, -0.32)` |
| `blaster-i` | `(0, 0.026667, -0.26)`, `(0, -0.046667, -0.26)` |
| `blaster-j` | `(0.03, 0.093333, -0.303333)`, `(-0.03, 0.093333, -0.303333)` |
| `blaster-k` | `(0, -0.013333, -0.233333)` |
| `blaster-l` | `(0.066667, 0.04, -0.28)`, `(-0.066667, 0.04, -0.28)` |
| `blaster-m` | `(0, 0.073333, -0.313333)` |
| `blaster-n` | `(0, 0.066667, -0.32)` |
| `blaster-o` | `(0.033333, 0.04, -0.193333)`, `(-0.033333, 0.04, -0.193333)`, `(0.033333, -0.026667, -0.193333)`, `(-0.033333, -0.026667, -0.193333)` |
| `blaster-p` | `(0, 0.063333, -0.43)`, `(0, 0, -0.43)` |
| `blaster-q` | `(0, 0.06, -0.36)`, `(0, -0.086667, -0.36)` |
| `blaster-r` | `(0, 0.086667, -0.42)` |
