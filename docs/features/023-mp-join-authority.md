# Feature 023 - MP join and authority

Devtools join an implicit one-world lobby. The server assigns identity and spawn, applies client input each tick, and returns authoritative pose for the joined self. Local solo stays available until join; after join the client yields self advancement to the server.

Depends on **022** (backbone, wire, session key fields, fixed tick).

## Join

- Dev console commands enter and leave multiplayer (e.g. `mp.join` / `mp.leave`).
- Join opens WebSocket to the game server port, sends **Hello** `{ protocol, content_rev }`.
- Server accepts with **Welcome** `{ you: PlayerId, tick, spawn, key, issued_tick, content_rev }` or **Reject** with a reason code/string.
- Spawn is server-chosen: random ground position (plane **y = 0**) and yaw.
- Leave closes the session and returns the client to solo self advancement at a coherent local pose.

## Authority

- While **solo**: client advances `SelfState` from local input (016–021) as today.
- While **joined**: client sends **Input** each outbound step; server **applies input** through `game-sim`; **Snapshot** carries the authoritative pose for `you`.
- Joined client presents local self from server pose (look + present as today, drive sourced from authority).
- Input carries movement intents (wish, look, jump, sprint, weapon cycle as needed) plus **seq** and **session key echo** (022). Server applies only when echo matches current key.

## Messages (this feature)

| Dir | Name | Role |
| --- | --- | --- |
| C→S | `Hello` | Protocol + content_rev |
| S→C | `Welcome` | Id, tick, spawn, key, issued_tick, content_rev |
| S→C | `Reject` | Join failure |
| C→S | `Input` | Seq, key echo, movement intents |
| S→C | `Snapshot` | Tick, key, issued_tick, pose for `you` (and room for others in 024) |

## Acceptance criteria

- Devtools can join and leave the server session.
- Successful join yields a server `PlayerId` and spawn; local self relocates to that spawn under authority.
- While joined, WASD / look / jump / sprint / weapon cycle reach the server as Input and move the authoritative self.
- Snapshot updates the joined local self each tick; presentation follows that drive.
- Session key on Welcome/Snapshot is echoed on Input; mismatched echo drops that input on the server.
- Protocol mismatch rejects join; content_rev mismatch discards affected handling per 022.
- Leave restores solo advancement without requiring further server messages.
