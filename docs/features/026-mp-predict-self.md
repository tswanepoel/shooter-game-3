# Feature 026 - MP predict self

While joined, the client advances **your** body with the same `game-sim` rules as the server, sends Input as today, and **hard-corrects** from Snapshot. Mouse look stays local and is never taken from `you`.

Depends on **023** and shared **`game-sim`**. Remotes stay latest-pose (**024**) until **027**.

## Wire

Bump **`protocol`**. Each Snapshot includes:

| Field | Role |
| --- | --- |
| `ack_seq` | Last `Input.seq` applied for this client into the sim that produced `you`. |

Server already tracks last applied seq; expose it (default `0` before any Input). Other Snapshot fields unchanged.

## Client

While joined (same non-move gates as today: console / flycam):

1. Apply look locally; sample move intents; send **Input**; append it to a seq-keyed **history**.
2. Advance local body with shared sim (same rules as server).
3. Present from **predicted** body + local look — not a full Snapshot overwrite.

On Snapshot with `you`:

1. Hard-set **body** from `you` (position, loco, phase, air/jump fields, loadout). Leave local yaw/pitch alone.
2. Drop history with `seq <= ack_seq`; **replay** the rest in order on that body.
3. That result is the new predict baseline.

Cap history for RTT; on overflow drop oldest and accept a larger correct next Snapshot. Snaps on mispredict are intended (no soft blend).

Solo and leave: unchanged solo path; leave clears history and returns coherent local pose.

## Server

Tick stays **128 Hz**. Input accept, key echo, seq, and pending merge unchanged. Snapshot carries `ack_seq` for the recipient. Server does not predict.

## Acceptance criteria

- Joined WASD / jump / sprint / weapon move the local body without waiting on Snapshot.
- Look stays instant; Snapshot does not overwrite local ocular angles.
- Protocol bumps; Snapshot has `ack_seq`; client resims only `seq > ack_seq` after hard-set body.
- Healthy matched rules: steady move has no repeated large snaps. Divergence still converges on later Snapshots.
- Leave restores solo; predict history cleared.
