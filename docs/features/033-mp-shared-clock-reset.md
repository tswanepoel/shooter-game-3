# Feature 033 - MP shared clock reset

Course-correct multiplayer after **022–032**. Those features remain in the tree as **immutable time capsules** (docs and git history). This feature does **not** rewrite them. It states the baseline philosophy going forward and **removes code** that diverges from it so later work can follow that model cleanly.

Prefer **delete** over reinterpretation. Do not keep land schedules, adaptive present delays, or peer-local time buffers under new names. If a symbol’s only job was the discarded model, remove it.

## Philosophy (product law)

The server and every client run the **same simulation** under a **shared clock** synchronized across all participants. At any moment this clock is treated as a **single value** everywhere.

Treating time as universal is a deliberate choice. It could just as easily remain relative, but the universal treatment maps cleanly onto real-world intuition. That mapping is what gives the model its vocabulary and keeps it simple to reason about. Equivalent systems can be built without a global clock and still produce the same outcomes, but they become harder to hold in the mind, harder to evolve, and more prone to complexity and subtle bugs.

Because of transmission delay, client input always arrives at the server as a **claim about the past**. The server **simulates that past moment** and **replicates the resulting historical state** downstream. By the time the state reaches the client it is already an **even more distant past**. The client continuously **corrects** against it.

This is the **baseline**. **Accurate prediction** is required to keep these necessary corrections from degrading the experience. Every compensatory technique exists to make those predictions as accurate and reliable as possible — not to invent a different time model.

### Vocabulary this implies

| Term | Meaning |
| --- | --- |
| **Shared clock** | One synchronized time base; the same *t* is what “now” and “then” mean for sim and net discussion. |
| **Claim about the past** | An Input (or equivalent) stamped for a clock time earlier than server receive. |
| **Historical state** | Sim outcome at a past clock time; Snapshots (or equivalent) are records of that past, not a live “present” authority that erases client time. |
| **Correction** | Client reconciling local belief to received historical truth. |
| **Prediction** | Advancing local belief ahead of last known history so play stays responsive; must match shared rules so corrections stay small. |

Anything that treats each peer’s uplink, RTT, or view lag as a **private timeline** instead of delay on a **shared** timeline is off this baseline.

## Why 022–032 are banked, not fixed in place

**022–032** explored join, authority, remotes, predict, remote interp, present clock, adaptive delay, high tick rate, net HUD, and input land time. Useful learning; wrong temporal story for this product.

Notable divergences from the philosophy above (illustrative, not exhaustive):

| Area (features) | Divergence |
| --- | --- |
| Remote present delay / adaptive RTT delay (**027–030**) | Draw remotes on a **personal** lag offset tuned from own RTT, not a single shared sim time. |
| Input land delay (**032**) | Stall body channels to a published land schedule so “everyone commits together” via **per-peer uplink tax**, not by simulating stamped past claims on one clock. |
| Joined predict + hard Snapshot overwrite (**026**, then **032**) | Partial step toward correct-against-history, but wired to tick/ack and land machinery rather than universal-clock history + prediction. |
| Net HUD lag fields (**031** + **032** extensions) | Instrumentation for the discarded delay model. |

Docs for **022–032** stay as written. Implementation of that path is what **033** erases.

## Scope

### In

- Record the philosophy above as the multiplayer baseline for all later features.
- **Remove** playable / joined multiplayer code and wire that implements the **022–032** temporal and session model (see [Removal](#removal)).
- Leave **solo** self (through **021**) as the working product path: input session, look/present, walk, jump, sprint, loadout, kits, cook, debug tools unrelated to net lag.
- Update root **README** (and any live CONTRIBUTING pointers that claim current MP behaviour) so they no longer describe join, remotes, land delay, or adaptive present delay as product behaviour.
- Raise or reset **`protocol`** / strip message shapes so mixed old/new binaries do not silently half-speak the removed model (if any net types remain as stubs).

### Out

- Implementing shared-clock sync, past-claim Input, historical replication, correction, or prediction (**later features**).
- Combat, hit registration, or new gameplay.
- Rewriting or “clarifying” **022–032** feature docs.
- Salvaging `land_delay`, `L_min`, `rdelay`, remote present clocks, or RTT→delay clamps under new names.
- Keeping dead joined code paths “for reference” in the working tree (git history is the reference).

## Removal

Err on the side of **deleting modules and call sites**. After **033**, multiplayer is **not** a playable mode.

### Client (`game-client`)

Remove the joined multiplayer surface, including but not limited to:

- The **`mp/`** tree as a working session: transport, join/leave, outbound Input, inbound Snapshot apply, land queue / body stall, predict-reconcile, remote pose table, RTT→delay (`lag`), remote present sampling.
- Frame-loop branches that yield self to land/authority or push `push_input_land` (or any successor name of that path).
- Console **`mp join` / `mp leave` / `mp status`** (and help text).
- Dev **net lag HUD** fields and wiring that exist only for RTT / rdelay / land / stall / Lmin / Lme / corr (**031**/**032**). A solo **FPS-only** strip may remain if it has no net dependency; otherwise remove the banner with the net fields.
- Any non-`mp/` helpers whose sole consumer was remote interp or land (e.g. dedicated remote present helpers if nothing else uses them).

Solo mount, flycam, lineup, screenshot, and other non-net debug tools stay.

### Server (`game-server`)

Remove the **022–032** authority world: session key input gate as product path, land schedule / uplink EMA, buffered land apply, Snapshot broadcast of `you`/peers/land fields, and join lobby behaviour tied to that model.

After removal the crate may be:

- a **minimal stub** binary (e.g. bind and refuse or no-op) so the workspace member still builds, or
- reduced further if the workspace no longer needs it —

but it must **not** still implement the discarded land/snapshot multiplayer. Prefer no fake “almost works” host.

### Wire (`game-net`)

Remove or gut DTOs and codec paths that only serve the discarded model: land fields on Snapshot, intent-stamp land path on Input, remote pose streams as currently shaped for **024–032**, and version constants documented solely for that stack.

If `game-net` remains as a workspace crate, it should not export a protocol that implies the old session is still live. Empty or near-empty module layout is fine until a later feature defines shared-clock messages.

### Shared sim (`game-sim`)

**Keep** pure movement / self / loadout rules used by solo. Do not add net clocks here in **033**. Remove only glue that exists solely for the discarded MP path (unlikely if sim stayed pure).

### Docs and product text

| Keep immutable | Update to match code after wipe |
| --- | --- |
| `docs/features/022`–`032` (all of them) | `README.md` multiplayer / `nethud` / control tables |
| Git history through the **032** bank commit | Any other **live** contributor text that describes current MP as shipping |

Do **not** edit **022–032** to say “superseded” in a way that rewrites their acceptance criteria. **033** is the supersession record.

## What “done” looks like

| Surface | After 033 |
| --- | --- |
| Page load | Solo self as through **021** |
| `mp join` | Absent or non-functional (prefer absent) |
| Two browsers | No remotes, no shared world |
| `cargo run -p game-server` | Builds; does not host the old 032 session (stub or equivalent) |
| `cargo test` / clippy / fmt | Green |
| Feature docs 022–032 | Unchanged files on disk |

## Relation to later work

Later features reintroduce multiplayer **only** under this philosophy:

1. Shared clock synchronized across participants.
2. Same `game-sim` rules on server and clients.
3. Input as past claims; server simulates that past; replicate history.
4. Client corrects to history; predicts to hide correction.

Compensatory techniques (interpolation, input timing, clock sync details, etc.) are justified **only** as tools for accurate prediction and faithful shared-time sim — not as alternate time models.

**033** does not specify those mechanisms. It only clears the slate and cements the law.

## Acceptance criteria

- This document is the recorded multiplayer **philosophy** and the **reset** scope; **022–032** docs are untouched.
- Solo play matches pre-join behaviour through **021** (movement, look/present split, jump, sprint, loadout, kits, input session).
- No playable join path: no working land delay, no adaptive remote present delay, no remote bodies from the old Snapshot path, no authority yield that depends on that stack.
- Client has no live dependency on removed `mp/` land/lag/remote session code (tree deleted or reduced to non-joined stubs with no product entry points).
- Server and `game-net` no longer implement or advertise the **032**-era land/snapshot multiplayer session.
- README (and live product pointers) no longer claim **022–032** multiplayer behaviour as current.
- Debug net-lag HUD fields for the discarded model are gone; release builds still omit debug-only tools as today.
- Workspace builds and tests pass; prefer deleting unused code over leaving commented or dead joined paths.
- No new prediction, clock sync, or historical-replication feature is required for **033** to be complete — only the philosophy text plus the wipe.
