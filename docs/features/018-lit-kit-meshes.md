# Feature 018 - Lit kit meshes

Characters and blasters draw full-bright from albedo alone (008 / 009 / 011). There is no light response. This feature adds the first lighting path: kit meshes take form from a simple scene light setup.

Flat albedo remains the base paint. Lighting multiplies that paint. No new art, no material maps, no shadows.

## Philosophy

- **Basic first.** Diffuse + ambient only. Specular, PBR, multi-light arrays, and shadows come later if needed.
- **One path for every kit body and gun.** Self and debug lineup share the same lit presentation. No special unlit self.
- **Form over full-bright.** Default lights may leave unlit sides moderately darker than today’s flat look when that reads clearer. Still readable; not crushed black.
- **Present only.** Light constants live on the client. Sim does not own lights in this feature.

## Lights

- **One directional key** (fixed direction and colour/intensity) plus **ambient fill**.
- No point/spot array, no warehouse grid, no dynamic lights (muzzle flash, pickups).
- Defaults are client constants. Debug cvars to tweak key direction and intensities are welcome; not required for acceptance.

## Shading

- **Matte diffuse:** albedo × (ambient + key × max(N·L, 0)), or a mild wrap (half-Lambert) if hard limbs look too harsh.
- Kit meshes upload **normals** from the glTF (already authored; unused today) and transform them with the same pose matrices as positions.
- Blaster materials stay **double-sided**; backfaces flip or otherwise correct `N` so lighting does not invert.
- Albedo sampling and atlas wrap stay as today (008 / 009). Lighting is a multiply on that result, not a material rewrite.
- Solid debug draws (muzzle markers, reticle) and the ground **grid** stay unlit. Only character and blaster kit batches use the lit path.

## Scope

| In | Out |
|----|-----|
| Self body + blaster | Cast shadows |
| Blaster lineup characters + blasters | Specular / PBR / IBL |
| Normals on kit GPU path | Light list / many point lights |
| Key + ambient | Floor, room shell, light fixtures as geometry |

## Acceptance criteria

- Kit character and blaster meshes (self and lineup) respond to a directional key: lit and unlit sides read as form, not flat full-bright.
- Ambient fill keeps unlit sides moderately darker than full-bright but still readable under default constants.
- One shared lit path for all those draws; no per-context unlit exception for self or lineup.
- Grid, reticle, and muzzle markers remain unlit.
- No cast shadows. No specular highlight requirement. No multi-light array.
- HiDPI, MSAA, mips, and screenshot paths (004 / 009) still apply to the presented frame.
- Kit load rules and albedos (008 / 011) unchanged aside from using normals and the lit multiply.
