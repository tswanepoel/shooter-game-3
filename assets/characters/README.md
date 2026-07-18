# Character kit (Kenney Blocky Characters)

Kit facts for loaders. Engine commands and debug UI live elsewhere.

## Source

- **Pack:** Blocky Characters 2.0 — Kenney ([kenney.nl](https://www.kenney.nl)), CC0
- **Export:** UnityGLTF → binary glTF (`.glb`)
- **Layout:**
  - `models/character-{a…r}.glb` — geometry + materials (no embedded image bytes)
  - `textures/texture-{a…r}.png` — matching albedo atlas (often **indexed/palette** PNG; expand to RGB(A) on load)

Pair by letter: `character-a` uses `texture-a`, and so on through `r`.

## Units and axes

- World convention for this project: **1 unit = 1 metre**, **Y-up**, XZ ground.
- Authored bind pose already places **soles at y ≈ 0** (legs sit on the ground plane).
- Full hierarchy standing height is about **2.7 kit units** after applying node TRS (head is parented under torso; do not treat mesh-local bounds alone as height).
- **Face / forward:** default face is **+Z**. (Stub camera looks −Z, so characters face the default view without extra yaw.)
- **Root scale into world metres:** **÷1.5** (`2.7 / 1.5 = 1.8 m` standing). One uniform root scale only; feet snapped to `y = 0` after scale. Do not invent per-part scale hacks.

## Materials and textures

- Materials use **`KHR_materials_unlit`** (listed in `extensionsRequired`). Loaders must accept unlit; do not require a full PBR light path.
- Present as **flat albedo:** `baseColorTexture × baseColorFactor` (factor defaults to white when omitted).
- Atlases are ordinary **sRGB PNG** bytes. For unlit present on a typical browser Unorm canvas, sample display-referred (no GPU sRGB decode on the albedo) so midtones match a PNG viewer. A linear path (`Rgba8UnormSrgb` + sRGB present encode) is for lit materials later — not required for this kit’s flat paint check.
- glTF image URIs point at `Textures/texture-*.png`; this repo stores atlases under `textures/` (same filenames). Prefer the repo path over the URI folder name.
- Samplers are sparse in the files; use **repeat** wrap on U and V so atlas UVs outside 0…1 still sample correctly.

## UVs

- `TEXCOORD_0` ranges include **negative U** and V above 1 (atlas packing with wrap).
- Sampling must **not** treat negative U as “missing texture.” Use wrap/repeat, not clamp-to-border as “untextured.”

## Hierarchy and bind pose

Typical node tree (names vary only by letter on the root):

```
character-{letter}
└── root
    ├── leg-left   (mesh)
    ├── leg-right  (mesh)
    └── torso      (mesh)
        ├── arm-left   (mesh)
        ├── arm-right  (mesh)
        └── head       (mesh; often has non-uniform scale e.g. 0.1)
```

- Multi-part body: draw all mesh nodes under one placement; keep relative bind transforms.
- Static **bind pose** only in these files (animation accessors may exist for tooling; clips/grips are out of scope for import check).
- Skinning beyond bind pose is out of scope for this kit’s first use.

## What not to put here

No engine how-tos, debug console commands, or feature flags — those belong in game docs / root README.
