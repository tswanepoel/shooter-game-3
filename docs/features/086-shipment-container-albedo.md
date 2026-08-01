# Feature 086 - Shipment container albedo

**085** finished pad albedos (gravel / cement / grass). Map **a** still paints its big prop as a flat blue-grey box named `landmark`. This feature gives that prop a **real name** and the same material treatment as the pads: corrugated-metal albedos on the **shipment container** only.

Client present + map def rename. Collide stays one AABB (066) — no second mesh. Boxes and ramp stay flat colours.

Depends on **064** (map **a** + container-sized solid), **066** (collide boxes), **083** (textured solid upload, PNG/JPEG decode, mips), **010** (cook packs), **009** (albedo mips / filter).

## Name

| Was | Becomes |
| --- | --- |
| `landmark` (map-a.def key, draw label, docs) | `shipment_container` |
| Informal “landmark colour box” | Shipment container (ISO-ish 20ft footprint already in map **a**) |

Sim `MapWorld` still receives it as an ordinary `MapBox` (first collide box). No new sim type.

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `container-side.jpg` / `.png` | `assets/source/materials/` | Vertical-rib side / roof metal; cook prefers `.jpg` / `.jpeg` then `.png`; asset id `container-side.albedo` |
| `container-door.jpg` / `.png` | `assets/source/materials/` | Horizontal-frame end / door metal; same cook preference; asset id `container-door.albedo` |

Cook packs both into pack id `maps-a` beside `map-a.def` / pad albedos (bytes as-is — thin cook; loaders decode PNG or JPEG by magic).

### What the renderer needs (for the PNGs)

| Need | Spec |
| --- | --- |
| Role | Diffuse / albedo only (unlit `tex * base_color`) |
| Side content | Corrugated / ribbed container wall — readable vertical ribs at standing eye height; weather / dirt OK |
| Door content | End / door panel with horizontal frame lines; weather / dirt OK |
| Alpha | Opaque (RGB; A ignored or 1) |
| Seam | Horizontal repeat; vertical non-seam is hidden with mirrored V repeat (both albedos) |
| Resolution | Power-of-two preferred (e.g. 512² or 1024²) |
| Contrast | Moderate — ribs read without sparkling after mips |
| Not needed | Unique door logos as non-tiling stamps, normals, roughness, AO, height, PBR sets |

## Present

- Map **a** def key is `shipment_container` (not `landmark`). Same `position` / `half_extents` as today’s solid.
- Container draws as **three** textured batches sharing one AABB pose:
  - **Sides (±X):** `container-side.albedo`
  - **Ends (±Z):** `container-door.albedo`
  - **Lids (±Y):** `container-side.albedo` with UVs transposed 90° so grooves run **width-wise** (±X)
- **Face UVs (not world-XZ):** pad tiling projects XZ — wrong on vertical walls. Assign UVs per box face from local metres on that face so corrugation runs consistently. Authored metres-per-tile client constant (e.g. ~1–2 m).
- **Vertical mirror:** retain each upper tile as authored; mirror it into the tile immediately below (`MirrorRepeat` on V). Horizontal U continues to repeat normally.
- **Tint:** white `[1, 1, 1, 1]` when the texture owns the grade (same lesson as gravel / cement / grass).
- **Mips + filter:** same as **083** (full mip chain; linear min/mag + mip; wrap repeat / mirrored V).
- Draw / collide pose and extents unchanged.
- Boxes, ramp, foot pads, gravel ground unchanged.
- Missing / failed side or door albedo fails soft: flat fallback colour for the whole container — collide and loco still work.

## Out of scope

| Out |
| --- |
| Texturing parkour boxes or the ramp |
| Unique door / logo stamps that break tiling |
| Normal maps, specular, PBR, shadows, lit map path |
| Opening doors, interiors, climbable ladders as art |
| Wet cement / grass blades / yard fiction |

## Acceptance

- Map **a** JSON / cooked `map-a.def` uses `shipment_container`, not `landmark`.
- The container reads as corrugated metal from standing eye height, not a flat grey AABB.
- Long walls show upright vertical ribbing; ends show door / frame metal; roof grooves run width-wise.
- Mips keep distant views from sparkling worse than pad / kit albedos.
- Collide / support match the same AABB as before (066 behaviour unchanged).
- Boxes and ramp still flat colours.
- Missing / failed container albedo load fails soft.
- HiDPI / MSAA / screenshot paths (**009** / **004**) still apply to the presented frame.
