# Feature 051 - Room join and never-ending FFA

A visitor enters a **room** with a room code and **display name** on **GPU** surfaces, becomes a **member**, and is in that room’s always-on **free-for-all** **match** on today’s empty scene (the **map**). Default **character** and **loadout** apply; an explicit **Spawn** confirm places a living figure on the plane already shipping. **Kills** raise **score** for the membership lifetime while the match keeps running.

Depends on **022**–**023** (join / authority / wire), **035+** (remotes), **038** / **042** / **043** (fire, ammo mass, health / death / hit claims), **021** (default loadout identity). **052** and **053** extend role and loadout/spawn; this feature stands alone with defaults.

## Goal

Name multiplayer entry: **room**, **member**, **match** (free-for-all), **map** (empty plane). Same ground and combat path as today. Product delta is identity, entry, and score.

## Presentation

Join chrome, spawn confirm, and score are **drawn by the game client** (WebGPU / in-engine). Browser storage may remember the display name (cookie or equivalent) for pre-fill only. The canvas is the interactive surface once the client is running.

## Room

A **room** is a joinable gathering. This MVP serves one logical room.

| Fact | Draft |
| --- | --- |
| Room code | String the client sends on join |
| Default code | `dev` — pre-filled on the join surface |
| Validation | Server accepts `dev` (case-sensitive draft). Unknown codes **Reject** with a clear reason. |
| Match | The room owns exactly one **match**, already running when the host process is up. Join attaches the member to that match. |

The empty scene (grid plane, y = 0, current lighting) **is** the **map**.

## Member and display name

Join succeeds as a **member**: a person in the room with a **display name**. Server assigns the session id used on the wire today (`PlayerId` / session key from **023**). Authority stays on that session id; display name is the roster label.

| Fact | Draft |
| --- | --- |
| Display name | Short string shown to others (roster / score). Server rejects empty after trim. |
| Unique | Unique among **current members** of the room. Compare trim + **case-insensitive**. Clash → **Reject**. Leave frees the name. |
| Length | Draft cap **24** visible characters after trim; server enforces consistently. |
| Persistence | Client may load/save the last name in a **cookie** (or same-site web storage). Pre-fill the GPU name field when present. |
| Wire | Join path carries display name. Server stores it on the member and relays it with presence for labels and score rows. |

Console `mp join` may remain as a dev bypass (name fallback e.g. `dev`). The supported path is the in-engine join surface.

## Join surface (GPU)

Before a session is joined, the client shows a small in-world (or full-frame) panel:

1. **Room code** — default `dev`, editable  
2. **Display name** — pre-filled from cookie when available, editable  
3. **Join** — confirm control  

This surface owns interaction until join succeeds or the user leaves the page. After **Welcome**, the join surface closes. Failed join (**Reject**, transport error) keeps the surface open with a short GPU-readable reason.

## Match: always-on free-for-all

On successful join the member is in the room’s **match**: **free-for-all** on the empty **map**.

| Fact | Draft |
| --- | --- |
| Mode | Free-for-all. Every **player** opposes every other **player**. |
| Lifecycle | Match runs for the host lifetime. Score accumulates; play continues. |
| Entry | Join places the member in the match. |
| Role | Every joiner is a **player** who will **spawn** (**052** adds spectate). |

## Defaults: character, loadout, spawn

**052** / **053** add pickers. Here, fixed defaults so combat and score are testable immediately:

| Slot | Default |
| --- | --- |
| **Character** | The same body kit the self uses today (current Kenney self). |
| **Loadout** | Primary `p`, secondary `b`, active primary (**021**). |
| **Spawn** | After join, a GPU **Spawn** confirm. On confirm, server places the figure on the map (random ground pose as **023**) with that loadout; the member has a living **player** figure. |

Until Spawn, the client is joined with session live and camera on a neutral overview of the empty map (fixed pose; **052** flycam is for spectate).

## Score and kill

**Kill** is a record that a **player** caused an **opponent**’s **death**. Under free-for-all, that raises the killer’s **score** by one.

| Fact | Draft |
| --- | --- |
| Opponent | Any other living player in the match. |
| Attribution | The firer whose **hit claim** reduced the target’s **health** to empty owns the kill (**043** claim path). One killer per death. |
| Score | Non-negative integer per member, starts at **0** on join. Increases by **1** per kill. |
| Present | GPU score readout: local score at minimum; preferably a roster of display name + score for members in the match. May compose with the debug net HUD in dev; product path draws in-engine. |
| Death | **043** death: living acts stop, `die` present. Dead player keeps score and stays on the roster (**053** returns them to spawn). |

A death with an opposing firer claim is the path that awards a kill. Score changes only through that path in this feature.

## Solo and leave

Local solo stays available as today. **Leave** ends membership, clears match present for that client, and returns to solo or to the join surface. Server removes the member from the roster; rejoin starts score at 0.

## Wire / authority

- Join carries **display name** and **room code** before **Welcome**.  
- Presence / roster S2C carries display name (**052** adds character id).  
- **One** server-truth score table for the roster; clients present that table. Draft: server applies score when it accepts a lethal outcome, aligned with **043** claims.  
- Room code mismatch, empty name, and name clash are **Reject** reasons.

## Acceptance criteria

- In-engine join surface: room code (default `dev`), display name (cookie pre-fill when present), Join control.  
- Valid join to `dev` with a free, non-empty name yields **Welcome** and a **member** in the room’s always-on FFA **match** on the empty **map**.  
- Display name is unique among current room members (case-insensitive); empty name, clash, or invalid code **Reject** with a visible in-engine reason.  
- Defaults: current self **character**, loadout primary `p` / secondary `b` / active primary.  
- Spawn confirm places a living figure on the plane under authority as **023**; pre-spawn joined clients use overview camera only.  
- A kill (lethal hit claim on an opponent) raises the killer’s **score** by 1; match and scorekeeping continue.  
- Score is presented in-engine (local at minimum; roster preferred).  
- Death follows **043**; leave returns the client out of the room cleanly.  
- Solo path without join remains usable.
