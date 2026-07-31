# Feature 072 - Grass footstep SFX

Local foot plants pick **grass** from present-only map patches under the figure, beside **070** gravel / cement. Outside all patches still defaults to gravel. Client present only — not sim collide, not net.

Depends on **070** (foot patches / plant timing), **010** (cook packs).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `grass-step1.wav` | `assets/source/sfx/grass-step1.wav` | Grass plant variant |
| `grass-step2.wav` | `assets/source/sfx/grass-step2.wav` | Grass plant variant |
| `grass-step3.wav` | `assets/source/sfx/grass-step3.wav` | Grass plant variant |

## Map

`map-a.json` `foot_patches[]` gains `kind` `grass`. Map **a** ships grass pads in the spawn radius so the voice is authored and visible next to cement and gravel.

## Behaviour

- Cook packs the three grass variants into pack id `sfx` beside gravel / cement / bang.
- Plant timing, gains, dual-land stagger, and no-repeat variant pick match **070**; grass keeps its own last-variant index.
- Surface is still sampled at the local figure XZ each plant (and land).

## Acceptance

- Stepping on a grass pad plays grass; cement and gravel pads keep their voices.
- Unmarked ground still plants gravel.
- Missing / failed SFX load still does not block loco.
