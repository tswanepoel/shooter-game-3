# Feature 066 - Map solids (boxes, ramp, support)

**Map** `a` gains crude collide solids: axis-aligned boxes at mixed heights and one ramp. Sim owns blocking and support — the [figure](../concepts.md#figure) no longer lives on a hard `y = 0` plane. Grounded means soles on a support surface (floor, box top, or ramp), including at `y > 0`; [jump](../concepts.md#jump) / [air](../concepts.md#air) / land use that local height. Low boxes are step-up (walk onto, no jump); higher ones need jump or the ramp. Stairs are not a primitive — a short stack of low boxes is enough if wanted. Client draws the same solids; no second collide mesh. Projectile–world collide is out of scope.

Depends on **019**, **064**, [map](../concepts.md#map), [stand](../concepts.md#stand) / [walk](../concepts.md#walk) / [air](../concepts.md#air) / [jump](../concepts.md#jump) in [concepts](../concepts.md).

## Acceptance criteria

- Figure cannot walk through solid volumes on **map** `a`.
- Standing and landing use local support height, not global `y = 0`.
- Low box: walk onto without jump; higher box: jump (or ramp) only.
- Ramp: continuous incline the figure can walk up and stand on.
- Cooked map def carries the solids; host and client share the same set.
- Client present matches those solids; does not invent a parallel collide world.
