# Feature 033 - MP shared clock reset

Course-correct multiplayer after **022–032**. Those features remain in the tree as **immutable time capsules** (docs and git history). This feature does **not** rewrite them. It states the baseline philosophy going forward and **removes code** that diverges from it so later work can follow that model cleanly.

Prefer **delete** over reinterpretation. Do not keep land schedules, adaptive present delays, or peer-local time buffers under new names. If a symbol’s only job was the discarded model, remove it.

## Philosophy (product law)

There is a **shared clock** \(T\) synchronized across the server and every client. At any moment we treat \(T\) as a common reference. This is **non-negotiable** — it is the only reliable way to order claims and reason about causality.

Every client runs its **own simulation in its own present**. Inputs, movement, and especially outcomes (hits, kills) are **claims** stamped with a time on the shared clock. Because of transmission delay these claims always arrive late.

The server does **not** run the single authoritative simulation of the world. Its job is to **relay** claims and to **reject** those that are clearly impossible relative to the shared timeline and previously accepted state. **Clients are the primary authors of their local reality.**

When late information arrives, a client **may** correct, but the default posture is to **preserve local experience**. Accurate prediction remains necessary only to keep those corrections small. Every compensatory technique exists to protect the client’s claimed reality from being needlessly disrupted.

### Vocabulary this implies

| Term | Meaning |
| --- | --- |
| **Shared clock (\(T\))** | One synchronized time base; the same \(t\) is what “now” and “then” mean when ordering claims and discussing causality. |
| **Local present** | Each client’s live sim and feel of play — advanced under shared rules, owned by that client until late claims force a choice. |
| **Claim** | An input, movement sample, or outcome (hit, kill, …) stamped with a time on \(T\). Claims are assertions about a moment, not permission requests that wait for a central world step. |
| **Relay** | Server forwards accepted claims to peers (and may echo) so others can integrate them into their local presents. |
| **Reject** | Server drops claims that are clearly impossible given \(T\), prior accepted claims, and shared rules (e.g. speed, reach, timeline order). Reject is a coarse filter, not full world authorship. |
| **Correction** | Optional client reconcile when late peer (or reject) information contradicts local belief. Not the default every packet. |
| **Prediction** | Advancing or filling local belief so play stays responsive and so necessary corrections stay small — in service of protecting claimed local reality, not of chasing a server-owned present. |

Anything that treats each peer’s uplink, RTT, or view lag as a **private timeline** instead of delay on a **shared** timeline is off this baseline. Anything that makes the server the **sole author** of world state (full authority sim + hard Snapshot overwrite of local self) is also off this baseline.

## Why 022–032 are banked, not fixed in place

**022–032** explored join, authority, remotes, predict, remote interp, present clock, adaptive delay, high tick rate, net HUD, and input land time. Useful learning; wrong multiplayer story for this product.

Notable divergences from the philosophy above (illustrative, not exhaustive):

| Area (features) | Divergence |
| --- | --- |
| Server authority world + Snapshot `you` (**023–026**, **032**) | Server owns the sim step and overwrites local self; clients are not primary authors of local reality. |
| Remote present delay / adaptive RTT delay (**027–030**) | Draw remotes on a **personal** lag offset tuned from own RTT, not integration of claims on one shared \(T\). |
| Input land delay (**032**) | Stall body channels to a published land schedule so “everyone commits together” via **per-peer uplink tax**, not stamped claims on \(T\) with late arrival as the normal case. |
| Net HUD lag fields (**031** + **032** extensions) | Instrumentation for the discarded delay / land model. |

Docs for **022–032** stay as written. Implementation of that path is what **033** erases.

## Scope

### In

- Record the philosophy above as the multiplayer baseline for all later features.
- **Remove** playable / joined multiplayer code and wire that implements the **022–032** temporal and session model (see [Removal](#removal)).
- Leave **solo** self (through **021**) as the working product path: input session, look/present, walk, jump, sprint, loadout, kits, cook, debug tools unrelated to net lag.
- Update root **README** (and any live CONTRIBUTING pointers that claim current MP behaviour) so they no longer describe join, remotes, land delay, or adaptive present delay as product behaviour.
- Raise or reset **`protocol`** / strip message shapes so mixed old/new binaries do not silently half-speak the removed model (if any net types remain as stubs).

### Out

- Implementing shared-clock sync details, claim wire formats, relay/reject policy, peer integration, correction, or prediction (**later features**).
- Combat, hit registration, or new gameplay.
- Rewriting or “clarifying” **022–032** feature docs.
- Salvaging `land_delay`, `L_min`, `rdelay`, remote present clocks, or RTT→delay clamps under new names.
- Keeping dead joined code paths “for reference” in the working tree (git history is the reference).
- Building a server-side full-world sim “for validation” that quietly becomes the old authority model.

## Removal

Err on the side of **deleting modules and call sites**. After **033**, multiplayer is **not** a playable mode.

### Client (`game-client`)

Remove the joined multiplayer surface, including but not limited to:

- The **`mp/`** tree as a working session: transport, join/leave, outbound Input, inbound Snapshot apply, land queue / body stall, predict-reconcile, remote pose table, RTT→delay (`lag`), remote present sampling.
- Frame-loop branches that yield self to land/authority or push `push_input_land` (or any successor name of that path).
- Console **`mp join` / `mp leave` / `mp status`** (and help text).
- Dev **net lag HUD** fields and wiring that exist only for RTT / rdelay / land / stall / Lmin / Lme / corr (**031**/**032**). A solo **FPS-only** strip may remain if it has no net dependency; otherwise remove the banner with the net fields.
- Any non-`mp/` helpers whose sole consumer was remote interp or land (e.g. dedicated remote present helpers if nothing else uses them).

Solo mount, flycam, lineup, screenshot, and other non-net debug tools stay. Local solo sim remains the only self advancement path.

### Server (`game-server`)

Remove the **022–032** authority world: session key input gate as product path, land schedule / uplink EMA, buffered land apply, Snapshot broadcast of `you`/peers/land fields, and join lobby behaviour tied to that model.

After removal the crate may be:

- a **minimal stub** binary (e.g. bind and refuse or no-op) so the workspace member still builds, or
- reduced further if the workspace no longer needs it —

but it must **not** still implement the discarded land/snapshot multiplayer. Prefer no fake “almost works” host.

### Wire (`game-net`)

Remove or gut DTOs and codec paths that only serve the discarded model: land fields on Snapshot, intent-stamp land path on Input, remote pose streams as currently shaped for **024–032**, and version constants documented solely for that stack.

If `game-net` remains as a workspace crate, it should not export a protocol that implies the old session is still live. Empty or near-empty module layout is fine until a later feature defines claim/relay messages on shared \(T\).

### Shared sim (`game-sim`)

**Keep** pure movement / self / loadout rules used by solo (and later by clients and by server reject checks). Do not add net clocks here in **033**. Remove only glue that exists solely for the discarded MP path (unlikely if sim stayed pure).

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

1. **Shared clock** \(T\) synchronized across participants (common reference for order and causality).
2. **Each client** advances its own local present with shared `game-sim` rules.
3. **Claims** (input, movement, outcomes) stamped with time on \(T\); they always arrive late.
4. **Server** relays accepted claims and rejects clear impossibilities — it does **not** own a single authoritative world sim.
5. **Peers** integrate late claims into local present; **default is preserve local experience**; correct only when needed.
6. **Prediction** and other compensation exist to keep corrections small and protect claimed local reality — not to invent private timelines or reinstall full server authority.

Compensatory techniques (interpolation, clock sync details, soft reconcile, etc.) are justified **only** as tools for that posture — not as alternate time models and not as a return to Snapshot-as-truth for the local self.

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
- No new prediction, clock sync, claim relay, or reject policy is required for **033** to be complete — only the philosophy text plus the wipe.
