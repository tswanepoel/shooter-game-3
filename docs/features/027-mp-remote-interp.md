# Feature 027 - MP remote interpolation

Draw **other** players from a short pose history, slightly in the past, so motion is smooth between Snapshots. Self stays **026**.

Depends on **024** and **026**. Server tick **128 Hz**; Snapshot contents unchanged.

## Present delay

Draw remotes at **now − 20 ms** (starting default; raise only if delivery needs it). Prefer server **`tick`** as sample time.

## Buffer

- On Snapshot: append each `others` pose + tick into a per-id ring.
- **PlayerLeft** / gone: drop that id (and mesh) as today.
- Present: find the two samples around present time; **lerp** continuous fields (position, phase, angles as needed); discrete fields (loco mode, loadout, character) from the newer sample.
- One sample only → show it. Present after newest → **hold** last (no extrapolate). Present before oldest → use oldest.

Remote present path unchanged (drive → mesh); drive is the interpolated pose.

## Acceptance criteria

- Peers move continuously between Snapshots at **20 ms** delay.
- Stationary remote does not pop when the local player turns or strafes (normal delivery).
- PlayerLeft still removes promptly; underrun holds last pose.
