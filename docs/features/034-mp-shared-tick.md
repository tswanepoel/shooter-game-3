# Feature 034 - Shared tick

The server runs a shared clock at **30 ticks per second**. While a client is joined, it keeps a local estimate of the server’s current tick so both sides can talk about the same moment in time even though messages take time to cross the network. Solo play is unchanged. Depends on the multiplayer baseline in **033**.

## Layout

| Piece | Role |
| --- | --- |
| `game-sim` | Pure rules. No sockets. |
| `game-net` | Wire DTOs, postcard codec, protocol version. No sim apply, no GPU. |
| `game-server` | Native host: WebTransport, fixed tick, session accept, clock replies. |
| `game-client` | WASM. Solo self as today. |
| `game-client` **`mp/`** | Transport, join, clock estimate. |

Sim and client input structs stay off the wire; each side maps to net DTOs at the boundary.

## Connection

Multiplayer uses **WebTransport** (QUIC over UDP). A reliable stream carries the session: join, welcome, and leave. Unreliable datagrams carry clock samples. In the dev console, `mp join`, `mp leave`, and `mp status` open and close that session against the game server process.

## Wire

Binary **postcard** bodies. Direction is type-level:

| Root | Variants in this feature |
| --- | --- |
| `ClientToServer` | Hello, clock probe |
| `ServerToClient` | Welcome, Reject, clock reply |

Session variants use the reliable stream; clock variants use datagrams. Encode/decode is pure `bytes ↔` those roots in `game-net`.

- **`protocol`**: `u16` on Hello / Welcome. Mismatch ends the join attempt cleanly.
- **`PlayerId`**: server-assigned **`u32`** on Welcome. Client holds it as local identity for this session.
- Welcome also carries enough server time/tick for the client to seed its clock estimate.

## How the client tracks server time

The server stamps time on clock replies. The client measures round-trip delay and computes an **offset** from its local clock to the server’s, using the usual four-timestamp exchange (client send, server receive, server send, client receive). It prefers samples with lower delay and eases the offset toward new estimates rather than jumping on every packet.

The tick shown in the UI is this **estimated server tick** derived from local time plus the disciplined offset. Welcome may seed the estimate; ongoing samples keep it aligned under lag.

## Dev HUD

In debug builds, a top banner (toggle `nethud` / `hud.net`, on by default) shows smoothed **fps**. While joined it also shows the estimated server **tick**. That reading is how we verify the clock is shared. `mp status` may print offset, delay, and `PlayerId` for deeper checks.

## Acceptance criteria

- The server advances a 30 Hz tick; `mp join` connects over WebTransport and receives a live shared time.
- Two clients joined on a LAN show the same HUD tick within about one step of each other.
- The HUD tick comes from the client’s offset estimate of server time.
- `game-net` exposes postcard encode/decode for `ClientToServer` and `ServerToClient` covering Hello, Welcome, Reject, clock probe, and clock reply.
- Welcome assigns a `PlayerId` (`u32`); protocol mismatch rejects join.
- Solo play and production builds without debug tools behave as they do today, with join and the banner available through the existing dev tooling path.
- The root README mentions the server, `mp join`, and the fps/tick banner.
