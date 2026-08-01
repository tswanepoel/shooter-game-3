# Feature 085 - Grass pad albedo texture

**084** textured cement pads. Grass still paints a flat kind albedo — a green rectangle on grit. This feature gives **grass** the same material treatment: a seamless short-turf albedo, world-metre tiling, and mip filtering on map **a** grass `foot_patches`.

Client present only. Same foot kind id as **072** / **082** (voice + look stay one noun). Wet cement stays flat. No blade cards, no yard fiction doc.

Depends on **083** (textured solid upload, world-XZ UVs, PNG/JPEG decode, mips), **084** (pad textured path), **082** (kind→albedo table + pads), **010** (cook packs), **009** (albedo mips / filter).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `grass.jpg` / `grass.png` | `assets/source/materials/` | Seamless albedo; cook prefers `.jpg` / `.jpeg` then `.png`; asset id `grass.albedo` |

Cook packs it into pack id `maps-a` beside `map-a.def` / `gravel.albedo` / `cement.albedo` as asset id `grass.albedo` (bytes as-is — thin cook; loaders decode PNG or JPEG by magic).

### What the renderer needs (for the PNG)

| Need | Spec |
| --- | --- |
| Role | Diffuse / albedo only (unlit `tex * base_color`) |
| Content | Well-cut sports-field / rugby-pitch turf — short, even, fine grain; subtle mow stripe OK |
| Alpha | Opaque (RGB; A ignored or 1) |
| Seam | Tileable on both axes (no hard edge at wrap) |
| Resolution | Power-of-two preferred (e.g. 512² or 1024²); one tile ≈ authored metres below |
| Contrast | Moderate — turf readable at standing eye height without sparkling after mips |
| Not needed | Tall blades, wild meadow, flowers, normals, roughness, AO, height, logos, perspective photos |

## Present

- **Grass `foot_patches`** on map **a** sample `grass.albedo` instead of a solid colour fill.
- **World tiling:** same path as gravel / cement — UV from world XZ metres. One authored repeat length (client constant; may differ, e.g. ~1–2 m per tile).
- **Tint:** `FootKind::Grass.albedo()` multiplies the sample. Prefer `[1, 1, 1, 1]` when the texture owns the grade (same lesson as gravel / cement); do not keep the old flat green as a bake-in tint.
- **Mips + filter:** same as **083** (full mip chain; linear min/mag + mip; wrap repeat).
- **Geometry:** pads stay thin unlit slabs above the gravel ground (draw order / pad height unchanged). Top face tiles; sides may keep trivial UVs.
- Wet cement pads unchanged — flat kind albedo.
- Gravel ground / cement pads continue to use their albedos.
- Landmark, boxes, ramp unchanged.
- Foot sampling and SFX unchanged.

## Out of scope

| Out |
| --- |
| Wet cement texture / puddles / coverage tiles |
| Grass blade cards, instances, wind, LOD stands |
| Shared “surface material” table beyond kind → albedo + optional tex (keep the **083** pattern: one asset per textured kind) |
| Normal maps, specular, PBR, shadows, lit map path |
| New foot kinds, collide/support changes |
| `look.md` / yard fiction, foam projectiles, HUD |

## Acceptance

- Map **a** grass pads show tiled short turf, not flat green rectangles, from standing eye height.
- Texture repeats in world metres across each pad; no single-stamp stretch across a pad footprint.
- Mips keep distant pads from sparkling worse than gravel / cement / kit albedos.
- `FootKind::Grass` remains the plant voice on those pads; look and sound stay the same kind.
- Wet cement pads still flat colours; gravel ground and cement pads still textured.
- Missing / failed grass albedo load fails soft: grass pads may fall back to flat grass albedo — loco and plants still work.
- HiDPI / MSAA / screenshot paths (**009** / **004**) still apply to the presented frame.
