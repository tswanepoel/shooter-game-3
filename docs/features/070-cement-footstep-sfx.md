# Feature 070 - Cement / gravel footstep SFX + map foot patches

Local foot plants pick **gravel** or **cement** from present-only map patches under the figure. Outside all patches defaults to gravel. Client present only — not sim collide, not net.

Depends on **069** (plant timing / gains), **066** (map-a def), **010** (cook packs). Renames **069** `stepN.wav` → `gravel-stepN.wav`.

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `gravel-step1.wav` | `assets/source/sfx/gravel-step1.wav` | Gravel plant variant (was `step1.wav`) |
| `gravel-step2.wav` | `assets/source/sfx/gravel-step2.wav` | Gravel plant variant (was `step2.wav`) |
| `gravel-step3.wav` | `assets/source/sfx/gravel-step3.wav` | Gravel plant variant (was `step3.wav`) |
| `cement-step1.wav` | `assets/source/sfx/cement-step1.wav` | Cement plant variant |
| `cement-step2.wav` | `assets/source/sfx/cement-step2.wav` | Cement plant variant |
| `cement-step3.wav` | `assets/source/sfx/cement-step3.wav` | Cement plant variant |

## Map

`map-a.json` gains `foot_patches[]`: XZ footprints with `kind` `cement` or `gravel`. Outside all patches → gravel. Patches are drawn as thin slabs for demo readability; they do not enter `MapWorld` collide/support.

Map **a** ships cement and gravel pads in the spawn radius so both voices are authored and visible.

## Behaviour

- Cook packs both three-variant banks into pack id `sfx` beside bang.
- Plant timing, gains, and no-repeat variant pick match **069**; each surface keeps its own last-variant index.
- Surface is sampled at the local figure XZ each plant (and land).
- Jump / fall land plays **two** surface plants in parallel (gain `0.2`), staggered by a random 15–50 ms — two soles, not a single plant.

## Acceptance

- Stepping on a cement pad plays cement; stepping on a gravel pad plays gravel.
- Unmarked ground still plants gravel.
- Land sounds like a dual plant (two variants, tiny offset), not one solo step.
- Missing / failed SFX load still does not block loco.
