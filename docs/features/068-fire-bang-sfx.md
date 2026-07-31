# Feature 068 - Fire bang SFX

Accepted local fire plays a one-shot bang through Web Audio. Client present only — not sim, not net.

Depends on **038** (discharge), **010** (cook packs).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `bang.wav` | `assets/source/sfx/bang.wav` | Mono PCM WAV; cook → `sfx` pack |

## Behaviour

- Cook packs `bang.wav` into pack id `sfx` (asset id `bang.wav`).
- Client loads the pack once, decodes with `AudioContext.decodeAudioData`.
- Each accepted self discharge plays one bang (overlaps for full-auto / multi-pellet strings).
- AudioContext resumes on canvas click (same gesture as input session) for autoplay policy.
- No peer distance attenuation in this feature; peers do not play this bang.

## Acceptance

- Local accepted fire produces an audible bang when the session is active and audio is unlocked.
- Missing / failed SFX load does not block play; fire and FX continue silently.
- Source lives under `assets/source/sfx/`; loaders use pack/asset ids only.
