# Feature 039 - Emote wheel

Mounted **B** opens a radial of in-kit character gestures. Picking one plays a short upper-body clip on self and remotes, with the **active blaster holstered** for the duration so the gesture does not fight the **037** arm → hand → weapon chain. In-built Kenney clips only; no custom dance pack.

Depends on **007** (session input), **013** / **016** / **017** / **020** (present pose, loco, sprint arms), **021** (loadout / armed identity), **037** (held attach), character kit clips (**008** / **010**), and the MP relay backbone (**022+**). Fire (**038**) cancels emote.

## Goal

Bodies need readable character between shots. The kit already ships social gestures; this feature wires them through one input surface and one present rule (holster while emoting) so peers see the same beat without inventing assets or a full animation graph.

## Input

Session-active, mounted **B**:

| Phase | Behaviour |
| --- | --- |
| **Press B** | Open the emote wheel (pointer stays locked; look may freeze or slow — draft: **freeze look while wheel is open** so the radial is readable). |
| **Hold B** | Wheel stays open. Mouse direction (or stick equivalent later) highlights a segment. |
| **Release B** | If a segment is highlighted, **commit** that emote and close the wheel. If none / centre dead-zone, close with no play. |

**Do not bind Esc** for wheel cancel. Esc is the browser/session edge for pointer lock and capture (**007**); the game must not steal or share it. Cancel-without-commit is **release B over the centre dead-zone** only.

Wheel is **not** mouse-wheel loadout (**021**). **B** does not cycle weapons.

### Gates (before open or commit)

| Gate | Draft | Role |
| --- | --- | --- |
| Session mounted | required | Same as fire / walk |
| Grounded | required | No emote start in air (**019**) |
| Not mid-burst | required | Burst string owns weapon-side actions (**038**); wait until string ends |
| Not already emoting | replace | New commit **replaces** the current emote (restarts holster + clip) |
| Active slot empty | allowed | Already unarmed: skip holster mesh change; still play clip |

Sprint: commit **cancels sprint latch** (same spirit as fire). Re-sprint needs a fresh Shift after the emote ends (or after cancel).

### Live during emote

| Input | Behaviour |
| --- | --- |
| Look | Live (after wheel closes) |
| WASD | **Cancels** emote on non-zero wish (draft: any walk wish) |
| Jump | **Cancels** |
| LMB fire | **Cancels**, then fire path runs with holster restored first so **037** muzzles exist for the shot |
| Shift sprint | **Cancels**, then sprint may latch if otherwise legal |
| Weapon wheel / equip | **Cancels**, then swap/equip applies |
| B again | Opens wheel; commit replaces |

## Wheel catalog (v1)

Four segments, fixed order (clock positions draft: N / E / S / W). Labels for UI; clip names are kit ids.

| Slot | Label | Kit clip | ~Duration (s) | Notes |
| --- | --- | --- | --- | --- |
| 0 | Yes | `emote-yes` | 0.67 | Affirm |
| 1 | No | `emote-no` | 0.67 | Negate |
| 2 | Wave | `interact-right` | 0.67 | Use / gesture stand-in |
| 3 | Bow | `pick-up` | 0.33 | Bend / respect |

All four write **`arm-right` rotation** (plus left arm, torso, head). None use root translation (unlike melee). **Out of scope:** `sit` (pose-hold), melee/kick, `die`, drive/wheelchair, custom floss.

Playback: **one-shot**, clip duration from kit, then end. No loop in v1. Rate **1×**.

## Holster (policy A)

While an emote is active (from commit until natural end or cancel):

1. **Present as unarmed for the weapon mesh:** do not draw the active blaster; do not apply **`holding-right`** / aim hold on `arm-right`.
2. **Loadout identity unchanged:** primary / secondary letters and active slot stay as **021** sim state. Holster is presentation + temporary arm ownership, not an equip to empty.
3. **Emote clip owns the upper body** for the nodes it channels (`arm-right`, `arm-left`, `torso`, `head` per kit). Legs: stand pose or last loco freeze at commit (draft: **stand bind legs**; no walk under emote because move cancels).
4. **On end or cancel:** restore armed present immediately if the active slot is filled — re-apply hold/aim (**015** / **037**) and draw the active blaster. No draw animation in v1 (instant, like **021** swap).
5. **Fire after cancel:** holster restore is ordered **before** discharge so muzzle world points exist on the same frame the shot claims.

Sprint while emoting is not a state: move/sprint cancels first.

### Why holster

`holding-right` and the hand socket parent the blaster under `arm-right`. Emote clips also write `arm-right`. Layering hold on the gesture breaks the read; leaving the gun parented makes the weapon flail with the wave. Holster removes the fight without a second grip recipe.

## Sim / present ownership

| Concern | Owner |
| --- | --- |
| Wheel open / highlight | Client input UI only (not net) |
| Emote id + start stamp + active flag | Sim self drive (reportable; rebuildable) |
| Clip sample + holster draw | Present from that drive (self and remote) |
| Cancel reasons | Sim (wish, jump, fire, swap, replace, natural end) |

Pose stays a function of sim drive (**016** philosophy): peers rebuild the same joints from emote id and time, not from baked bone streams.

Conceptual self fields (names flexible):

| Field | Role |
| --- | --- |
| `emote: Option<EmoteId>` | Slot 0…3 or none |
| `emote_start` | Tick or time when commit landed |
| (derived) `emote_age` | Now − start; ends when age ≥ clip duration |

## Net

Emote drive rides **`DriveView`** (already claim-sampled and peer-relayed at tick rate): `emote: Option<slot>` + `emote_age_s`. Shooter validates gates and owns start/cancel/age locally; server relays drive as today. Peers rebuild present (clip sample + holster) from those fields. No separate emote datagram in v1 — continuous drive is enough for short one-shots and early cancel.

Bump **`protocol`** when `DriveView` gains emote fields.

## Present details

- Sample the kit clip at `emote_age` (clamped to duration); sparse channels leave other nodes at bind unless loco/hold would have set them — under holster, do **not** layer hold.
- Self and remote use the same clip extract path already used for walk/sprint (`AnimClip` / character letter GLB).
- First-person: emote plays on the drawn body; camera stays look-mounted (**017**). Holster means no viewmodel gun for the beat (acceptable for v1).
- Wheel UI: **GPU clip-space radial** (four wedges + centre dead-zone + 5×7 bitmap labels). Readable under pointer lock; mouse angle from centre drives highlight.

## Explicitly out of scope

- Custom clips / floss / dance pack
- `sit` hold-to-stay, looped idle emotes
- Melee or kick as “emotes”
- Holster/draw animation, weapon on back mesh
- Emote damage, taunt scoring, voice
- Wheel rebinding or more than four slots
- Playing emotes on the character lineup (**008**) unless free later

## Acceptance criteria

- Session-active **B** opens a four-slot radial; release on a segment commits `emote-yes`, `emote-no`, `interact-right`, or `pick-up`; centre/empty closes without play.
- Emote starts only when grounded and not mid-burst; replaces an in-flight emote; cancels sprint latch on commit.
- While emoting, the active blaster is **not** drawn and **`holding-right` is not applied**; loadout letters and active slot are unchanged. On natural end or cancel, armed present returns instantly if the active slot is filled.
- WASD wish, jump, fire, sprint latch, and weapon swap/equip cancel the emote; fire restores holster before discharge so **037** muzzles are valid.
- Clip plays once at 1× for kit duration on self present pose; remotes show the same clip and holster via claim-and-relay (and cancel clear if wired).
- No root-translating combat clips on the wheel; no change to loadout rules beyond temporary present holster.
