# Feature 028 - MP remote present clock

Remotes from **027** only update present time when a Snapshot arrives, so peers step at tick rate. Advance a client **present clock** every render frame so interpolation runs at display rate.

Depends on **027**. Wire and server tick unchanged (**30 Hz**). Delay stays **`REMOTE_INTERP_DELAY_SECS`** (100 ms).

## Problem

`present_t = tick_to_secs(last_tick) − delay` freezes between Snapshots. Lerp picks a point on a segment, but that point does not move until the next packet — remotes look delayed, not smooth.

## Clock

Keep on the client remote table (or equivalent):

| State | Role |
| --- | --- |
| `server_clock` | Estimated server time in seconds (same units as `tick / TICK_HZ`). |
| Samples | Unchanged: per-id ring of `(tick, pose)`. |

**On Snapshot** (tick `T`):

- Push `others` as today.
- `server_clock = max(server_clock, tick_to_secs(T))`.
- If `server_clock` is far behind `tick_to_secs(T)` (large stall / tab return), snap forward to `tick_to_secs(T)` (simple threshold; no soft blend required).

**Each render frame** (joined, remotes active):

- `server_clock += dt` (frame delta, same as the client frame loop).
- Do not run the clock past the newest sample time by more than a small epsilon if desired; underrun already **holds last** pose.

**Present:**

- `present_t = server_clock − REMOTE_INTERP_DELAY_SECS`
- `sample_at(present_t)` per peer as in 027 (lerp / hold / oldest rules unchanged).

Reset `server_clock` on leave / clear remotes.

## Out of scope

- Adaptive delay / RTT estimator.
- Extrapolation past newest sample.
- Changing tick rate or delay defaults.
- Self prediction (**026**).

## Acceptance criteria

- With a moving peer and normal Snapshot delivery, remote position advances **every render frame**, not only on Snapshot receive.
- Delay remains ~100 ms behind the estimated server timeline.
- Buffer underrun still holds last pose; no long-range extrapolate.
- Join / leave does not leave a stale clock driving empty tracks.
