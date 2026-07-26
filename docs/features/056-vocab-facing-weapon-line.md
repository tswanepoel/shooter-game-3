# Feature 056 - Close dual vocabulary (facing / weapon line)

Leftover free-angle / Kick-era names after **054**. Prefer concept terms on public net, HUD, and combat paths.

**Changes:**
- `YouSpawned.yaw` → `facing` (spawn placement of the legs; protocol **11**). Client applies via `set_drive_look`, not `set_look`.
- Drop debug `kickhud` alias; residual HUD stays `residualhud` / `hud.residual`.
- Fire path locals speak `weapon_line`, not `aim`.

Restart client **and** server after pull (protocol bump).
