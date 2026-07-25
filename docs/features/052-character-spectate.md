# Feature 052 - Character picker and spectate

After **051** join, the member chooses **Play** (embody a **character** as a **player**) or **Spectate**. Character choice is a GPU picker over kits already in the cook path. Spectate uses **flycam**-style free view over the empty map and living bodies.

Depends on **051** (room, member, always-on FFA, join surface, score). Uses character kits from **008** / **010** and the free-camera control pattern from **006**. **053** owns loadout edit and respawn.

## Goal

Explicit fork after join: pick a body kit and continue toward spawn, or watch. Interactive chrome is **GPU-rendered** in the client (**051** presentation rule).

## When this runs

After successful join (**Welcome**) and before living **spawn** (or as the standing role instead of spawn):

1. Member is in the room’s FFA match (session live).  
2. Client shows a **role** step: **Play** or **Spectate**.  
3a. **Play** → **character picker** → confirm character → proceed toward spawn (defaults from **051** until **053**).  
3b. **Spectate** → enter spectator view.

From spectate, a GPU control may return to the role step to become a player (pick character, then spawn). v1 enters spectate from the post-join role step and from pre-spawn; living players keep their figure until death bench (**053**) or leave.

## Character

A **character** is a body kit. The picker lists kits the client can present (Kenney `character-a` … as cooked — same catalog lineage as lineup / self).

| Fact | Draft |
| --- | --- |
| Catalog | Playable character kits in the cooked character pack (subset validated for loco / die / hold as needed). GPU row or grid: letter or thumbnail + id. |
| Sharing | Any number of players may use the same kit. Roster identity is display name (**051**). |
| Default | **051**’s current self kit highlighted on first open. |
| Confirm | Explicit confirm (or double-activate) commits the selection. |
| Identity | Committed character id goes to the server on the member’s player intent. Peers present that kit on the figure after spawn. |
| Change | Character may be changed on this picker while waiting to spawn (and on **053** bench if that surface reuses the picker). Living figures keep the kit they spawned with until the next spawn. |

Presentation of other players uses each peer’s committed character id. Server accepts known ids; unknown commit is rejected and the prior or default kit remains.

## Spectate

A **spectator** is a member who watches. They stay in the **room** and **match** for roster and leave. Their view is free camera only.

| Fact | Draft |
| --- | --- |
| Camera | **Flycam**: free position and look, same control spirit as **006** (WASD move, mouse look, Q/E up/down, Shift sprint move). One renderer; view unmounted from a head. |
| Availability | Product path while role is spectator. May share the **006** controller implementation with debug **F8**. |
| Motion | View-only camera; may pass through empty space on the plane. |
| Input session | Pointer lock / session rules from **007** so mouse look works. |
| Scoreboard | Spectators read the GPU score roster from **051**. |
| Leave | Same as **051**. |

## Play path after character confirm

With character committed and role = player:

- **051** defaults still apply for loadout and the single **Spawn** confirm until **053**.  
- Camera while waiting to spawn stays the neutral overview from **051** (or a calm hold on the picker).  
- On spawn, figure uses the committed **character** kit for self and for how remotes draw this player.

## Wire / present

- C→S: commit role (`player` | `spectator`), and when player, `character_id`.  
- S→C: roster/presence includes role and character id so peers draw the right kit and treat spectators as members without a living figure.  
- Remote pose tables include living figures; spectators contribute presence/roster only.

## Acceptance criteria

- After join, member chooses **Play** or **Spectate** on an in-engine surface.  
- **Play** opens a GPU **character** picker over available kits (shared kits allowed); confirm stores character id for upcoming spawn; default kit matches **051**.  
- Peers present the committed character on that player’s figure after spawn.  
- **Spectate** enters **flycam** free view; controls work from the product path.  
- Spectators remain members for leave and score viewing.  
- All of the above UI is GPU-rendered inside the client.
