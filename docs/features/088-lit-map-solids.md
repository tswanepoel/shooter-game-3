# Feature 088 - Lit map solids

**087** built closed door assemblies out of frames, rods, keepers, and hinges, but every one of them rendered as a single flat colour. Map solids uploaded with the unlit material flag, so the shader returned `albedo × tint` and discarded the vertex normal. A cuboid whose top, side, and front faces emit identical RGB has no visible edges — the whole assembly read as one bright silhouette with no distinguishable parts.

This feature puts map solids on the existing key + ambient pass so their geometry reads as form.

Client present only. No new lights, no shadow maps, no new albedos. Sim collide, support, pose, and extents do not change.

Depends on **018** (lit kit shading, key + ambient frame uniforms), **087** (door hardware geometry), **083** / **086** (textured solid upload), **066** (map solids).

## Shading select

Solid uploads take an explicit shading choice instead of hard-coding the unlit flag:

| Shading | Material flag | Used by |
| --- | --- | --- |
| `Lit` | `flags.x = 1` | Ground, foot pads, shipment container skin, door hardware, boxes, ramp |
| `Unlit` | `flags.x = 0` | Debug muzzle markers |

Both `upload_solid_batch` and `upload_textured_solid_batch` carry the choice, so flat-colour and textured map solids light the same way. Debug overlays stay deliberately flat — a marker is a readout, not a thing in the world.

## What lights them

No new lighting: map solids join the **018** path already bound for kits. The map's own `UnlitMeshGpu` frame uniforms already carried `light_dir`, `key_color`, and `ambient`; only the material flag was withholding them. Normals were already authored on every primitive and already transformed by `merge_transformed_prims`, so no geometry changes.

Shading stays half-Lambert wrap so blocky solids soften rather than cliff at the terminator, matching kits.

## Face values

With the key slightly elevated front-right, one paint colour now separates by facing — up-facing near full key, inboard faces near ambient only. Locking rods gain a cylindrical gradient, corner posts separate from the door skin behind them, and lids read brighter than walls.

Flat ground and pads face up, so they keep roughly their previous brightness — the floor does not visibly change.

## Out of scope

- Shadow maps, contact shadows, or ambient occlusion.
- Extra lights, sky, or horizon.
- Per-material roughness, specular, normals, or PBR.
- Re-tinting albedos authored under flat light.
- New container add-ons or markings.

## Acceptance

- Door hardware reads as separate parts at standing distance: frame posts, rods, keepers, hinges, and latch cover are individually distinguishable.
- The same paint colour shows different values on differently facing faces.
- Container walls, lids, and ends separate from each other without new geometry.
- Debug muzzle markers remain flat and unlit.
- Ground and pad brightness is substantially unchanged.
- Collide / support behaviour is unchanged.
