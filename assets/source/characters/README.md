# Character kit (Kenney Blocky Characters)

Kit facts for loaders. Engine commands and debug UI live elsewhere.

## Source

- **Pack:** Blocky Characters 2.0 — Kenney ([kenney.nl](https://www.kenney.nl)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Repo path:** `assets/source/characters/` (authoring kit; cook packs live under `assets/cooked/`, not this tree)
- **Layout:**
  - `models/character-{a…r}.glb` — geometry + materials (no embedded image bytes) + glTF animation clips
  - `textures/texture-{a…r}.png` — matching albedo atlas (often **indexed/palette** PNG; expand to RGB(A) on load)

Pair by letter: `character-a` uses `texture-a`, and so on through `r`. All 18 letters share the same hierarchy, bind proportions, and animation clip names.

## Units and axes

- World convention for this project: **1 unit = 1 metre**, **Y-up**, XZ ground.
- Authored bind pose already places **soles at y ≈ 0** (legs sit on the ground plane).
- Full hierarchy standing height is **2.7 kit units** after applying node TRS (head is parented under torso; do not treat mesh-local bounds alone as height).
- **Face / forward:** default face is **+Z**. (Stub camera looks −Z, so characters face the default view without extra yaw.)
- **Left / right:** mesh names use Kenney’s convention with face +Z: **`leg-left` / `arm-left` are at +X**; **`leg-right` / `arm-right` at −X**.
- **Root scale into world metres:** **÷1.5** (`2.7 / 1.5 = 1.8 m` standing). One uniform root scale only; feet snapped to `y = 0` after scale. Do not invent per-part scale hacks.

## Materials and textures

- Materials use **`KHR_materials_unlit`** (listed in `extensionsRequired`). Loaders must accept unlit; do not require a full PBR light path.
- Present as **flat albedo:** `baseColorTexture × baseColorFactor` (factor defaults to white when omitted).
- Atlases are ordinary **sRGB PNG** bytes. For unlit present on a typical browser Unorm canvas, sample display-referred (no GPU sRGB decode on the albedo) so midtones match a PNG viewer. A linear path (`Rgba8UnormSrgb` + sRGB present encode) is for lit materials later — not required for this kit’s flat paint check.
- glTF image URIs point at `Textures/texture-*.png`; this repo stores atlases under `textures/` (same filenames). Prefer the repo path over the URI folder name.
- Samplers are sparse in the files; use **repeat** wrap on U and V so atlas UVs outside 0…1 still sample correctly.
- Extensions present: `KHR_materials_unlit` (required), `KHR_texture_transform` (used).

## UVs

- `TEXCOORD_0` ranges include **negative U** and V above 1 (atlas packing with wrap).
- Sampling must **not** treat negative U as “missing texture.” Use wrap/repeat, not clamp-to-border as “untextured.”

## Hierarchy and bind pose

Node tree (root name varies only by letter; child names are fixed):

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

- Multi-part body: draw all mesh nodes under one placement; keep relative bind transforms.
- **No skins.** There are no `skins`, no `JOINTS_0` / `WEIGHTS_0`, no inverse-bind matrices, and no morph targets. Motion is **rigid part TRS** (each limb is a separate mesh under a transform node).
- Mesh vertex attributes: `POSITION`, `NORMAL`, `TANGENT`, `TEXCOORD_0`.
- “Joints” for this kit are the **node origins** (pivots) that animation channels target.

### Bind pivots (character space, kit units)

Pivot = node origin after parent TRS. Bind local rotations are identity. Only `head` has non-identity scale (`0.1, 0.1, 0.1`).

| Node | Parent | Local translation | Local scale | Pivot (kit) | Pivot (m, ÷1.5) | Role |
|------|--------|-------------------|-------------|-------------|-----------------|------|
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

How each cube hangs off its pivot. Local AABB is mesh space; world values are after node TRS (including head scale).

| Part | Local AABB min → max | Local size | World center (kit) | World size (kit) | World size (m) |
|------|----------------------|------------|--------------------|------------------|----------------|
| `leg-left` | `(-0.2, -1, -0.2)` → `(0.2, 0, 0.2)` | `0.4 × 1.0 × 0.4` | `(+0.2, 0.5, 0)` | `0.4 × 1.0 × 0.4` | `0.267 × 0.667 × 0.267` |
| `leg-right` | same local | same | `(-0.2, 0.5, 0)` | same | same |
| `torso` | `(-0.4, 0.3, -0.3)` → `(0.4, 1.2, 0.3)` | `0.8 × 0.9 × 0.6` | `(0, 1.45, 0)` | `0.8 × 0.9 × 0.6` | `0.533 × 0.600 × 0.400` |
| `arm-left` | `(0, -1, -0.2)` → `(0.4, 0.1, 0.2)` | `0.4 × 1.1 × 0.4` | `(+0.6, 1.35, -0.1)` | `0.4 × 1.1 × 0.4` | `0.267 × 0.733 × 0.267` |
| `arm-right` | `(-0.4, -1, -0.2)` → `(0, 0.1, 0.2)` | same | `(-0.6, 1.35, -0.1)` | same | same |
| `head` | `(-4, 0, -4)` → `(4, 8, 4)` (pre-scale) | `8 × 8 × 8` local | `(0, 2.3, 0)` | `0.8 × 0.8 × 0.8` | `0.533 × 0.533 × 0.533` |

Pivot semantics:

- **Legs:** pivot at the **top** of the limb (hip); mesh hangs down so soles sit on `y = 0`.
- **Arms:** pivot at the **shoulder**; mesh hangs down and outward; shoulder has a slight **−Z** offset (`-0.1` kit).
- **Torso:** pivot near the **bottom** of the torso volume (waist); mesh extends upward.
- **Head:** pivot at the **bottom** of the head cube (neck); geometry is authored large and reduced by **uniform scale 0.1**. Preserve that scale when applying animation scale channels.

### Overall proportions (bind pose AABB)

| Measure | Kit units | Metres (÷1.5) |
|---------|----------:|--------------:|
| Standing height | **2.7** | **1.8** |
| Full width (outer arm extents) | **1.6** | **1.067** |
| Depth (front–back) | **0.8** | **0.533** |
| Hip width (leg pivots) | **0.4** | **0.267** |
| Shoulder width (arm pivots) | **0.8** | **0.533** |
| Leg length (hip pivot → sole) | **1.0** | **0.667** |
| Arm length (along limb) | **~1.1** | **~0.733** |

World AABB (kit): min `(-0.8, 0, -0.4)`, max `(0.8, 2.7, 0.4)`.

Proportions are blocky/stylized, not human anthropometry. There is no knee, elbow, wrist, or multi-segment spine — only the pivots above.

## Animations

Each `character-*.glb` embeds **27** named glTF 2.0 clips (same names on every letter). Interpolation is **LINEAR**. Channels target **node translation / rotation / scale** (sparse: unmentioned nodes stay at bind or last applied pose — layering policy is engine-side).

There is **no** skeletal skinning path: play clips by sampling channel times onto local TRS, recomputing the hierarchy, and drawing each of the six meshes with its world matrix.

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
| `holding-right` | 0.17 | Static right-hand hold pose |
| `holding-left` | 0.17 | Static left-hand hold pose |
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

Hold vs shoot and hold vs walk are authored as **separate clips** so an engine can layer them; the files do not define blend trees.

Root translation on walk / sprint / die / melee is in **kit units** (still apply the single **÷1.5** root scale into metres).

### Import / lineup scope

Lineup and first-load paint checks only need the **static bind pose**. Clips remain in the files for later animation playback; they are not required for scale/facing/albedo verification.

## What not to put here

No engine how-tos, debug console commands, or feature flags — those belong in game docs / root README.
