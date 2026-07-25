# Feature 053 - Loadout picker, spawn, and respawn

Players choose a **loadout** on a GPU surface, then press **Spawn** to enter the map with that loadout applied. After **death**, they return to the same bench (loadout + spawn) and may enter again. Flow: member → (character | spectate) → loadout → spawn → live → die → bench → spawn, inside the always-on FFA match.

Depends on **051** (room, FFA, score, spawn), **052** (character commit for players; spectators use the spectate path only), **021** (primary / secondary / active slot rules and class limits), **023** (server spawn placement), **043** (death).

## Goal

**Loadout** and **spawn** are explicit player choices. After death the member stays in the match, may edit loadout, and **Spawn**s again. Score from **051** persists across lives in the same membership.

## Presentation

Loadout grid, slot assignment, and Spawn controls are **GPU-rendered** in the client — same canvas stack as join and character pickers.

## When the loadout surface shows

| State | Surface |
| --- | --- |
| Player, character committed, waiting to enter | Loadout picker + **Spawn** |
| Player, death accepted in present | After living acts have stopped and `die` present has begun, open the same loadout + **Spawn** bench. Corpse present may remain until next spawn of that player or a short timeout already used by present. Bench is available as soon as death is accepted. |
| Living player | Active-slot wheel only (**021**). |

Spectators stay on the **052** spectate path. First-time join after **052** character confirm lands on this bench (replacing **051**’s bare Spawn step). Defaults pre-select primary `p`, secondary `b`, active primary so Spawn can be immediate.

## Loadout picker

**Loadout** is the blaster choices for **primary slot** and **secondary slot** (**021** / concepts).

| Fact | Draft |
| --- | --- |
| Primary | Optional. Any **weapon class** / letter allowed by **021**. |
| Secondary | Optional. **Launcher** or **pistol** only (**021**). |
| Active | Slot in hand at spawn: primary or secondary. Default primary when that slot is filled; else secondary when filled; else unarmed. |
| Empty slots | Allowed; active empty means unarmed at spawn. |
| UI | GPU list or two columns: letter or “empty” per slot. Illegal secondary picks stay unassigned (highlight feedback). |
| Catalog | Same letter map as **021**. Presentation may use kit labels; sim identity is letter ids. |
| Staging | Loadout is **staged** on the bench until **Spawn**. Bench edits apply on the next spawn. |

Server re-validates class rules on spawn and accepts only legal loadouts.

## Spawn

**Spawn** is the player’s figure entering play alive on the **map**, with that player’s **loadout** and **052** **character** applied.

| Fact | Draft |
| --- | --- |
| Control | Explicit GPU **Spawn** on the bench. |
| Placement | Server-chosen ground pose on the empty map (**023** spirit: random position on y = 0, yaw). Prefer a simple distance retry away from other living bodies when cheap. |
| Apply | Figure alive at full **health**; loadout slots and active as staged; fire supply as **038** / **042**. |
| Camera | On success, mount **view** on the self (**050** / look-mounted path). |
| Input | Living controls (walk, look, fire, …) while living. |

Server accepts spawn when the member is a player with a known character, staged loadout is legal, and they are waiting to enter (including after death).

## Death → bench

**043** owns the death moment: health empty, living acts stop, `die` on the corpse present, holster.

Return path:

1. Lethal outcome applies; kill/score rules from **051** run once.  
2. Member remains in the room and match; score is unchanged by dying.  
3. Client opens the loadout + **Spawn** bench. Staged loadout defaults to **what they had at death** (editable). Character stays the **052** commit for this membership.  
4. Player presses **Spawn** when ready.  
5. New living figure; corpse present ends or is replaced per the present rule above.

Membership may spawn for the whole time they remain in the match.

## Score continuity

Score is per membership in the match (**051**). Kills after a later spawn add to the same total. Leave resets as **051**. Dying leaves score as-is.

## Wire / authority

- C→S: staged loadout (primary letter or empty, secondary letter or empty, active slot) and **Spawn** request.  
- Server validates class rules, character known, player role, waiting-to-enter → spawns figure, broadcasts presence/pose.  
- S→C: living flag / figure spawn so peers create remote present; death already claimed via **043**.  
- Each spawn creates a new living figure under authority.

## Relation to 051 / 052

| Earlier step | This feature |
| --- | --- |
| **051** fixed loadout + Spawn | Loadout bench + Spawn for player entry |
| **051** death hold | Death → bench → Spawn |
| **052** character before play | Required before first bench; character held across lives |
| **052** spectate | Spectate path unchanged |

Ship **051**/**052** first with the simpler spawn step if needed; fold that step into this bench so one spawn UI remains.

## Acceptance criteria

- Player with committed character sees an in-engine **loadout** picker (primary / secondary / active) and **Spawn**, with **021** class rules enforced.  
- Defaults remain `p` + `b` + active primary so immediate Spawn works.  
- Spawn applies loadout and character, places a living figure on the empty map under server authority, and mounts the play view.  
- On death, living acts stop as **043**; player returns to the loadout + Spawn bench; score is kept; next Spawn creates a new living figure.  
- Server accepts only legal loadouts.  
- Loadout and Spawn UI are GPU-rendered in the client.
