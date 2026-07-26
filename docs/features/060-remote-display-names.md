# Feature 060 - Remote display names over body kit

Float each remote [member](../concepts.md#member)’s **display name** as 2D screen text above their [character](../concepts.md#character) (body kit) so you can tell who is in [view](../concepts.md#view).

Depends on **035** / **027** (remote present bodies), **051** (display name on roster), **057** (room-scoped roster). Present-only chrome — not sim, not score, not a new wire field.

## Goal

Roster chrome (**051**) already lists names. In the world you only see kits. This feature pins the same roster display name over each remote figure you can see.

## What floats

| Fact | Draft |
|------|--------|
| Text | The member’s **display name** from the room roster (**051** / **057**) |
| Whose | Remotes only — other members with a drawn body kit. Not the local self (first-person). Not spectators without a figure |
| When | Living remote body in local present, roster name known, and the head anchor projects in front of the camera onto the game view |
| Where | Screen-space label above the kit — head **joint** origin (no face offset) + 0.80 m world up, then project |
| Colour | **Blue** = ally, **red** = opponent. Thin **dark outline** on the glyphs so the label stays readable on any ground. No health bars or arrows in this feature |
| Size | Font scales with camera distance (larger near, smaller far; clamped) |

**Show** only for living remotes with a figure in present and an on-screen, in-front anchor. **Hide** for self, spectators, corpses, behind-camera, and off-screen. Occlusion / LOS stay out for v1 (label may draw through cover if the anchor projects).

### Ally / opponent colour

| Relation | Fill | Notes |
|----------|------|--------|
| Ally | Blue | Same [team](../concepts.md#team) as local (when the match is [team deathmatch](../concepts.md#team-deathmatch)) |
| Opponent | Red | Opposes local — every remote in [free-for-all](../concepts.md#free-for-all); other teams in TDM |

FFA today: all remotes are opponents → red. Outline is the same thin dark stroke for both fills.

## Ownership

| Piece | Role |
|-------|------|
| Roster | Source of truth for `PlayerId` → display name (already relayed) |
| Remote present | Supplies the kit pose / head world point used as the float anchor |
| Draw / overlay | Project world → screen; paint the string each frame |

No protocol bump. No drive change. Name changes only when the roster does.

## Out of scope

- Local self name over own body
- Distance fade or LOS hide (may tune later)
- Score, kit letter, or role marks on the float (roster keeps those)
- Corpse labels

## Acceptance criteria

- Each living remote body kit in view shows that member’s roster display name floating above the kit.
- Label tracks the remote as they move (anchor from present pose each frame).
- Fill is blue for allies, red for opponents; thin dark outline on both. FFA remotes are all red.
- Local self has no floating name; corpses and spectators have none.
- Remotes with no roster name yet show nothing until the roster entry exists.
- Leave / disconnect drops the label with the remote.
- Present-only; no sim or wire change.
