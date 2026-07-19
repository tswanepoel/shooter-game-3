# Feature 022 - MP backbone

Network multiplayer rests on a shared wire protocol, a native authority host, and a client multiplayer mode. Solo load stays the default path; multiplayer is an explicit client mode.

## Layout

| Piece | Role |
| --- | --- |
| `game-sim` | Pure rules: input apply, self drive, pose rebuild inputs. No sockets. |
| `game-net` | Wire DTOs, postcard codec, protocol version, content revision. No sim apply, no GPU. |
| `game-server` | Native process: WebSocket accept, fixed tick, server sim, broadcast. Multiplayer is the whole process. |
| `game-client` | WASM. Solo self as today. |
| `game-client` **`mp/`** | Client multiplayer mode only: transport, session, outbound intents, inbound apply, remotes. |

Client multiplayer code lives under **`mp/`**. The server has no `mp/` tree.

### `mp/` owns

- WebSocket binary transport
- Join lifecycle and authority yield/reclaim
- Building and sending C2S; receiving and applying S2C
- Inbound and outbound queues (seam for later delay and metrics)
- Joined local pose from server; remote pose table for presentation

### `mp/` uses, does not redefine

- Wire types and codec from `game-net`
- Movement rules from `game-sim` (server applies; client presents)
- Present/view paths already on the client (drive in → mesh out)
- Session keyboard/mouse from 007 (wish and look sources)

## Wire

- **WebSocket**, **binary** frames, postcard body.
- Full **f32** for positions and angles on the wire.
- **`PlayerId`**: server-assigned **`u32`**.
- Direction is type-level: **`ClientToServer`** and **`ServerToClient`** only.
- Sim and client input structs stay off the wire; each side maps to net DTOs at the boundary.

### Framing (conceptual)

| Direction | Body |
| --- | --- |
| C→S | `ClientToServer` |
| S→C | `ServerToClient` |

Slice modules under `game-net` (e.g. session, movement, world) own their variants. Encode/decode is pure `bytes ↔ message`.

### Version

- **`protocol`**: `u16`. Hello/Welcome carry it. Mismatch ends the join attempt cleanly.
- **`content_rev`**: `u32` (cook/content stamp). Carried on Hello/Welcome. Mismatch → message discarded (silent).

## Session key (folded)

Server mints a recycled key; client echoes it on every mutating C2S. One policy: client copies what the server last sent.

| Message | Key role |
| --- | --- |
| **Welcome** | Initial `key` + `issued_tick` |
| **Snapshot** | Current `key` + `issued_tick` (always present; rotation is a new value) |
| **Input** | `echo_key` + `echo_issued_tick` from last accepted server values |

MVP may use a trivial server-chosen value. Client always echoes. Server checks echo against current key before applying input.

## Time

- Server runs a **fixed tick**.
- Game messages carry **client `seq`** (C→S) and **server `tick`** (S→C) where they apply.
- Client render frame rate stays independent of server tick.

## Solo and join (contract for later features)

- Page load runs **solo**: local `SelfState` advances as today; no server messages required for a full self.
- Multiplayer join is a later client action (023). While joined, local self **yields** to server authority; remotes appear from snapshots (024).

## Acceptance criteria

- Workspace includes `game-sim`, `game-net`, `game-server`, and `game-client` with the roles above.
- `game-net` exposes postcard encode/decode for `ClientToServer` and `ServerToClient` and the version fields.
- Server listens on a **port separate** from the Vite/static app.
- Client `mp/` is the sole home of browser multiplayer session code; solo path remains loadable without it.
- Wire uses binary WebSocket + postcard; `PlayerId` is `u32`; positions/angles are f32.
- Welcome/Snapshot carry session key + issued tick; Input carries the echo fields.
- Fixed server tick exists and advances even with a single connection.
- Documentation matches this layout and message direction model.
