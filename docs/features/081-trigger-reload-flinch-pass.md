# Feature 081 - Trigger, reload gate, flinch, magazine, and name-label pass

A combat-feel and chrome pass. Burst blasters stop streaming under a held trigger, reload costs time before rounds land, hit impulse reads harder, magazines shrink, the hit click survives its own bang, and the overhead display name stops covering the loadout panel.

Depends on **038** (fire gates), **044** / **045** (hit marker, hit impulse), **058** (magazine / reserve / reload), **060** (remote display names), **063** (product surfaces), **074** (seat voice / reload slap), **075** (hit marker SFX).

## Trigger

A [fixed string per press](../concepts.md#fire-mode) ends on its own. Holding the trigger no longer arms the next string — only a fresh press does, mid-string or after. Full-auto (held stream) and semi (one per press) are unchanged.

## Reload gate

An asked-for [reload](../concepts.md#reload) now takes handling time before it fills the [magazine](../concepts.md#magazine). No [fire](../concepts.md#fire) is accepted while rounds are being loaded — by an asked-for reload or by a timed **chamber seat**. Swapping the [active slot](../concepts.md#active-slot), spawning, or dying cancels a reload in flight.

| Mode | Handling time |
| --- | --- |
| Timed-seat letters | **074** seat time (base + per-round) |
| Mag-fed (full-auto / burst) R | One slap, matching the `reload-a` clip |

The reload voice plays on the ask, not on the landing, so the clip and the gate run together.

## Hit impulse

[Hit impulse](../concepts.md#hit-impulse) fold and twist per point of drained [health](../concepts.md#health) roughly double, and their caps rise with them. Fall time is unchanged, so a hit reads as a harder jolt, not a longer one.

## Magazine, chamber & reserve

Carry cap = 2× spare. Spare draft = max across loadout per ammo kind.

**Chamber** = seated store (noun). **Seat** = the (possibly timed) act of moving round(s) into the chamber; a timed seat blocks fire and plays the seat voice (verb). Never call the verb “chamber.”

### Per blaster

|   | class    | mode  | muz | chamber | seat | mag | load | spare |
| - | -------- | ----- | --: | ------: | ---: | --: | ---: | ----: |
| a | launcher | semi  |   1 |       1 |    1 |   — |    — |     2 |
| b | pistol   | semi  |   1 |       1 |    — |  12 |    8 |    30 |
| c | smg      | auto  |   1 |       1 |    — |  24 |   20 |    60 |
| d | ar       | burst |   1 |       1 |    — |  24 |   16 |    48 |
| e | sniper   | semi  |   1 |       1 |    1 |   6 |    4 |    12 |
| f | sniper   | semi  |   1 |       1 |    1 |   6 |    4 |    12 |
| g | smg      | auto  |   1 |       1 |    — |  24 |   20 |    60 |
| h | smg      | auto  |   1 |       1 |    — |  24 |   20 |    60 |
| i | pistol   | semi  |   2 |       2 |    2 |   — |    — |    30 |
| j | shotgun  | semi  |   2 |       2 |    2 |   — |    — |    24 |
| k | shotgun  | semi  |   1 |       1 |    1 |   6 |    4 |    18 |
| l | smg      | auto  |   2 |       1 |    — |  24 |   20 |    60 |
| m | smg      | auto  |   1 |       1 |    — |  24 |   20 |    60 |
| n | ar       | burst |   1 |       1 |    — |  20 |   16 |    48 |
| o | shotgun  | semi  |   4 |       4 |    4 |   — |    — |    24 |
| p | smg      | auto  |   2 |       1 |    — |  24 |   20 |    60 |
| q | ar       | burst |   2 |       1 |    — |  20 |   16 |    48 |
| r | ar       | burst |   1 |       1 |    — |  20 |   16 |    48 |

### Align (code ↔ table)

- Letter table owns `chamber`, `seat` (count), `mag`, `load`, `spare`.
- Every letter tracks chamber rounds; [fire](../concepts.md#fire) spends the chamber.
- One seated round per firing muzzle, each a full pellet spray — so `chamber` covers `muz` on the letters that fire every barrel at once (`j` 2, `o` 4). A part-seated chamber lights only the barrels it can pay for.
- `seat` — / `0`: seat from mag (or reserve when no mag) instantly, with no timed seat / no seat voice.
- `seat` N: N rounds go into the chamber, one timed seat at a time; seat voice once per round seated.
- No-mag (`a` `i` `j` `o`): `mag` —; chamber / seat 1 / 2 / 2 / 4; refill from reserve starts only when empty, so `i` fires both chambered rounds before reseating.
- SFX / APIs: verb is **seat** (asset ids stay `pump-*.wav` / `breech-*.wav`, unchanged this pass); not “chamber.”
- Carry cap / spare draft already match.
- Dev HUD shows chambered + magged (not mag alone).

## Active hand

Wheel / cycle will not move onto an empty slot while the other hand still holds a [blaster](../concepts.md#blaster). Spawn and the loadout bench coerce [active slot](../concepts.md#active-slot) the same way — secondary-only loadouts spawn in the secondary hand, not unarmed.

## Voices

The bang sits under the hit click and the click is nudged past the bang attack, so a landed shot is heard over its own report. Client present only.

Bang class map (same four clips as **076**, remapped):

| Bang | Classes | Letters |
| --- | --- | --- |
| `bang-a` | Pistol, sniper | `b` `i` · `e` `f` |
| `bang-b` | Assault rifle, shotgun | `d` `n` `q` `r` · `j` `k` `o` |
| `bang-c` | Launcher | `a` |
| `bang-d` | SMG | `c` `g` `h` `l` `m` `p` |

Seat / breech envelope (sim emits start → seat×N → end; present picks the style):

| Style | When | Letters |
| --- | --- | --- |
| Slide `pump-a` | each seat | `k` `o` |
| Slide `pump-b` | each seat | `e` `f` |
| Slide `pump-c` | each seat | `i` |
| Slide `pump-d` | each seat | `a` |
| Breech `breech-open-a` / `breech-close-a` | open at sequence start, close after last seat (seats silent) | `j` |

## Name label

The overhead display name anchor sits 0.6 m above the head joint, is outlined in near-black at a thinner offset, and is not drawn while a product Gate or Panel surface (**063**) owns the screen.

## Acceptance

- Holding fire on `d` / `n` / `q` / `r` yields exactly one string; releasing and pressing again yields the next.
- R on an SMG plays the slap, rounds appear when it ends, and the trigger is dead until then.
- Timed-seat letters still seat (**074**) and block fire until the round lands.
- Taking a hit visibly jolts the weapon line more than before and settles in the same time.
- SMG spawns 20 / 60; AR 16 / 48; sniper 4 / 12.
- A hit landed on the same frame as a shot is audible.
- No display name floats over the loadout panel.
- Primary-only: scroll wheel does not go unarmed. Secondary-only spawn: hand is secondary, not empty.
- Name label sits ~0.6 m above the head.
