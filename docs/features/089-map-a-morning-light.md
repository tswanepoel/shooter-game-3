# Feature 089 - Map a morning light

**018** / **088** light kits and map solids with one shared directional key plus ambient. Those defaults read as a high, near-neutral day fill — peak `ambient + key` sits above 1.0, so the plate is already full and any future local lamp has nowhere soft to land. The clear behind map **a** is still near-black `(0.05, 0.06, 0.08)`, so a lit morning yard sits in a night hole.

This feature gives map **a** its own **morning light plate**: underexposed early-morning daylight with headroom for later artificial lights.

Client present only. Sim, collide, support, albedos, and geometry do not change.

Depends on **018** (key + ambient frame uniforms), **088** (lit map solids), **066** / **064** (map **a** present).

## Intent

Map **a** should feel like **good-morning freshness** — alert, readable, daytime — without washing the frame to full-bright noon.

| Aim | Meaning |
| --- | --- |
| Time | ~1 hour after sunrise; sun ~8–15° elev; mid-latitude equinox ≈ 06:45–07:30 local solar |
| Weather | Fair morning: cool dry air, good visibility, soft horizon haze only, light broken cloud / thin cirrus |
| Mood | Casino-style perpetual morning — stable, slightly cool, never tired — not blue-hour gloom and not noon wash |
| Shadows | Keep directional form (N·L / half-Lambert). No cast shadow maps in this feature |
| Headroom | Peak natural `ambient + key` stays under full plate so a few soft locals can still earn their keep later |
| Lamps | Still-on practicals must feel believable (world not noon-bright) and useful (world not so dim that every lamp is mandatory fill) |

Art direction name: **clear early morning with light broken cloud.**

## Ownership

The plate is **map a’s**, not a new global default.

| Context | Lights / clear |
| --- | --- |
| Match present with map **a** ready | Map **a** morning plate on all lit draws in that frame (map solids, kits, drops, corpses) + map **a** clear colour |
| Lineup / non-map debug | Keep **018** defaults |
| Future maps | Own their own plates; do not silently inherit map **a**’s |

Key, ambient, and light direction stay client present constants (or a small map-a present struct). Sim does not own lights. `map-a.json` does not gain a light block in this feature.

## Light plate

Same **018** path: one directional key + ambient fill, half-Lambert wrap. No new shader terms.

### Budget

| Quantity | Target |
| --- | --- |
| Ambient (unlit face) | ~0.35–0.45 — daytime readable, not crushed |
| Key at N·L wrap = 1 | ~0.30–0.45 — sculpts form, does not own the frame |
| Peak `ambient + key` | **~0.75–0.85** — leaves ~15–25% multiply headroom for later locals |
| Today (**018** defaults) | ambient `0.42` + key `0.70` → peak **~1.12** (full / over) |

### Draft constants (eye-tune in range)

Starting values inside the bands — adjust for read, keep the budget:

| Uniform | Draft | Notes |
| --- | --- | --- |
| `light_dir` (toward key) | `(0.82, 0.22, 0.38)` then normalize | Low elevation (~12°); from +X / +Z so the container at `x ≈ 8` takes a side key, lids catch soft top, yard gets long soft form |
| `key_color` | `(0.40, 0.37, 0.32)` | Soft warm-leaning morning sun — not golden-hour fire |
| `ambient` | `(0.36, 0.39, 0.45)` | Cool sky fill — freshness without blue-hour navy |
| Peak (draft) | ~0.40 + 0.36 = **~0.76** at wrap = 1 | Inside budget |

Colour split: **cool dome fill + soft warm low key**. That is the morning signature; do not flatten both to the same neutral grey.

### Clear colour (sky stand-in)

No sky mesh / cubemap in this feature. While map **a** is drawn, replace the near-black clear with a flat morning stand-in:

| | Draft RGB |
| --- | --- |
| Map **a** clear | `(0.30, 0.38, 0.48)` — cool desaturated upper-sky blue, underexposed |
| Non-map clear | Unchanged `(0.05, 0.06, 0.08)` |

This is a temporary backdrop, not the broken-cloud sky-drop. Horizon warmth and cloud shapes are out (see below).

## Out of scope

- Cast shadows, contact shadows, AO.
- Point / spot / local artificial lights (budget only; locals are a later feature).
- Sky mesh, cubemap, horizon band, clouds, fog volumes.
- Per-map light block in `map-a.json` / cooked def.
- Changing **018** lineup defaults.
- Re-tinting albedos authored under the old plate.
- Tonemap / exposure curve changes.
- Weather particles, wetness, or time-of-day animation.

## Acceptance

- With map **a** loaded, the yard reads as early morning daylight: directional form present, unlit sides still readable, frame not noon-washed.
- Peak natural lighting on a fully lit face stays visibly under full-bright relative to today’s **018** plate (headroom preserved).
- Cool ambient vs warm-leaning key is noticeable on white-ish / neutral surfaces (container ribs, cement pads) without turning the map blue or orange.
- Clear behind the map is a cool morning blue, not near-black night.
- Lit map solids (**088**) and kits (**018**) in the match share the same map **a** plate.
- Lineup / non-map debug still use **018** defaults and the existing clear.
- Debug muzzle markers stay unlit.
- Collide / support / albedos / geometry unchanged.
- A still-on practical *would* read as believable and useful on this plate (no practicals shipped in this feature — judgment call on the empty plate).
