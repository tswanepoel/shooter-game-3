# Feature 035 - MP remotes

Joined clients **see other players**. Each peer keeps advancing its own local self (solo rules through **021**), **publishes** a drive sample stamped with shared **tick**, and the server **relays** it to the others. Receivers draw remotes with the same **present pose** path the self uses for body and locomotion.

**MVP present:** paint the latest sample as it arrived — remotes sit in that past state in the local present. The goal is to **see** raw late remotes, learn the feel, and keep the wire and table ready for interpolation later.

Depends on **033** (baseline) and **034** (join, WebTransport, shared clock / tick estimate). Keep that clock as the correlation spine.

## Layout

| Piece | Role |
| --- | --- |
| `game-sim` | Pure drive → present pose. |
| `game-net` | Drive + presence DTOs on the directional roots; every new variant carries **tick**; postcard codec; protocol bump. |
| `game-server` | Roster; relay drive samples to other joined peers. |
| `game-client` | Local self stays client-authored while joined. |
| `game-client` **`mp/`** | Publish own drive; remote table keyed by id with **tick + drive**; feed presentation. |

## Shared tick on the wire

**034** already gives a common \(T\). This feature **uses** it:

- Every C→S / S→C variant added here carries a **`tick: u64`** in the same units as **034** (`TICK_HZ`): the **sender’s tick at send time**, left unchanged on relay.
- Presence (peer joined / left) also carries the sender’s **tick** at the event.

Stamps stay on the shared clock so peers can correlate later.

## Drive sample (net)

Enough to rebuild **present pose** for walk, sprint, jump, and stand (**016** / **019** / **020** / **021**):

| Field group | Content |
| --- | --- |
| Stamp | **`tick`** (shared \(T\)) |
| Identity | `PlayerId`, character, active weapon (and slots if mesh needs them) |
| Placement | position, ocular yaw/pitch |
| Locomotion | mode, phase, air/jump state as required for present |

Sim and client structs stay off the wire; map at the boundary.

### Publish

While **joined**, advance self with existing solo rules, then send drive samples from that local drive (about **`TICK_HZ`**), each with **tick**. The sample is for peers only — local self keeps its own drive.

### Receive / table

Relayed samples update a **remote table**: `PlayerId` → at least **latest `(tick, drive)`**. Store tick with drive so ordered samples on \(T\) are ready for later interpolation.

**MVP draw:** use the **latest received** sample’s drive as-is (arrival order / replace on receive). Optionally keep a short ring of recent `(tick, drive)` now if cheap; latest alone is enough for acceptance when the field layout already carries tick.

In the local present, remotes look late and stepped. That is intentional learning surface.

## Server

- Keep the joined roster (**034**).
- On a drive sample from peer \(i\): if \(i\) is joined, **relay** the sample (including **tick**) to every other joined peer.
- Drop garbage (decode failure, not joined).

Server only relays; peers keep authoring their own selves.

## Presence

| Event | Role |
| --- | --- |
| Peer visible | Others learn `{ id, tick, … }` so a remote slot exists (first drive sample may supply pose). |
| **PeerLeft** `{ id, tick }` | Drop the remote on disconnect / leave. |

Reliable stream for join/leave; datagrams fine for high-rate drive samples (same split idea as **034** clock probes).

## Presentation

- Remotes: **present pose** only (**017**) — body and active blaster from the drive used this frame (MVP: latest sample).
- Local self: first-person look-mounted as today; remotes are third-person bodies.
- Same kit rebuild as solo; **`mp/` supplies drive only**.

## Wire

Bump **`protocol`**. Add (all with **`tick`**):

| Dir | Role |
| --- | --- |
| C→S | Drive sample (`tick` + drive view) |
| S→C | Relayed peer drive (`tick` + id + drive); peer joined / peer left (`tick` + id, …) |

Keep Hello / Welcome / Reject / clock probe / clock reply from **034** unchanged in role. Shared clock path stays the correlation spine.

## Foundations for later

This feature leaves the wire and table ready for:

- Present time on \(T\), interpolation between stamped samples, adaptive delay, extrapolation.
- Soft correction of local self, hits, combat claims.
- Movement validation beyond joined + decodes.

## Acceptance criteria

- Two clients joined to the same host each see the other’s body in the world.
- Remote walk, sprint, jump, and stand follow the peer’s latest relayed drive.
- Local self while joined stays client-authored.
- Leave / disconnect removes the remote on other clients.
- Remote meshes use existing present/kit paths; `mp/` supplies drive only.
- Drive and presence messages carry the **sender’s tick** at send time (shared-clock units).
- MVP presentation is latest-sample paint only.
- Protocol bumps; `game-net` round-trips the new variants; solo and clock estimate stay as today.
