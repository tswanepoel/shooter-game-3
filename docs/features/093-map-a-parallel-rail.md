# Feature 093 - Map a parallel rail

**090** laid one present-only east–west rail on centerline `z = −8`. **091** / **092** parked the freight consist on that line. This feature adds a second parallel track north of the first so the corridor reads as a double cut with the empty twin toward the yard.

Client present + map def layout. Same present-only rule as **090** — no rail collide. Consist stays on the south track of the pair; ground cargo stays south of that home line.

Depends on **090** (rail atoms + corridor), **091** (consist pose), Kenney Train Kit (CC0).

## Intent

| Aim | Meaning |
| --- | --- |
| Role | Double east–west track across map **a** |
| South track | Existing **090** line at `z = −8` (train home) |
| North track | Parallel twin into the yard buffer |
| Spacing | ~3.2 m center-to-center (tight twin, slight gap past gauge) |
| Consist / cargo | Unchanged on the south/home track; cargo still south of home |
| Collide | Still none from either rail |

## Layout (world metres)

Reuse **090** run: full width `x ∈ [−24, 24]`, yaw `+π/2`, same `stride` / `scale`.

| Track | Centerline | Role |
| --- | --- | --- |
| South (home) | `z = −8` | Home for the **091** consist |
| North | `z = −4.8` | Empty parallel rail |

Inter-track gap (~3.2 m) sits north of the consist — slight clearance past scaled gauge (~2.4 m). The **091** ground lumber (`side_z ≈ −2.4` → world `z ≈ −10.4`) stays south of the home track, clear of both gauges.

Do not shift yard solids, pads, consist order, or invent switches / crossovers.

## Ownership / delivery

| Concern | Owner |
| --- | --- |
| Track centerlines + shared stride / scale / yaw / span | Map **a** def `rail.centerlines_z` (+ existing rail knobs) |
| Draw | Client present; same spline atoms / morning plate as **090** |
| Foot voice | Undrawn cement strip per centerline (same half-Z as **092**) |
| Collide / support | Unchanged; rails still add no `MapBox` |

## Out of scope

- Rail / sleeper collide or support.
- Second consist, motion, switches, crossovers, grades.
- Moving the train onto the north track.
- Relocating yard cover or the **089** light plate.
- Station architecture.

## Acceptance

- Two parallel straight rails run the full east–west width: home at `z = −8`, twin north at `z = −4.8`.
- Same atoms / tiling / scale as **090** on both tracks.
- Stationed train remains on the home (`z = −8`) centerline; ground cargo remains south of that line.
- Figures walk both tracks with no new rail collide / support.
- Layout truth is the cooked map def (`rail.centerlines_z`); present does not invent a second placement.
- Lineup / non-map paths unchanged.
