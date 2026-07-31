# Feature 075 - Hit marker SFX

Local firer impact claim plays a one-shot hit tick with the hit-marker flash. Client present only — not sim, not net.

Depends on **044** (hit marker), **068** (Web Audio / `sfx` pack).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `hit.wav` | `assets/source/sfx/hit.wav` | Mono PCM WAV; cook → `sfx` pack |

## Behaviour

- Cook packs `hit.wav` into pack id `sfx` (asset id `hit.wav`).
- Same claim batch that pulses the X (**044**) also plays one hit tick (overlaps on successive frames).
- Peers do not play this clip; only the local firer’s claim.
- Missing / failed SFX load does not block play; marker flash and fire continue silently.

## Acceptance

- Local accepted impact claim produces an audible hit when the session is active and audio is unlocked.
- Hit sound and X flash share the same pulse moment.
- Source lives under `assets/source/sfx/`; loaders use pack/asset ids only.
