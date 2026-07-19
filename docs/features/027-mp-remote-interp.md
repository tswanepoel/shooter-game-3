# Feature 027 - MP remote interpolation

Draw **other** players from a short pose history, slightly in the past, so motion blends between Snapshots. Self stays **026**.

Depends on **024** and **026**. Server tick is shared **`game_net::TICK_HZ`** (30). Snapshot contents unchanged.

## Present delay

Draw remotes at **present − 100 ms** (`REMOTE_INTERP_DELAY_SECS`). Sample time is server **`tick`** mapped with `TICK_HZ`.

## Buffer

- On Snapshot: append each `others` pose + tick into a per-id ring.
- **PlayerLeft** / gone: drop that id (and mesh) as today.
- Present: find the two samples around present time; **lerp** continuous fields (position, phase, angles as needed); discrete fields (loco mode, loadout, character) from the newer sample.
- One sample only → show it. Present after newest → **hold** last (no extrapolate). Present before oldest → use oldest.

Remote present path unchanged (drive → mesh); drive is the interpolated pose.

Present time in v1 is derived from the latest received tick (advances only on Snapshot). Continuous present clock is **028**.

## Acceptance criteria

- Peers present from a buffered history at **100 ms** delay.
- Stationary remote does not pop solely from local camera motion (normal delivery).
- PlayerLeft still removes promptly; underrun holds last pose.
