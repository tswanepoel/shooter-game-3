# Feature 083 - Gravel ground albedo texture

**082** made the default yard floor a continuous gravel slab, but paint is still a flat kind albedo — one big brown square. This feature gives **gravel** its first real material: a seamless albedo texture, world-metre tiling, and mip filtering so the floor reads as gravel up close and at distance.

Client present only. Same foot kind id as **070** / **082** (voice + look stay one noun). No other surface textures yet. No yard fiction doc.

Depends on **082** (gravel ground slab + kind→albedo table), **010** (cook packs), **009** (albedo mips / filter).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `gravel.jpg` / `gravel.png` | `assets/source/materials/` | Seamless albedo; cook prefers `.jpg` / `.jpeg` then `.png`; asset id `gravel.albedo` |

Cook packs it into pack id `maps-a` beside `map-a.def` as asset id `gravel.albedo` (bytes as-is — thin cook; loaders decode PNG or JPEG by magic).

## Present

- **Gravel ground** (map **a** `ground` slab) samples `gravel.albedo` instead of a solid colour fill.
- **World tiling:** UV from world XZ metres. One authored repeat length (client constant, e.g. ~1–2 m per tile) so a 24×24 m yard repeats the grit — does not stretch one stamp across the whole square.
- **Tint:** keep `FootKind::Gravel.albedo()` as a multiply on the sample (same kit pattern: `tex * base_color`), so the shared kind colour still owns the grade.
- **Mips + filter:** full mip chain on upload; linear min/mag + linear mip; wrap repeat on U/V (**009** quality path).
- **Geometry:** ground stays the thin unlit slab. Prim UVs must be authored from world XZ (today’s `box_prim` zeros UVs — ground needs a path that sets them). Side faces of the thin slab may keep trivial UVs; the **top** face is what must tile correctly.
- Explicit gravel `foot_patches` (if any) may share the same albedo + tiling; not required for acceptance while map **a** has none.
- Cement / wet cement / grass pads stay flat kind albedos.
- Landmark, boxes, ramp unchanged.
- Foot sampling and SFX unchanged.

## Out of scope

| Out |
| --- |
| Cement / wet cement / grass / solid-top textures |
| Normal maps, specular, PBR, shadows |
| Lit map path (key+ambient on ground) — flat textured unlit is enough |
| New foot kinds, collide/support changes |
| `look.md` / yard fiction, foam projectiles, HUD |

## Acceptance

- Map **a** gravel ground shows tiled grit, not a flat brown field, from standing eye height and when looking across the yard.
- Texture repeats in world metres; no single-stamp stretch across the full footprint.
- Mips keep distant ground from sparkling / shimmering worse than kit albedos.
- `FootKind::Gravel` remains the unmarked plant voice; look and sound stay the same kind.
- Override pads (cement / grass / wet cement) still flat colours on top of textured gravel.
- Missing / failed gravel albedo load fails soft: ground may fall back to flat gravel albedo (or map fail as today) — loco and plants still work.
- HiDPI / MSAA / screenshot paths (**009** / **004**) still apply to the presented frame.
