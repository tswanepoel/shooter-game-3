# Feature 030 - MP 128 Hz tick + LAN-friendlier delay

Raise shared **`TICK_HZ`** to **128** and retune **029** delay clamps so LAN can sit tight. Adaptive delay math, present clock, and Snapshot shapes stay as today. Bump **`protocol`**.

Depends on **029**.

## Numbers

| | Was | Now |
| --- | --- | --- |
| `TICK_HZ` | 30 | **128** |
| `DELAY_MIN` | 80 ms | **32 ms** |
| Default (pre-sample) | 100 ms | **48 ms** |
| `DELAY_MAX` | 200 ms | **200 ms** |
| `k` / `jitter_pad` | 0.5 / 0 | unchanged |

Pose buffer must still cover `DELAY_MAX` at the new rate (raise cap if hold-last shows under load).

## Acceptance criteria

- Client and server run at 128 Hz; protocol mismatch rejects join.
- Low RTT → delay near **32 ms**; high RTT still climbs toward **200 ms**.
- Remotes stay smooth on the present clock; underrun still holds last pose.
