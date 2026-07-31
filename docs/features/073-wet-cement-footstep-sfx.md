# Feature 073 - Wet cement footstep SFX

Local foot plants pick **wet cement** from present-only map patches under the figure, beside **070** gravel / cement and **072** grass. Outside all patches still defaults to gravel. Client present only — not sim collide, not net.

Depends on **070** (foot patches / plant timing), **010** (cook packs).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `wet-cement-step1.wav` | `assets/source/sfx/wet-cement-step1.wav` | Wet cement plant variant |
| `wet-cement-step2.wav` | `assets/source/sfx/wet-cement-step2.wav` | Wet cement plant variant |
| `wet-cement-step3.wav` | `assets/source/sfx/wet-cement-step3.wav` | Wet cement plant variant |

## Map

`map-a.json` `foot_patches[]` gains `kind` `wet_cement`. Map **a** ships wet cement pads in the spawn radius so the voice is authored and visible next to cement, gravel, and grass.

## Behaviour

- Cook packs the three wet cement variants into pack id `sfx` beside gravel / cement / grass / bang.
- Plant timing, gains, dual-land stagger, and no-repeat variant pick match **070**; wet cement keeps its own last-variant index.
- Surface is still sampled at the local figure XZ each plant (and land).

## Acceptance

- Stepping on a wet cement pad plays wet cement; cement, gravel, and grass pads keep their voices.
- Unmarked ground still plants gravel.
- Missing / failed SFX load still does not block loco.
