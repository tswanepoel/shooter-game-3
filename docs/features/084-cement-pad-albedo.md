# Feature 084 - Cement pad albedo texture

**083** textured the unmarked gravel ground. Override pads still paint flat kind albedos — cement reads as a grey rectangle on grit. This feature gives **cement** the same material treatment: a seamless albedo, world-metre tiling, and mip filtering on map **a** cement `foot_patches`.

Client present only. Same foot kind id as **070** / **082** (voice + look stay one noun). Wet cement and grass stay flat. No yard fiction doc.

Depends on **083** (textured solid upload, world-XZ UVs, PNG/JPEG decode, mips), **082** (kind→albedo table + pads), **010** (cook packs), **009** (albedo mips / filter).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `cement.jpg` / `cement.png` | `assets/source/materials/` | Seamless albedo; cook prefers `.jpg` / `.jpeg` then `.png`; asset id `cement.albedo` |

Cook packs it into pack id `maps-a` beside `map-a.def` / `gravel.albedo` as asset id `cement.albedo` (bytes as-is — thin cook; loaders decode PNG or JPEG by magic).

### What the renderer needs (for the PNG)

| Need | Spec |
| --- | --- |
| Role | Diffuse / albedo only (unlit `tex * base_color`) |
| Content | Dry cement / concrete slab look — cool grey, fine aggregate, subtle stain OK |
| Alpha | Opaque (RGB; A ignored or 1) |
| Seam | Tileable on both axes (no hard edge at wrap) |
| Resolution | Power-of-two preferred (e.g. 512² or 1024²); one tile ≈ authored metres below |
| Contrast | Moderate — grit readable at standing eye height without sparkling after mips |
| Not needed | Normals, roughness, AO, height, logos, perspective photos |

## Present

- **Cement `foot_patches`** on map **a** sample `cement.albedo` instead of a solid colour fill.
- **World tiling:** same path as gravel — UV from world XZ metres. One authored repeat length (client constant; may differ from gravel, e.g. ~1–2 m per tile).
- **Tint:** `FootKind::Cement.albedo()` multiplies the sample. Prefer `[1, 1, 1, 1]` when the texture owns the grade (same lesson as gravel after **083**); do not keep the old flat grey as a bake-in tint.
- **Mips + filter:** same as **083** (full mip chain; linear min/mag + mip; wrap repeat).
- **Geometry:** pads stay thin unlit slabs above the gravel ground (draw order / pad height unchanged). Top face tiles; sides may keep trivial UVs.
- Non-cement pads (grass, wet cement, optional gravel) unchanged — flat kind albedos.
- Gravel ground continues to use `gravel.albedo`.
- Landmark, boxes, ramp unchanged.
- Foot sampling and SFX unchanged.

## Out of scope

| Out |
| --- |
| Wet cement / grass / solid-top textures |
| Shared “surface material” table beyond kind → albedo + optional tex (keep the **083** pattern: one asset per textured kind) |
| Normal maps, specular, PBR, shadows, lit map path |
| New foot kinds, collide/support changes |
| `look.md` / yard fiction, foam projectiles, HUD |

## Acceptance

- Map **a** cement pads show tiled cement grit, not flat grey rectangles, from standing eye height.
- Texture repeats in world metres across each pad; no single-stamp stretch across a pad footprint.
- Mips keep distant pads from sparkling worse than gravel / kit albedos.
- `FootKind::Cement` remains the plant voice on those pads; look and sound stay the same kind.
- Grass / wet cement pads still flat colours; gravel ground still textured.
- Missing / failed cement albedo load fails soft: cement pads may fall back to flat cement albedo — loco and plants still work.
- HiDPI / MSAA / screenshot paths (**009** / **004**) still apply to the presented frame.
