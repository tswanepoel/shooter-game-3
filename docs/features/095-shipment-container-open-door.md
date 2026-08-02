# Feature 095 - Shipment container open door (CQC pocket)

**087** closed both ends as door assemblies over one solid `MapBox`. This feature opens **one** end so the interior becomes a walkable cul-de-sac — enter, fight, exit the same mouth — not a throughfare.

Client present + collide shell built from the same map-def root. Sim still owns blocking and support via ordinary `MapBox` volumes (**066**). Static open state only.

Depends on **087** (closed door hardware / end assignment), **086** (container albedos + face groups), **088** (lit solids), **066** (map solids), **064** (map **a** pose).

## Intent

| Aim | Meaning |
| --- | --- |
| Beat | One-door **pocket** for close-quarters encounters |
| Open end | Local **−Z** (`BoxFaceGroup::Front`) — south mouth toward the yard / box cluster |
| Closed end | Local **+Z** (`Rear`) stays a sealed door assembly (**087**) |
| Collide | Replace the single solid AABB with a **shell** (floor, roof, long walls, closed end, jambs); open mouth walkable |
| Present | No closed leaf draw on the open end; draw interior faces; one or both leaves swung open beside the jamb |
| Foot SFX | Interior floor stand → **steel** (SFX-only) |

Pose / outer half-extents stay the map-def `shipment_container` root. Present does not invent a second placement.

## Collide shell

Authored thin volumes in the container local frame (shared root with draw):

| Piece | Role |
| --- | --- |
| Floor | Standable strip; full outer length × width, thin height |
| Roof | Blocks standing through the lid; same footprint as floor |
| Long walls (±X) | Side skins; leave clear interior width for a figure |
| Closed end (+Z) | Full-height back wall |
| Jambs / sill / header | Frame the open mouth so the opening reads and blocks clipping the frame |
| Open leaf (optional) | Thin vertical slab beside the mouth if the swung door should block |

Wall / lid thickness is small vs outer half-extents so the clear tube is playable (~standing height, two figures tight). No hollow prim type — only `MapBox`es in `MapWorld`.

## Present

- Drop the closed two-leaf draw and latch cover on the **Front** end only.
- Keep **Rear** closed hardware and door albedo as today.
- Draw **interior** faces (floor, ceiling, inners of long walls and closed end) with the existing side / door albedos so the tube is not sky-through.
- Swing one or both Front leaves open (~90° about the outer hinge) as present geometry; reuse door albedo + frame paint where it helps read.
- Lit under **088** / **089**; no extra interior light. Ambient must still leave silhouettes readable inside the tube.
- Missing albedos still fail soft to flat colour; shell collide and loco still work.

## Out of scope

- Opening the rear end or cutting a second mouth (throughfare).
- Hinge joints, animation, interact-to-open, cargo props inside.
- Per-triangle collide; thickness matched to real ISO steel.
- Shadow maps, bounce fill, or a dedicated interior light.
- Relocating the container, boxes, ramp, or rail / train layout.
- Changing jump peak or loco size.

## Acceptance

- Figures can walk into the container from the south mouth and stand on the interior floor.
- Figures cannot walk through long walls, roof, floor, closed end, or jambs.
- The rear end still reads as a closed door assembly; the front reads as open (leaf/leaves swung, no sealed latch cover).
- Interior faces are drawn; looking in from the mouth does not show through to the sky or yard through missing inners.
- Outer footprint / root pose matches the cooked map def; collide shell is derived from that root, not a parallel placement.
- Footfalls inside voice **steel**.
- Lineup / non-map paths and the rest of map **a** solids unchanged.
