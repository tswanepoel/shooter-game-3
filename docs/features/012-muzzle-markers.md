# Feature 012 - Muzzle markers

Developers need debug **muzzle markers** on every held blaster in the lineup (011): a small **magenta ball** at each barrel exit so multi-barrel aim, grip placement, and scale can be checked against the metre grid. Presentation is client-only and rides the same lineup toggle.

**Pairing:** markers for each `blaster-{letter}` slot (same letter order as 011). Some blasters have more than one muzzle.

## Acceptance criteria

- Gated with the debug tools surface (003) and the blaster lineup (011). Markers appear only when the lineup is shown. Dev builds own the implementation; release builds strip debug tools. No new root command is required unless a separate show/hide proves useful later; the existing lineup entry point remains the toggle. Model facts live in the blaster kit README.
- **One or more muzzle points per blaster** (letter `a` … `r`). Count matches the kit table (most have one; multi-barrel weapons list every exit). Source of the recipe values: Kenney blaster glTF plus the historical `muzzlePoints` list for that weapon.
- **Offset space** matches **grip offsets** (011): **arm-attachment frame** — **`arm-right` local after `holding-right`**, in **character-kit / recipe units** — not world metres. The character root scale (character kit README) maps those units into the scene.
- **World placement:** each marker is the image of its offset under the same arm hierarchy used for the grip position (no separate blaster-local re-basing for this cue). Markers must sit at the barrel exits when grip, orientation, and scale are correct.
- **Presentation:** a solid **magenta** sphere per muzzle point (debug draw / unlit solid colour is fine). Size is small enough not to hide the gun, large enough to read clearly against the grid and body. Markers do not cast shadows or change lighting of the lineup.
- Feature 011 attachment contract (grip, 180° about Y, dual kit scales) remains unchanged; this feature only adds the muzzle cues on top of that held pose.

## Kit documentation

- Blasters, grip table, and muzzle table: `assets/source/blasters/README.md`
