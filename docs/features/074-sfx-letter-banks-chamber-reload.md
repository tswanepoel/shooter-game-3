# Feature 074 - Letter SFX, muzzle-load semi, auto-chamber

SFX source ids use letter suffixes (`bang-a.wav`, `gravel-step-b.wav`, …) instead of digits. Bang, chamber, and reload join the pack with fixed per-class voices. Semi-fire blasters are muzzle-load (darts in the front tube(s), not a strip mag) and auto-chamber when empty. Client present for audio; capacity and auto-chamber are sim. Mag-fed (full-auto / burst) keep class magazine sizes and manual **R**.

Depends on **071** (bang by class), **068** (Web Audio / `sfx` pack), **058** (magazine / reserve / reload), **038** (muzzle policy).

## Source

| Asset | Path | Notes |
| --- | --- | --- |
| `bang-a.wav`…`bang-e.wav` | `assets/source/sfx/` | Per-class discharge (five clips; launcher shares shotgun) |
| `chamber-a.wav`…`chamber-d.wav` | `assets/source/sfx/` | Fixed per class; semi load voice |
| `reload-a.wav` | `assets/source/sfx/` | Mag-fed reload slap (auto / burst) |
| `*-step-a.wav`…`c` | `assets/source/sfx/` | Foot plants renamed from `*-stepN.wav` |

Former `bang-d` clip is kept as `throw-a.wav` (not cooked) for a future throw action.

## Bang / chamber assignment

Same class → same bang / chamber. `reload-a` is shared by mag-fed letters.

| Class | Bang | Chamber | Letters |
| --- | --- | --- | --- |
| Pistol | `bang-a` | `chamber-a` | `b` `i` |
| SMG | `bang-b` | `chamber-b` | `c` `g` `h` `l` `m` `p` |
| Assault rifle | `bang-c` | `chamber-c` | `d` `n` `q` `r` |
| Sniper | `bang-d` | `chamber-d` | `e` `f` |
| Shotgun | `bang-e` | `chamber-a` | `j` `k` `o` |
| Launcher | `bang-e` | `chamber-b` | `a` |

## Semi muzzle-load capacity

| Semi muzzle policy | Capacity | Meaning |
| --- | --- | --- |
| `Single` | `pellets` | One tube; multi-pellet letters load that many foam bits |
| `Alternate` | kit muzzle count | One dart per tube; round-robin empties one per shot |
| `All` | muzzles × pellets | Full bank for one volley |

Kit muzzle counts match client `BLASTER_MUZZLE_POINTS` (`a`..=`r`).

| Letter | Mode / policy | Capacity |
| --- | --- | --- |
| `a` `b` `e` `f` | Semi / Single | 1 |
| `i` | Semi / Alternate (2) | 2 |
| `k` | Semi / Single (6 pellets) | 6 |
| `j` | Semi / All (2×3) | 6 |
| `o` | Semi / All (4×2) | 8 |
| Mag-fed (auto / burst) | class table | unchanged (e.g. SMG 30) |

## Behaviour

- Cook packs all letter-id WAVs into pack id `sfx`.
- Accepted self discharge plays the class bang only (no post-bang chamber).
- `WeaponDef::mag_capacity` uses the semi table above for `FireMode::Semi`; otherwise `mag_capacity_for_class`.
- **R** fills from reserve up to capacity (**058**). Present: **semi** → class chamber; **full-auto / burst** → `reload-a`.
- Semi auto-chamber: after a shot, only when tubes are **empty**, arm a load timer

\[
t_\mathrm{chamber} = 0.25\ \mathrm{s} + 0.12\ \mathrm{s} \times \mathrm{rounds\ needed}
\]

  then fill from reserve (same as reload). Multi-muzzle semis (e.g. dual-tube `i`) fire the bank dry first, then one seat fills every empty tube. Empty reserve seats nothing. Death / unarmed / swap cancel a pending chamber. Manual **R** still works (instant top-up, including mid-bank).
- A landed auto-chamber plays the class chamber clip; nothing when reserve was empty.
- Fire is self-gating on empty tubes, so semi cadence is chamber-limited when the load is slower than RPM.
- Foot plant timing / gains / surfaces unchanged — only asset ids renamed; foot plants still randomize within surface banks.

## Acceptance

- Numbered SFX ids are gone; letter ids load and play.
- Same letter always uses the same bang / chamber clips.
- Pistol / sniper / launcher semis hold at most one ready load unless multi-tube / multi-pellet rules say otherwise.
- Dual-tube `i` fires both barrels before one chamber seats both; `j` / `o` empty in one volley then auto-seat.
- Mag-fed letters keep prior magazine sizes and never auto-chamber; they still need R.
- Semi R / auto-chamber sound like chamber; auto/burst R sounds like reload slap; failed / dry reserve stays silent.
- Missing / failed SFX load still does not block fire, reload, or loco.
