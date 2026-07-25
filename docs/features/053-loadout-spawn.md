# Feature 053 - Loadout picker, spawn, and respawn

Players choose a **loadout** on a GPU surface, then press **Spawn** to enter the map with that loadout applied. After **death**, they return to the same bench (loadout + spawn) and may enter again. Flow: member → (character | spectate) → loadout → spawn → live → die → bench → spawn, inside the always-on FFA match.

Depends on **051** (room, FFA, score, spawn), **052** (character commit for players; spectators use the spectate path only), **021** (primary / secondary / active slot rules and class limits), **023** (server spawn placement), **043** (death).

## Goal

**Loadout** and **spawn** are explicit player choices. After death the member stays in the match, may edit loadout, and **Spawn**s again. Score from **051** persists across lives in the same membership.

## Product decisions (ship)

| Topic | Choice |
| --- | --- |
| Character on bench | **Frozen** after **052** confirm for this membership (no re-pick on loadout/death bench). |
| Wire | **`Spawn { primary, secondary, active }`** — loadout rides the spawn request; peers learn it from drive after enter, not in advance. |
| Staging | Bench edits are local until Spawn; editing cancels an in-flight spawn resend. |
| Defaults | **None.** Empty primary/secondary and active primary until the player picks. Empty slots are legal (unarmed when active empty). |
| Placement | Server random ground pose (**023** spirit); no min-distance retry in this feature. |
| Protocol | Alpha; version number is not a compatibility promise. |

## Presentation

Loadout grid, slot assignment, and Spawn controls are **GPU-rendered** in the client — same canvas stack as join and character pickers.

## When the loadout surface shows

| State | Surface |
| --- | --- |
| Player, character committed, waiting to enter | Loadout picker + **Spawn** |
| Player, death accepted | Same loadout + **Spawn** bench. Staged loadout starts as **what they had at death** (editable). |
| Living player | Active-slot wheel only (**021**). |

Spectators stay on the **052** spectate path. First-time join after **052** character confirm lands on this bench (replacing **051**’s bare Spawn step).

## Loadout picker

**Loadout** is the blaster choices for **primary slot** and **secondary slot** (**021** / concepts).

| Fact | Draft |
| --- | --- |
| Primary | Optional. Any **weapon class** / letter allowed by **021**. |
| Secondary | Optional. **Launcher** or **pistol** only (**021**). UI only offers legal letters. |
| Active | Slot in hand at spawn: primary or secondary. Empty active slot means unarmed at spawn. |
| Empty slots | Allowed. |
| UI | GPU two rows: letter or empty per slot; active hand toggle. |
| Catalog | Same letter map as **021**. Presentation may use kit labels; sim identity is letter ids. |
| Staging | Loadout is **staged** on the bench until **Spawn**. Bench edits apply on the next spawn. |

Server re-validates class rules on spawn and accepts only legal loadouts.

## Spawn

**Spawn** is the player’s figure entering play alive on the **map**, with that player’s **loadout** and **052** **character** applied.

| Fact | Draft |
| --- | --- |
| Control | Explicit GPU **Spawn** on the bench. |
| Placement | Server-chosen ground pose on the empty map (random position on y = 0, yaw). |
| Apply | Figure alive at full **health**; loadout slots and active as staged; fire supply as **038** / **042**. |
| Camera | On success, mount **view** on the self (**050** / look-mounted path). |
| Input | Living controls (walk, look, fire, …) while living. |

Server accepts spawn when the member is a player with a known character, staged loadout is legal, and they are **not living** (waiting to enter, including after death).

## Death → bench

**043** owns the death moment: health empty, living acts stop, `die` on the corpse present, holster.

Return path:

1. Lethal outcome applies; kill/score rules from **051** run once.  
2. Member remains in the room and match; score is unchanged by dying.  
3. Client opens the loadout + **Spawn** bench as soon as local death is accepted. Staged loadout = loadout at death. Character stays the **052** commit.  
4. Player presses **Spawn** when ready.  
5. New living figure under authority.

Membership may spawn for the whole time they remain in the match.

## Score continuity

Score is per membership in the match (**051**). Kills after a later spawn add to the same total. Leave resets as **051**. Dying leaves score as-is.

## Wire / authority

- C→S: **`Spawn { primary, secondary, active }`** (reliable).  
- Server validates class rules, character known, player role, not living → sets living, full health, broadcasts roster, sends **YouSpawned** pose.  
- Client applies staged loadout + pose on **YouSpawned** and enters Living.  
- Peers learn loadout from subsequent **DriveSample** / **PeerDrive** (not pre-announced on the bench).  
- Each spawn creates a new living figure under authority; death clears living so the next Spawn may succeed.

## Relation to 051 / 052

| Earlier step | This feature |
| --- | --- |
| **051** fixed loadout + Spawn | Loadout bench + Spawn for player entry |
| **051** death hold | Death → bench → Spawn |
| **052** character before play | Required before first bench; character held across lives |
| **052** spectate | Spectate path unchanged |

## Acceptance criteria

- Player with committed character sees an in-engine **loadout** picker (primary / secondary / active) and **Spawn**, with **021** class rules enforced.  
- No pre-selected weapons; empty slots allowed.  
- Spawn applies loadout and character, places a living figure on the empty map under server authority, and mounts the play view.  
- On death, living acts stop as **043**; player returns to the loadout + Spawn bench; score is kept; next Spawn creates a new living figure.  
- Server accepts only legal loadouts and allows re-entry after death.  
- Loadout and Spawn UI are GPU-rendered in the client.
