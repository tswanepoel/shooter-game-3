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

When a blaster is placed through a chain that already applies the character **×1/1.5** scale to positions, the blaster mesh uses an additional uniform scale of **1.5** relative to that chain so its authored size remains **×1** in metres. Grip **positions** stay in character-kit units and take the character scale with the body.

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

Held presentation combines three independent facts: **where the hand is**, **how the gun is oriented in character space**, and **each kit’s scale into metres**.

### Grip point (from the character)

With the character in the **`holding-right`** pose, the grip point is the image of a per-blaster **offset** under the **`arm-right`** node matrix (character kit space). Offsets are listed below. That hierarchy supplies **position** for the weapon origin.

Character hold and limb layout: `assets/source/characters/README.md`.

### Weapon orientation (character space)

In character / world axes (face +Z, up +Y), a single yaw aligns the authored mesh to the held aim:

**180° about Y** maps mesh **−Z (muzzle) → character +Z (forward)** and keeps mesh **+Y** as world **+Y** (top up).

### Scale

Character body: **×1/1.5** from kit units to metres.  
Blaster mesh: **×1** from authored units to metres.  
Relative factor on the mesh when positions already follow the character scale: **1.5**.

### Composition

```
kit_to_world     = placement · (character scale 1/1.5) · feet on y = 0
grip_in_kit      = arm_right (holding-right) · grip_offset
held_blaster     = kit_to_world · translate(grip_in_kit) · rotate_y(180°) · (blaster scale relative to character chain)
```

### Grip offsets

Offsets are in **`arm-right` local space after `holding-right`** (character kit units).

| Blaster | Offset (x, y, z) |
|---------|------------------|
| `blaster-a` | `(0, -1.14, 0.34)` |
| `blaster-b` | `(0, -1.00, 0.30)` |
| `blaster-c` | `(0, -1.11, 0.20)` |
| `blaster-d` | `(0, -1.11, 0.18)` |
| `blaster-e` | `(0, -2.34, 0.22)` |
| `blaster-f` | `(0, -1.39, 0.19)` |
| `blaster-g` | `(0, -1.27, 0.22)` |
| `blaster-h` | `(0, -1.25, 0.24)` |
| `blaster-i` | `(0, -0.93, 0.22)` |
| `blaster-j` | `(0, -1.20, 0.15)` |
| `blaster-k` | `(0, -1.09, 0.20)` |
| `blaster-l` | `(0, -1.16, 0.20)` |
| `blaster-m` | `(0, -1.18, 0.26)` |
| `blaster-n` | `(0, -0.99, 0.22)` |
| `blaster-o` | `(0, -1.06, 0.19)` |
| `blaster-p` | `(0, -1.21, 0.14)` |
| `blaster-q` | `(0, -1.28, 0.19)` |
| `blaster-r` | `(0, -1.18, 0.10)` |

### Muzzle points

Barrel exits for each blaster (one or more). Offsets use the **same space as grip offsets** — **arm-attachment frame**: **`arm-right` local after `holding-right`**, character-kit / recipe units (not world metres). Values match the Kenney blaster glTF recipe and the historical `muzzlePoints` list per weapon.

Debug lineup draws a magenta ball at **each** listed point (feature 012).

| Blaster | Muzzle points (x, y, z) |
|---------|-------------------------|
| `blaster-a` | `(0, -1.7, 0.42)` |
| `blaster-b` | `(0, -1.39, 0.32)` |
| `blaster-c` | `(0, -1.47, 0.23)` |
| `blaster-d` | `(0, -1.795, 0.265)` |
| `blaster-e` | `(0.07, -2.34, 0.26)` |
| `blaster-f` | `(0, -2.37, 0.26)` |
| `blaster-g` | `(0, -1.8, 0.34)` |
| `blaster-h` | `(0, -1.73, 0.28)` |
| `blaster-i` | `(0, -1.32, 0.26)`, `(0, -1.32, 0.15)` |
| `blaster-j` | `(-0.045, -1.655, 0.29)`, `(0.045, -1.655, 0.29)` |
| `blaster-k` | `(0, -1.44, 0.18)` |
| `blaster-l` | `(-0.1, -1.58, 0.26)`, `(0.1, -1.58, 0.26)` |
| `blaster-m` | `(0, -1.65, 0.37)` |
| `blaster-n` | `(0, -1.47, 0.32)` |
| `blaster-o` | `(-0.05, -1.35, 0.25)`, `(0.05, -1.35, 0.25)`, `(-0.05, -1.35, 0.15)`, `(0.05, -1.35, 0.15)` |
| `blaster-p` | `(0, -1.855, 0.235)`, `(0, -1.855, 0.14)` |
| `blaster-q` | `(0, -1.82, 0.28)`, `(0, -1.82, 0.06)` |
| `blaster-r` | `(0, -1.81, 0.23)` |
