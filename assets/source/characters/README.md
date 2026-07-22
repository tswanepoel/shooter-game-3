# Character kit (Kenney Blocky Characters)

Model and material facts for the Blocky Characters authoring kit. Engine commands, debug UI, and feature flags live in game documentation and the root project README.

## Source

- **Pack:** Blocky Characters 2.0 — Kenney ([kenney.nl](https://www.kenney.nl)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Authoring path:** `assets/source/characters/`
- **Cooked delivery:** hashed packs under `assets/cooked/` (loaders address packs by id)
- **Files:**
  - `models/character-{a…r}.glb` — geometry, materials, node hierarchy, and animation clips (image bytes live in separate PNGs)
  - `textures/texture-{a…r}.png` — matching albedo atlas (often indexed/palette PNG; expand to RGB(A) on decode)

Letter pairing is one-to-one: `character-a` with `texture-a`, through `r`. All eighteen letters share the same hierarchy, bind proportions, and animation clip names.

## Space, units, and facing

Project world after the character root scale:

- **1 unit = 1 metre**, **Y-up**, ground on the **XZ** plane
- **Face / forward:** **+Z**
- **Character left / right** (face +Z): **left limbs at +X**, **right limbs at −X** (`leg-left`, `arm-left` vs `leg-right`, `arm-right`)

Authored (kit) space before root scale:

- Bind pose places **soles on y = 0**
- Full hierarchy standing height is **2.7 kit units** after node TRS (head is parented under torso; standing height is a hierarchy quantity)
- **Root scale into metres:** multiply by **1/1.5** → standing height **1.8 m**. One uniform scale on the character placement. After scale, soles remain on **y = 0** (feet snap from sole bounds if needed)

## Materials and textures

- Materials list **`KHR_materials_unlit`** in `extensionsRequired` (and `KHR_texture_transform` among used extensions)
- Presentation for this kit’s flat paint: **base color texture × base color factor** (factor defaults to white when omitted)
- Atlases are **sRGB PNG** bytes. On a typical browser Unorm canvas with unlit shading, sample **display-referred** (albedo without GPU sRGB decode) so midtones match a PNG viewer
- glTF image URIs use folder `Textures/`; this repository stores the same filenames under `textures/`
- Samplers are sparse; atlas UVs use **repeat** wrap on U and V

## UVs

- `TEXCOORD_0` spans values **outside 0…1**, including **negative U**, by design (atlas packing with wrap)
- Correct sampling uses **repeat/wrap** address modes so those coordinates map into the atlas

## Hierarchy and bind pose

Rigid multi-part body: each limb is its own mesh under a transform node. Animation and hold poses act on **node local translation, rotation, and scale**. Vertex attributes present: `POSITION`, `NORMAL`, `TANGENT`, `TEXCOORD_0`.

Node tree (root name varies by letter; child names are fixed):

```
character-{letter}
└── root
    ├── leg-left   (mesh)
    ├── leg-right  (mesh)
    └── torso      (mesh)
        ├── arm-left   (mesh)
        ├── arm-right  (mesh)
        └── head       (mesh; bind scale 0.1 on all axes)
```

Draw all six mesh nodes under one character placement, preserving relative bind transforms. Pivot points for motion are the **node origins**.

### Bind pivots (character space, kit units)

Pivot = node origin after parent TRS. Bind local rotations are identity. Only `head` has non-identity scale (`0.1, 0.1, 0.1`).

| Node | Parent | Local translation | Local scale | Pivot (kit) | Pivot (m, ×1/1.5) | Role |
|------|--------|-------------------|-------------|-------------|-------------------|------|
| `character-{letter}` | — | `(0, 0, 0)` | `(1,1,1)` | `(0, 0, 0)` | origin | Placement root |
| `root` | character | `(0, 0, 0)` | `(1,1,1)` | `(0, 0, 0)` | origin | Locomotion bob / die |
| `leg-left` | root | `(+0.2, 1.0, 0)` | `(1,1,1)` | `(+0.2, 1.0, 0)` | `(+0.133, 0.667, 0)` | Hip (top of leg) |
| `leg-right` | root | `(-0.2, 1.0, 0)` | `(1,1,1)` | `(-0.2, 1.0, 0)` | `(-0.133, 0.667, 0)` | Hip |
| `torso` | root | `(0, 0.7, 0)` | `(1,1,1)` | `(0, 0.7, 0)` | `(0, 0.467, 0)` | Waist / lower torso |
| `arm-left` | torso | `(+0.4, 1.1, -0.1)` | `(1,1,1)` | `(+0.4, 1.8, -0.1)` | `(+0.267, 1.200, -0.067)` | Shoulder |
| `arm-right` | torso | `(-0.4, 1.1, -0.1)` | `(1,1,1)` | `(-0.4, 1.8, -0.1)` | `(-0.267, 1.200, -0.067)` | Shoulder |
| `head` | torso | `(0, 1.2, 0)` | `(0.1, 0.1, 0.1)` | `(0, 1.9, 0)` | `(0, 1.267, 0)` | Neck / base of head |

Landmark heights as a fraction of standing height (2.7 kit / 1.8 m): soles 0%, torso pivot ~26%, hips ~37%, shoulders ~67%, head pivot ~70%, head top 100%.

### Mesh extents (bind pose)

Local AABB is mesh space; world values are after node TRS (including head scale).

| Part | Local AABB min → max | Local size | World center (kit) | World size (kit) | World size (m) |
|------|----------------------|------------|--------------------|------------------|----------------|
| `leg-left` | `(-0.2, -1, -0.2)` → `(0.2, 0, 0.2)` | `0.4 × 1.0 × 0.4` | `(+0.2, 0.5, 0)` | `0.4 × 1.0 × 0.4` | `0.267 × 0.667 × 0.267` |
| `leg-right` | same local | same | `(-0.2, 0.5, 0)` | same | same |
| `torso` | `(-0.4, 0.3, -0.3)` → `(0.4, 1.2, 0.3)` | `0.8 × 0.9 × 0.6` | `(0, 1.45, 0)` | `0.8 × 0.9 × 0.6` | `0.533 × 0.600 × 0.400` |
| `arm-left` | `(0, -1, -0.2)` → `(0.4, 0.1, 0.2)` | `0.4 × 1.1 × 0.4` | `(+0.6, 1.35, -0.1)` | `0.4 × 1.1 × 0.4` | `0.267 × 0.733 × 0.267` |
| `arm-right` | `(-0.4, -1, -0.2)` → `(0, 0.1, 0.2)` | same | `(-0.6, 1.35, -0.1)` | same | same |
| `head` | `(-4, 0, -4)` → `(4, 8, 4)` (pre-scale) | `8 × 8 × 8` local | `(0, 2.3, 0)` | `0.8 × 0.8 × 0.8` | `0.533 × 0.533 × 0.533` |

Pivot placement:

- **Legs:** pivot at the **top** of the limb (hip); mesh extends downward to soles on **y = 0**
- **Arms:** pivot at the **shoulder**; mesh extends downward and outward; shoulder carries a slight **−Z** offset (**−0.1** kit)
- **Torso:** pivot near the **bottom** of the torso volume (waist); mesh extends upward
- **Head:** pivot at the **bottom** of the head volume (neck); geometry is authored large and reduced by **uniform scale 0.1** (preserve that scale when evaluating animation scale channels)

### Overall proportions (bind pose AABB)

| Measure | Kit units | Metres (×1/1.5) |
|---------|----------:|----------------:|
| Standing height | **2.7** | **1.8** |
| Full width (outer arm extents) | **1.6** | **1.067** |
| Depth (front–back) | **0.8** | **0.533** |
| Hip width (leg pivots) | **0.4** | **0.267** |
| Shoulder width (arm pivots) | **0.8** | **0.533** |
| Leg length (hip pivot → sole) | **1.0** | **0.667** |
| Arm length (along limb) | **~1.1** | **~0.733** |

World AABB (kit): min `(-0.8, 0, -0.4)`, max `(0.8, 2.7, 0.4)`.

Proportions are blocky and stylized. Articulation pivots are the eight nodes above (placement, root, hips, waist, shoulders, neck).

## Animations

Each `character-*.glb` embeds **27** named glTF 2.0 clips (identical names on every letter). Interpolation is **LINEAR**. Channels write **node translation, rotation, and/or scale**. Channels are sparse: nodes omitted from a clip keep their bind local TRS until another clip or layer sets them.

Playback evaluates channel times onto local TRS, rebuilds the hierarchy, and draws each of the six meshes with its world matrix.

### Clip catalog

Durations from kit inspection (`character-a`; other letters share the same names).

| Clip | ~Duration (s) | Notes |
|------|--------------:|-------|
| `static` | 0.10 | Full hierarchy TRS snapshot (bind-like reset) |
| `idle` | 1.33 | Upper-body rotation loop |
| `walk` | 0.67 | Root translation bob + leg/arm/head rotation |
| `sprint` | 0.50 | Faster locomotion |
| `sit` | 0.17 | Seated pose |
| `drive` | 0.17 | Seated drive pose |
| `die` | 0.33 | Collapse (root translation + full-body rotation) |
| `pick-up` | 0.33 | Bend / reach |
| `emote-yes` | 0.67 | Affirm gesture |
| `emote-no` | 0.67 | Negate gesture |
| `holding-right` | 0.17 | Right arm hold pose (`arm-right` rotation) |
| `holding-left` | 0.17 | Left arm hold pose |
| `holding-both` | 0.17 | Two-handed hold pose |
| `holding-right-shoot` | 0.20 | Short recoil on right hold |
| `holding-left-shoot` | 0.20 | Short recoil on left hold |
| `holding-both-shoot` | 0.20 | Short recoil on two-hand hold |
| `attack-melee-right` | 0.42 | Melee swing |
| `attack-melee-left` | 0.42 | Melee swing |
| `attack-kick-right` | 0.58 | Kick |
| `attack-kick-left` | 0.58 | Kick |
| `interact-right` | 0.67 | Use / interact |
| `interact-left` | 0.67 | Use / interact |
| `wheelchair-sit` | 0.17 | Wheelchair seated pose |
| `wheelchair-move-forward` | 0.50 | Wheelchair locomotion |
| `wheelchair-move-back` | 0.50 | Wheelchair locomotion |
| `wheelchair-move-left` | 0.50 | Wheelchair locomotion |
| `wheelchair-move-right` | 0.50 | Wheelchair locomotion |

Hold, shoot, and locomotion clips are separate so a consumer may layer them. Root translation on walk, sprint, die, and melee is in **kit units** and then follows the character **×1/1.5** root scale into metres.

### `holding-right` (armed presentation)

The `holding-right` clip sets **`arm-right` rotation** to a static hold (approximately **−90° about local X**), so the right limb’s hand direction aligns with character **+Z**. Other body nodes remain at bind pose under that clip alone.

Armed lineup and self present use this hold for the **body silhouette**. The right-hand **hand socket** is a presentation logic node parented to **`arm-right`** (not a GLB bone): under hold it cancels this clip’s arm rotation and yaws so a held weapon sits level with muzzle character-forward. Feature **037** owns the socket → weapon grip composition; blaster grip `G` and muzzle tables: `assets/source/blasters/README.md`.
