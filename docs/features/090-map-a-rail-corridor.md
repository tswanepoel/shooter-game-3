# Feature 090 - Map a rail corridor

Map **a** is a tight 24×24 m yard. A train line and (later) a station need room and a hard spatial split. This feature expands the plate, reserves the south third for a future station, and lays a straight present-only rail that cuts east–west across the full width.

No station buildings. No trains. Client present + map def layout. Sim collide for the rail is out — figures walk across the corridor freely.

Depends on **064** / **066** (map **a** def + solids), **010** (cook / packs), **088** / **089** (lit map solids + morning plate), Kenney Train Kit (CC0).

## Intent

| Aim | Meaning |
| --- | --- |
| Size | Ground footprint ~**2×** in X and ~**2×** in Z → **48×48 m** (`half_extents` **`[24, 24]`**) |
| Split | Rail is the divider: **⅓ south** / **⅔ north** |
| South ⅓ | Empty provision for a later station — no platform, canopy, or furniture in this feature |
| North ⅔ | Play yard — today’s cover and pads live here |
| Rail | Straight, full width, Kenney **spline** atoms only |

## Layout (world metres)

Ground centred at origin; `z ∈ [−24, 24]`, `x ∈ [−24, 24]`.

| Region | Span | Role |
| --- | --- | --- |
| South ⅓ | `z ∈ [−24, −8]` | Station apron (empty) |
| Rail corridor | centerline **`z = −8`**, gauge ~1 m about that line | Straight track, present only |
| North ⅔ | `z ∈ [−8, 24]` | Play yard |

Rail runs **along +X** from `x = −24` to `x = +24` (full ground width).

### Yard content move

Existing shipment container, boxes, ramp, and foot patches must sit entirely in the **north ⅔**, clear of the rail corridor band (keep a few metres of open gravel north of `z = −8` so the line reads as a cut, not clutter through cover). Translate / nudge the current cluster as a group; do not invent new cover in this feature.

## Track atoms (Kenney Train Kit)

Fundamental units only:

| Asset | Role |
| --- | --- |
| `spline-segment` | Two parallel metal rails |
| `spline-track` | Wooden sleeper under them |

Authoring kit facts (scale, axes, colormap) live in a source kit README under `assets/source/` (same pattern as characters / blasters). Authored units are **metres at ×1**. Shared `colormap.png`.

Placement:

- Tile along the centerline for the full 48 m run.
- **`spline-segment` (metal)** at **2×** frequency: one per half-`stride`, soles on the gravel top.
- **`spline-track` (wood)** at **1×** `stride`: centered on every other segment, soles on the segment **deck** (Y band below rail-head detail — not mesh `max_y`).
- Stride from the sleeper’s along-track extent (expect ~1 m); authored in the map def as `rail.stride`.
- Orient instances so the kit’s track-forward axis follows world **+X** (yaw as needed — confirm in the kit README).
- Gauge centred on `z = −8`.

Prefab `railroad-*` pieces are out of this feature (composites of the same atoms; optional later shortcut).

## Ownership / delivery

| Concern | Owner |
| --- | --- |
| Ground size, yard solid / pad poses | Cooked `map-a.def` (shared host/client as today) |
| Rail centerline, strides, instance list or generator inputs | Map **a** def (or a small rail block on that def) — layout truth with the map, not a free client constant |
| Rail meshes + colormap | Cooked pack (join **`maps-a`** or a same-cadence sibling loaded with the map) |
| Draw | Client present; lit under the map **a** morning plate (**089**) like other map kits |
| Collide / support | Unchanged set of boxes + ramp; **rail adds no `MapBox` / ramp** |

## Out of scope

- Station architecture, platforms, signs, furniture.
- Trains, motion, schedules, hazards, SFX.
- Curves, switches, grades, damaged variants.
- Rail collide / support (including sleeper tops as standables).
- Projectile–world collide.
- New play cover beyond relocating today’s cluster.
- Changing the **089** light plate (expanded ground still uses map **a** morning).

## Acceptance

- Ground present reads ~48×48 m (`half_extents` `[24, 24]`).
- A straight rail runs the full east–west width on centerline `z = −8`: metal segments at half-stride on the gravel, wooden tracks at full stride resting on segment tops, kit colormap.
- South of the rail (`z < −8`) is open apron — no station props, and no relocated yard solids/pads.
- North of the rail holds the existing cover and foot patches, clear of the corridor.
- Figures walk across the tracks with no new collide / support from the rail.
- Host and client share the same expanded ground + yard solids from the cooked def; rail present does not invent a parallel collide world.
- Lineup / non-map paths unchanged.
