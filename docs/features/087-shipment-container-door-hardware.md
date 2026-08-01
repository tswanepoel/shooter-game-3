# Feature 087 - Shipment container door hardware

**086** gave the shipment container distinct side and door albedos, but left the ends looking like undecorated panels. This feature makes both ends read as fully closed, functional intermodal-container door assemblies.

Client present only. The shipment container remains one simulation `MapBox`; its collide, support, pose, and extents do not change.

Depends on **086** (shipment-container face UVs and side / door albedos), **066** (solid collision), and **083** (unlit solid upload).

## End assignment

- Local **+Z** and **−Z** both use `container-door.albedo` with the two-leaf door UV layout.
- Long walls and lids retain their **086** side-albedo assignments.

## Albedo scale

- The door image is one complete door leaf, not a corrugation tile.
- It maps once over the full container height with clamped V, so no false mirrored shadow appears at a vertical repeat.
- It repeats exactly twice across the container width: one image on the left leaf and one on the right.
- The side albedo scales independently so exactly two side tiles stacked vertically equal the full door / container height.

## Closed doors (both ends)

Present-only geometry sits just beyond each door surface:

- Painted perimeter header, sill, and side posts on both ends.
- Matching painted top / bottom rails and corner posts around both long walls.
- Black perimeter gasket tight against the painted frame, plus the centre meeting seal.
- Four vertical locking rods, two per leaf.
- Top and bottom cam keepers for every rod.
- One operating linkage per leaf between outer and inner rods (no inward stub handles).
- Four small painted external hinge assemblies per leaf, close to the outer frame.
- A painted latch cover centred at mid-height across both closed leaves.

The rods and hinge pins use low-sided cylinders. Frames, keepers, hinge leaves, and seals use small box solids. Geometry is grouped into three unlit batches by material colour: painted frame and hinges, black gasket, and galvanized locking hardware.

## Out of scope

- Opening, hinges as simulation joints, an interior, or cargo.
- Collision for individual door hardware.
- Owner codes, weights, approval plates, logos, or other writing.
- Bolt seals, padlocks, rust simulation, normals, roughness, or PBR.

## Acceptance

- Both container ends read as closed door assemblies.
- Each door leaf shows one complete door image without vertical mirroring.
- Two stacked side tiles equal the height of one complete door.
- From ordinary standing distance, each end clearly shows two sealed leaves, four inset locking rods, keepers, per-leaf linkages, four hinges per leaf, and a centred painted latch cover.
- The painted frame continues coherently around both long sides and meets flush at the outer corners.
- All hardware remains closed and follows the shipment container pose.
- The existing shipment-container AABB remains the sole collide / support shape.
- Missing albedos still fail soft to the flat container while the closed hardware remains visible.
