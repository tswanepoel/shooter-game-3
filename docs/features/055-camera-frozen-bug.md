# Feature 055 - Soft look freeze after spawn

Slow horizontal mouse look did nothing after spawn; fast flicks still twitched. One `set_look(spawn.yaw)` was enough.

**Cause:** `spawn_pose` mapped `(seed >> n) as f32 / u32::MAX` without truncating to 32 bits, so yaw was kilradians. That stamped a huge `facing`; `f32` sin/cos lost precision and tiny `apply_look` deltas stopped moving the camera.

**Fix:** map spawn bits through `u32` (`unit01` / `unit_turn` in `game-server` roster). Restart the server process after the fix (client rebuild alone is not enough).
