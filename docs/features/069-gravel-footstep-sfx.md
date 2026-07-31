# Feature 069 - Gravel footstep SFX

Local walk / sprint foot plants play a gravel one-shot through Web Audio. Walk and sprint share the same three variants for now. Client present only — not sim, not net. No terrain surface yet; gravel is the default ground voice.

Depends on **068** (SFX pack / Web Audio), **016** (walk phase), **019** (jump / land), **020** (sprint).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `step1.wav` | `assets/source/sfx/step1.wav` | Gravel plant variant |
| `step2.wav` | `assets/source/sfx/step2.wav` | Gravel plant variant |
| `step3.wav` | `assets/source/sfx/step3.wav` | Gravel plant variant |

## Behaviour

- Cook packs the three steps into pack id `sfx` beside `bang.wav`.
- Footfalls fire when sim `walk_phase` crosses Kenney neutrals **0** and **0.5** (same plants as settle — **016**).
- Active while locomotion is Walk, Sprint, or Stopping; Stand / Air clears phase tracking (no air steps).
- Each plant picks a random variant, avoiding an immediate repeat of the last index.
- Walk and sprint use the same gravel set, mixed under the **068** bang: walk / settle `0.12`, sprint `0.28`.
- Leaving Air for ground (jump land or fall) plays one gravel plant at `0.35` — no dedicated land clip yet.
- No peer footsteps; missing / failed SFX load does not block loco.

## Acceptance

- Grounded local walk and sprint produce audible gravel plants locked to stride phase; walk is quieter than sprint.
- Stopping settle can plant once at the neutral it reaches.
- Jump / fall land produces one audible plant; air stays silent for stride steps.
- Source lives under `assets/source/sfx/`; loaders use pack/asset ids only.
