# Feature 082 - Default gravel ground present

Unmarked ground already **plants** gravel (**070**). The scene still reads as void + debug grid with floating foot pads. This feature makes the default [foot](../concepts.md#figure) surface **visible**: a present-only gravel ground under map **a**, one shared kind→albedo table for all foot kinds, and the product grid no longer standing in for a floor.

Client present only — not sim collide, not net. No new foot kinds. No solid-top surfaces. No lit map path. No yard fiction doc.

Depends on **070** / **072** / **073** (foot patches + voices), **066** (map solids present), **064** (map **a** cook), **010** (cook packs).

## Map

`map-a.json` gains an authorable **ground** footprint on the XZ plane:

| Field | Meaning |
| --- | --- |
| `position` | Centre `[x, y, z]` — `y` ignored for draw (slab sits on `y = 0`) |
| `half_extents` | `[half_x, half_z]` playable yard size |

Ground paint is always **gravel** (the unmarked default). Override pads stay in `foot_patches[]` (`cement` / `wet_cement` / `grass` / optional explicit `gravel`).

Map **a** ships a ground rect that covers the spawn yard and today’s pads / solids with modest margin. Redundant explicit gravel pads that only existed to paint unmarked ground may be removed; cement / grass / wet cement pads stay.

## Present

- One client table maps each foot kind to its albedo (gravel, cement, wet cement, grass). Foot pads and the ground slab both read from that table — no one-off colour constants per draw site.
- Ground draws as a thin unlit slab (same present style as today’s pads), under override pads in draw order.
- Foot sampling unchanged: XZ patch stack, outside all patches → gravel. Ground does not enter `FootSurfaces` as a patch and does not enter `MapWorld`.
- When map ground is ready, the product scene defaults the debug grid **off**. With `debug-tools`, `draw.grid` remains the toggle. Release / no map: no requirement to invent a new grid policy beyond “grid is not the floor.”

## Out of scope

| Out |
| --- |
| New foot kinds or SFX banks |
| Surface kind on box / ramp / landmark tops |
| Textured albedo, lit map batches, shadows |
| `look.md` / yard fiction, foam projectiles, HUD chrome |
| Changing plant timing, gains, or dual-land |

## Acceptance

- Map **a** shows a continuous gravel ground over the authored footprint; unmarked standing still plants gravel.
- Cement / wet cement / grass pads still override look and voice on top of that ground.
- Kind albedos come from one shared table for ground + pads.
- Ground is present-only: no new collide / support; `MapWorld` unchanged.
- Product match with map ready does not rely on the debug grid as the visible floor; debug can still show the grid when enabled.
- Missing / failed map load still fails soft as today (no floor invent from thin air).
