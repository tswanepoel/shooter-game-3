# Feature 092 - Map a train collide (cargo-jump escape)

**091** parked a present-only freight consist and staged an unloaded lumber pile south of a mid flatbed. This feature turns that layout into a corridor block: the train and ground cargo enter `MapWorld`, and the only practical path onto the empty flatbeds is a jump from the half-buried lumber pile.

Client builds the same kit AABBs the present uses for unit roots; sim owns blocking and support (**066**). No motion, schedules, or rail collide.

Depends on **091** (consist + ground cargo pose), **066** (map solids), **090** (corridor), Kenney Train Kit (CC0).

## Intent

| Aim | Meaning |
| --- | --- |
| Role | Train blocks the south corridor; cargo is the step onto the flatbeds |
| Cargo visual | Two-layer lumber pile sunk ~50% under the ground so only the top layer protrudes |
| Cargo support | Protruding top is standable (`MapBox` top ≈ half kit height × scale) |
| Train collide | One AABB per unit + the ground cargo; flatbed tops are **deck** height (not stake tips) so a cargo jump can land |
| Escape beat | Ground → jump onto logs → jump onto mid empty flatbed; walking through the consist is blocked |
| Jump | Peak **1.1 m** so a ground hop misses the flatbed deck (~1.12 m); cargo top still clears |
| Rail | Still present-only (unchanged from **090**) |

## Collide volumes

Kit-space magic AABBs (scaled / yawed with the **091** roots):

| Piece | Top | Notes |
| --- | --- | --- |
| Loco **c**, tank, lumber car | Body / load roof | Tall block |
| Empty flatbed | Deck (~0.36 kit Y) | Stakes stay visual-only |
| Ground `lumber-cargo` | Half height above `y = 0` | `seat_y` buries the bottom layer |

Layout truth stays the map def (`train.*` / `ground_cargo.*`). Present does not invent a second placement; collide roots share `train_unit_roots` with the draw path. Along-track collide pads seal the visual `unit_gap` (ground support is point-sampled, so an open gap is walkable).

## Out of scope

- Rail / sleeper collide or support.
- Motion, boarding, hazards, SFX.
- Per-mesh triangle collide; stake-accurate flatbed collide.
- Server-authoritative map solids (same client `MapWorld` path as **066**).
- Changing consist order, yard solids, or the **089** light plate.

## Acceptance

- Figures cannot walk through the stationed train or the ground lumber pile.
- Ground cargo reads as a half-buried top layer; its top is standable.
- From that top, a normal jump can reach a mid empty flatbed deck; flatbed stakes do not define support height.
- Loco / tank / lumber car remain tall blocks (not the intended on-foot crossing).
- Rail corridor remains walk-through except where train / cargo boxes sit.
- Cooked map def + shared kit AABBs are the collide truth; no parallel client-only blockers.
