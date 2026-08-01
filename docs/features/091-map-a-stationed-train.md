# Feature 091 - Map a stationed train

**090** laid a present-only east–west rail across map **a** and left rolling stock out. This feature parks a static freight consist on that corridor and stages a small unloaded-lumber beat beside it so the line reads as a working cut.

Client present + map def layout. No motion, schedules, or train collide — figures still walk through the corridor (and through the train / cargo meshes) as with the rail.

Depends on **090** (rail corridor + train kit pack path), **010** (cook / packs), **088** / **089** (lit map solids + morning plate), Kenney Train Kit (CC0).

## Intent

| Aim | Meaning |
| --- | --- |
| Role | One parked freight train on the **090** centerline |
| Place | Consist on the corridor, midpoint nudged ~half a car west of centre |
| Facing | East — loco nose toward world **+X** |
| Story | Empty flatbeds mid-consist; lumber still on a rear car; a lumber pile on the ground south of a mid empty flatbed (unloaded) |
| Scale | Rail and rolling stock scale independently (track slightly larger than stock) |
| Collide | None from the train or ground cargo (same present-only rule as the rail) |

## Consist (Kenney Train Kit)

Front → back (east → west):

| Order | Asset | Role |
| --- | --- | --- |
| 1 | `train-locomotive-c` | Steam loco **c** (nose = kit **+Z**) |
| 2 | `train-carriage-flatbed` | Empty flatbed |
| 3 | `train-carriage-flatbed` | Empty flatbed (unload beat) |
| 4 | `train-carriage-lumber` | Flatbed with lumber still aboard |
| 5 | `train-carriage-tank` | Small tank at the rear (not `…-tank-large`) |

No couplers. Gaps between units are authored on the map def. Kit facts (axes, colormap) live in `assets/source/train/README.md`. Shared `colormap.png`. Authored kit units at **×1**; map def applies separate `rail.scale` / `train.scale`.

Ground pile: `lumber-cargo` (cargo node stripped from the lumber carriage), beside a mid empty flatbed, south of the rails, lightly yawed. Pose knobs live on the map def (`ground_cargo`) — not free client constants.

## Layout (world metres)

Reuse the **090** corridor: centerline **`z = −8`**, track-forward world **+X**, yaw **`+π/2`** so kit **+Z → world +X**.

| Parameter | Value |
| --- | --- |
| Centerline | `z = −8` (same as `rail.centerline_z`) |
| Consist midpoint | Full front–back span centred at **`x ≈ −2.7`** (~half carriage west of span centre) |
| Facing | East: loco at the east end, cars trail west, tank last |
| Ground cargo | South of the corridor (negative world **Z** from the centerline), next to a mid empty flatbed — yard north stays the clearer run |
| Vertical | Wheel / cargo seats tuned on the map def (`seat_y` knobs) |

Do not shift yard solids, pads, or invent station props. Rail tiling stays **090**’s (metal on gravel, wood on segment deck); only `rail.scale` / `stride` may differ from the toy kit ×1.

## Ownership / delivery

| Concern | Owner |
| --- | --- |
| Consist order, gaps, scales, seat / nudge / ground-cargo pose | Map **a** def `train` (+ `rail.scale` / `stride`) |
| Rolling-stock, lumber-cargo, shared colormap | Cooked pack **`maps-a`** |
| Draw | Client present; lit under the map **a** morning plate (**089**) |
| Collide / support | Unchanged; **train and ground cargo add no `MapBox` / ramp** |

## Out of scope

- Motion, animation, schedules, hazards, SFX, boarding.
- Train / cargo collide or support (including later jump-on-logs / flatbed escape — present layout only prepares the beat).
- Passenger couplers, `tank-large`, other loco sets.
- Curves, switches, second tracks, damaged stock.
- Station architecture, platforms, signs, furniture.
- Changing the **089** light plate or relocating the **090** yard cluster.

## Acceptance

- Consist on the **090** corridor: loco **c**, two empty flatbeds, lumber car, tank at the rear; gaps between units; no connectors.
- Mid-track, facing east; tank is the westmost car.
- A lumber pile sits on the gravel south of a mid empty flatbed, lightly angled.
- Rail and stock use the shared train colormap and the map **a** morning plate; track may read slightly larger than the cars.
- Figures still walk the corridor with no new collide / support from train or cargo.
- Layout truth is the cooked map def; present does not invent a second placement.
- Lineup / non-map paths unchanged.
