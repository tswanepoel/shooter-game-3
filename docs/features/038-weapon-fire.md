# Feature 038 - Weapon fire

Mounted **LMB** fires the active blaster. Class fire modes, per-letter cadence and ballistics, and **claim-and-relay** projectile spawns drive presentation: muzzle flash and mild weapon jolt on self and remotes. Infinite ammo; projectiles fly under gravity and despawn at max range.

Depends on **007**, **012**, **013** / **015** / **017**, **020**, **021**, **037** (held attach / muzzle world), and the MP relay backbone (**022+**).

## Input

Session-active, mounted **LMB** is fire. Active slot empty: no discharge. Wheel cycle and Shift sprint stay as **021** / **020**.

| Mode | Class | Trigger |
| --- | --- | --- |
| semi | pistol, sniper, shotgun, launcher | press edge; hold adds nothing |
| full auto | smg | while held, paced by letter RPM |
| burst | assault rifle | 3-round string per press (draft count) |

Air and jump leave fire free. Fire **cancels** sprint latch; re-sprint needs a fresh Shift after the fire tax. Burst commit and chain below.

## Fire gates

Before a discharge may start, two clocks (seconds, not frame counts) can still be running. They are separate from inter-shot RPM.

| Gate | Draft | Role |
| --- | --- | --- |
| **Weapon ready** `T_ready` | per class / letter family | After equip, swap, or spawn onto the letter that will fire |
| **Sprint→fire** | **0.12 s** + active letter’s `T_ready` | From the moment fire clears sprint until fire may spawn |

Both apply when both apply. RPM is its own interval between discharges.

### `T_ready` (s, draft)

| Class | Letters | `T_ready` |
| --- | --- | --- |
| pistol | b, i | 0.06 |
| smg | c, g, h, l, m, p | 0.08 |
| shotgun | j, k, o | 0.10 |
| assaultRifle | d, n, q, r | 0.12 |
| sniperRifle | e, f | 0.16 |
| launcher | a | 0.18 |

## Burst (assault rifle)

- String length **3**; spacing from the letter’s RPM family.
- While a string runs, weapon-side actions wait: sprint, wheel cycle, equip command. Look, WASD, and jump stay live.
- The current string always finishes.
- **Hold-to-chain:** LMB still down at string end starts the next string (after shot interval).
- **Pending press:** one re-press mid-string arms a single follow-up string when the current one ends.

## Discharge and projectiles

Each accepted discharge spawns one or more projectiles in sim. Config uses metres, m/s, seconds, degrees; ticks come from `TICK_HZ`.

| Field | Unit | Role |
| --- | --- | --- |
| id | — | Stable for net and present |
| owner | player | Shooter claim |
| weapon | letter | Active blaster at spawn |
| origin | m | World spawn point |
| velocity | m/s | Initial world velocity |
| max_range | m | Despawn after this path length |

**Aim** is look / view centre (**015**). **Origin** is the firing muzzle point(s) from the **037** held chain (blaster-local muzzles under `held_blaster`; same world points **012** marks on the lineup), at or slightly ahead of the barrel exit. Present jolt does not steer aim.

**Motion:** constant gravity \(\mathbf{g} = (0, -9.81, 0)\,\mathrm{m/s^2}\):

\[
\mathbf{v} \mathrel{+}= \mathbf{g}\,dt,\quad \mathbf{p} \mathrel{+}= \mathbf{v}\,dt
\]

Despawn when distance along the path from origin reaches `max_range`.

### Muzzle policy (per letter)

| Policy | Meaning |
| --- | --- |
| single | One muzzle base; pellets from that point |
| all | Every kit muzzle on the same discharge |
| alternate | Round-robin one muzzle per discharge |

| Letter | Policy | Pellets (draft) |
| --- | --- | --- |
| a–h, k, m, n, r | single | 1 (shotgun **k**: 6, spread) |
| i, l, p, q | alternate | 1 per discharge |
| j | all (2) | 3 per muzzle |
| o | all (4) | 2 per muzzle |

Shotgun and multi-pellet letters use a look-space cone (half-angle in the tune table). Scatter is sim-owned and calibratable.

### Tune table (draft — feel later)

RPM → discharge interval \(60 / \mathrm{RPM}\) s. Burst uses the same family inside a string.

| Letter | RPM | \(v_\mathrm{muzzle}\) (m/s) | max range (m) | spread half-angle (°) |
| --- | --- | --- | --- | --- |
| a | 48 | 85 | 80 | 0.5 |
| b | 220 | 380 | 120 | 0.4 |
| c | 700 | 420 | 140 | 0.6 |
| d | 600 | 650 | 200 | 0.35 |
| e | 48 | 820 | 300 | 0.15 |
| f | 42 | 850 | 320 | 0.12 |
| g | 750 | 400 | 130 | 0.65 |
| h | 720 | 410 | 135 | 0.6 |
| i | 200 | 360 | 110 | 0.45 |
| j | 90 | 380 | 60 | 2.5 |
| k | 85 | 370 | 55 | 3.0 |
| l | 680 | 415 | 135 | 0.6 |
| m | 710 | 405 | 130 | 0.65 |
| n | 580 | 640 | 200 | 0.35 |
| o | 75 | 360 | 50 | 3.5 |
| p | 780 | 430 | 140 | 0.55 |
| q | 560 | 630 | 195 | 0.4 |
| r | 590 | 645 | 200 | 0.35 |

## Net

Shooter runs fire gates and spawn, then **claims** the projectiles. Server **relays** to peers. Remotes **accept** and present with the same motion rules. Server watches traffic; it does not own projectile objects.

Conceptual wire (postcard, names flexible):

| Message | Direction | Carries |
| --- | --- | --- |
| projectile spawn (batch ok) | C→S | id(s), weapon, origin, velocity, spawn stamp/tick |
| peer projectile spawn | S→C | same + `PlayerId` |

Bump **`protocol`** when the wire changes. Align claim send/apply with the joined input path so self and peers share one story.

## Present

Flash and jolt follow **discharge**, self and remote alike.

- **Muzzle flash:** solid sphere (~0.03 m, warm colour, ~0.05 s draft) at each muzzle that fired, in **present pose**: world points from **037** (`held_blaster · muzzle_local`), same chain as the drawn blaster (optional slight bore offset). Present placement — not a free sim/aim fudge. **Rebind each frame** to the live muzzle while the flash lives (no frozen world crumbs under move).
- **Weapon jolt:** mild present-pose kick on the held blaster — pitch/yaw and a short push back along the bore — then settle toward rest. **Pivot is the hand / weapon grip `G` (037)** (`held · T(G) · J · inv(T(G))`), not the blaster mesh origin. Camera and reticle stay put. Flash tracks the same pose.
- Optional dev-gated debug tracer for projectiles while tuning.

### Jolt (class draft)

| Class | pitch (°) | yaw ± (°) | back (m) | settle (s) |
| --- | --- | --- | --- | --- |
| pistol | 0.4 | 0.1 | 0.008 | 0.04 |
| smg | 0.25 | 0.08 | 0.006 | 0.03 |
| assaultRifle | 0.35 | 0.1 | 0.010 | 0.045 |
| sniperRifle | 0.7 | 0.12 | 0.014 | 0.07 |
| shotgun | 0.9 | 0.2 | 0.018 | 0.06 |
| launcher | 1.1 | 0.15 | 0.020 | 0.08 |

New discharge adds kick (clamped); recover toward rest each frame.

## Dev equip

Text command (same console paradigm; name in help, e.g. `blaster <letter>`): set active slot to `a`…`r`. If the letter is illegal for the current active slot under **021**, flip primary ↔ secondary so the letter fits, then apply. Pays `T_ready` like a swap. Waits while a burst string is running.

## Config

One baked weapon table in **`game-sim`** (shared with present as needed): mode, `T_ready`, RPM, muzzle velocity, max range, spread, muzzle policy, jolt. Sim owns spawn and integration; client reads flash/jolt for draw.

## Acceptance criteria

- Mounted session LMB fires when the active slot is armed: semi on press edge, SMG full auto while held, AR 3-round burst with commit, hold-to-chain, and one pending re-press.
- Fire clears sprint; next fire waits for sprint→fire tax and letter `T_ready`; swap/equip pays `T_ready`.
- Burst holds weapon-side actions; look, move, and jump stay free. Air leaves fire free.
- Each discharge spawns projectile(s) from look aim and muzzle points, with letter muzzle velocity, gravity, and max-range despawn; ammo is unlimited.
- Multi-muzzle and shotgun spread follow the letter policy and tune table.
- Shooter claims spawns; server relays; remotes show flash and jolt (and optional debug tracers) from those claims.
- Flash spheres sit on the present-pose **037** muzzles that fired (self and remotes) and **rebind each frame** for their lifetime; jolt is present-pose on the held blaster, pivoting on grip / hand socket.
- Dev text command equips any letter, flipping slots when **021** requires it.
- Tables are real units (s, m, m/s, °), calibratable without changing the model.
