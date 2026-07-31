# Feature 071 - Per-blaster bang voices

Accepted local fire picks a bang by active blaster class. Three one-shots replace the single **068** clip. Client present only — not sim, not net.

Depends on **068** (SFX pack / Web Audio), **021** (weapon class).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `bang1.wav` | `assets/source/sfx/bang1.wav` | Was **068** `bang.wav` |
| `bang2.wav` | `assets/source/sfx/bang2.wav` | Mid discharge |
| `bang3.wav` | `assets/source/sfx/bang3.wav` | Heavy discharge |

## Assignment

| Bang | Classes | Letters |
| --- | --- | --- |
| `bang1` | Pistol, SMG | `b` `i` · `c` `g` `h` `l` `m` `p` |
| `bang2` | Assault rifle, sniper | `d` `n` `q` `r` · `e` `f` |
| `bang3` | Shotgun, launcher | `j` `k` `o` · `a` |

## Behaviour

- Cook packs all three into pack id `sfx` (asset ids `bang1.wav`…`bang3.wav`).
- Each accepted self discharge plays the bang for that discharge’s weapon letter (class map above).
- Overlaps still allowed for full-auto / multi-pellet / burst strings.
- Missing / failed SFX load still does not block fire.

## Acceptance

- Swapping between a pistol/SMG, an AR/sniper, and a shotgun/launcher yields three distinct bangs.
- Same class letters share a bang; letter never randomizes within a discharge.
- Source lives under `assets/source/sfx/`; loaders use pack/asset ids only.
