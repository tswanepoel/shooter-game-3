# Feature 029 - MP remote adaptive delay

Tune how far behind remotes are drawn from **measured own lag**, so same-region play can sit tighter and high-RTT paths get more buffer. Present clock (**028**) and pose buffers (**027**) stay; only the delay value becomes dynamic.

Depends on **026** (`ack_seq`), **027**, **028**. No wire or server changes.

> **028** is the frame present clock. This feature is **029**.

## Problem

Fixed `REMOTE_INTERP_DELAY_SECS` (100 ms) is a one-size default: a bit soft on LAN, a bit thin when RTT and jitter are large. The client already learns authority lag via Input `seq` + Snapshot `ack_seq`.

## Measure lag

Client-only RTT to authority (includes net + server queue/tick, not pure ICMP ping):

1. On each sent **Input**, record `(seq → send_time)` (frame / performance clock).
2. On **Snapshot** with `ack_seq`, if that seq is in the map:  
   `sample_rtt = now − send_time[ack_seq]`.
3. Drop timestamps with `seq <= ack_seq`.
4. Smooth: `rtt_ema = mix(rtt_ema, sample_rtt)` (simple EMA; alpha fixed in impl).
5. Cap the send-time map (same order of magnitude as predict history).

No sample yet → keep the default delay until the first ack.

## Delay from lag

Replace the constant present offset with a live value:

```
delay = clamp(k * rtt_ema + jitter_pad, DELAY_MIN, DELAY_MAX)
```

| Symbol | Role | Starting defaults (impl may tune) |
| --- | --- | --- |
| `k` | Scale RTT → view delay | **0.5** (one-way-ish) |
| `jitter_pad` | Extra cushion | **0** v1, or small fixed (e.g. 10–20 ms) if underruns show up |
| `DELAY_MIN` | Floor so a lucky ping does not underrun | **80 ms** |
| `DELAY_MAX` | Ceiling so spikes do not bury peers | **200 ms** |
| Default before samples | Same as today’s fixed feel | **100 ms** |

- Present: `present_t = server_clock − delay` (028 clock unchanged).
- Smooth **delay** itself lightly if needed so it does not jump every Snapshot (either EMA on RTT only, or also EMA on `delay`).
- Do **not** set delay = raw RTT with no clamp.

## Scope

| In | Out |
| --- | --- |
| Client RTT from `ack_seq` | Server ping messages |
| Dynamic remote interp delay | Hit lag compensation / rewind |
| Clamps + EMA | Per-remote delay, adaptive tick rate |

## Acceptance criteria

- After join and a few acked Inputs, client has a non-zero smoothed RTT sample path.
- Remote present delay moves with RTT inside **[DELAY_MIN, DELAY_MAX]**; never uses unclamped raw RTT.
- Low stable RTT → delay near the floor; high stable RTT → delay toward the ceiling.
- First frames before any ack still present remotes (default delay).
- Leave / solo clears RTT map and resets delay state.
- 028 still advances present every frame; underrun still holds last pose.
