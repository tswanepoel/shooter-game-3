# Feature 032 - MP input land time

While joined, every client **sends Input eagerly** and applies **body channels** only at a server-authored **land delay**, so local sim and authority commit the same command at the same shared time. **Look** stays session-immediate. Solo is unchanged.

Depends on **023**, **026** (`ack_seq`, joined self path), **030** (`TICK_HZ`), **031** (dev HUD). Remote present delay (**027–029**) stays the S→C path.

This feature **replaces immediate body predict (026)** while joined: body follows land time; Snapshot hard-correct remains for late and divergent apply.

## Land delay

Server measures each peer’s **uplink** and publishes that peer’s **land delay**. Client stalls body-channel apply by the published value. Look is applied every frame from session input and is carried on Input for authority, but is not delayed by land time.

```
land_delay_i = L_min + E_i + T_tick
             = L_i + T_tick
```

| Symbol | Time it represents |
| --- | --- |
| `L_i` | Smoothed **uplink** for peer *i*: intent time → server recv (one-way on the server clock after offset). |
| `L_min` | **Session floor**: min of joined peers’ `L_j`. How soon the best uplink can deliver a command. |
| `E_i` | **`L_i − L_min`**: this peer’s extra uplink above the floor (personal tax; already inside `L_i`). |
| `T_tick` | **`1 / TICK_HZ`**: one authority step. Land is scheduled on tick boundaries; this is quantize-to-tick plus one tick of buffer after expected arrival so early packets wait in the schedule instead of racing the step. |

`TICK_HZ` is **128** (**030**). Impl may expose `T_tick` as a named const derived from it.

### Measure `L_i`

Favour server-side age over RTT/2 alone:

1. **Input** carries a client **intent stamp** (client clock at send).
2. Server keeps a light **clock offset** per session (Welcome + running samples from stamp vs recv).
3. Each Input: `sample = recv_server_time − map(intent_stamp)`; reject pathological samples.
4. `L_i = EMA(sample)` (alpha fixed in impl; half-life on the order of a few hundred ms of jitter, not a silent fudge).
5. Cross-check with existing **seq → ack** RTT (**029**) when stamps are thin; prefer age when both exist.

`L_min` is **server-only**: min over current peers’ `L_j`, from the same EMAs. Slew `L_min` and published `land_delay_i` so a join/leave or spike moves the floor over a short interval (order of ~0.25–0.5 s to traverse a step change), not in a single tick.

### Schedule (server)

For each accepted Input with mapped intent time `T`:

- **Land time** `T_land = T + land_delay_i` (quantized to the authority tick at or after that instant).
- **Early** (`recv` before land tick): **buffer**, apply on the land tick through `game-sim`.
- **Late** (recv at/after land tick): **apply on the current tick** and accept Snapshot correction on clients (**apply-and-correct**).

`ack_seq` remains the last seq applied into the sim that produced `you`.

### Apply (client, joined)

1. Sample session input. **Look** updates ocular state immediately.
2. Build and **send Input** immediately (eager): seq, key echo, intent stamp, look, wish, jump, sprint, weapon cycle as today.
3. Enqueue **body channels** (wish, jump, sprint, weapon cycle, and any other non-look self drive) until local time matches the server land schedule using published `land_delay_i` (and tick from Snapshots).
4. At land time, apply those channels with **`game-sim`** (same rules as server).
5. On Snapshot: hard-set body from `you` when late path or divergence requires it; keep local look; advance from `ack_seq` as today where history still applies.

Gates (console / flycam) stay as today. Leave clears land/uplink state and restores solo advancement (**023**).

## Wire

Extend joined messages (postcard as today). Raise **`protocol`** so mixed builds reject cleanly.

| Message | Add |
| --- | --- |
| **Input** | Intent stamp (client send time for uplink age). |
| **Snapshot** (to you) | `land_delay` (seconds or ms; full `L_i + T_tick`), `L_min`, and `L_i` (or reconstruct `L_i = land_delay − T_tick`) for HUD and stall. |

`ack_seq` and pose fields stay. Impl picks compact encodings (e.g. ms `u16` / `f32` seconds) consistent with existing net style.

## Dev HUD (**031**)

When joined, extend the top banner (same toggle). Keep the line short.

| Field | Source |
| --- | --- |
| **FPS** | Frame loop (031) |
| **tick** | Last server tick |
| **RTT** | 029 EMA |
| **rdelay** | Remote present delay (029); rename label if needed so it is not confused with land |
| **Lmin** | Snapshot session floor |
| **Lme** | Snapshot uplink `L_i` |
| **land** | Published `land_delay` / client stall target |
| **stall** | Client body-channel buffer in use (should track **land**) |
| **err** | Land alignment error (local body apply vs server apply / ack land); show ms |
| **corr** | Hard-corrects per second (or recent count) |

Solo: FPS only (031).

## Scope

| In | Out |
| --- | --- |
| Joined body-channel land schedule | Blaster fire / hit resolve |
| Eager send + server buffer / late apply-and-correct | Stall look |
| Uplink floor + personal excess as components of land delay | Changing remote interp rules |
| Snapshot-published delay + HUD | Solo path changes |

## Acceptance criteria

- Solo and leave behaviour match pre-032 self advancement.
- Joined: Input leaves as soon as sampled; body channels on self apply only after land delay; look still tracks the mouse every frame.
- Server applies buffered Input on the scheduled land tick; late Input still applies and clients converge via Snapshot.
- `land_delay ≈ L_i + 1/TICK_HZ` with `L_i ≥ L_min` and `E_i = L_i − L_min` visible on HUD (or derivable).
- Under stable LAN, mean **err** stays within about **one tick** (`T_tick`); forced extra uplink on one client raises that client’s **Lme** / **land** / **stall** and leaves a faster peer’s land near the floor.
- Two joined clients with different forced uplink: each stall tracks its own published land delay; shared `Lmin` reflects the faster uplink.
- Protocol mismatch rejects join; debug HUD shows the new fields when joined; release without debug-tools omits the HUD path.
