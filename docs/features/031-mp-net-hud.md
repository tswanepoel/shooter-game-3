# Feature 031 - Dev net / FPS HUD

Dev-only **top banner** with live FPS and lag readouts so delay floors and tick changes (029 / 030) can be tuned without digging in the console.

Depends on **003** (debug-tools) and **029** (RTT / delay already measured). Production builds omit it.

## Surface

- Thin strip along the **top** of the game view (in-engine overlay, not page DOM).
- Always visible in **debug-tools** builds while joined or always-on in dev — pick one in impl; default: **on in dev, toggleable**.
- Toggle: console command `nethud [on|off|toggle]` (same style as `grid` / `lineup`). Optional backing cvar `hud.net` if needed for get/set. On by default in debug is fine.

## Readouts (v1)

| Field | Source |
| --- | --- |
| **FPS** | Smoothed frame rate from the client frame loop |
| **RTT** | 029 EMA (ms); `—` until first ack |
| **delay** | Current remote present delay (ms) |
| **tick** | Last known server tick (joined only) |

Solo: show **FPS** (and optional `solo`); hide or dash the net fields.

Keep the line short — one row, monospaced or plain egui text. No graphs, no history sparklines.

## Out of scope

- Packet loss %, bandwidth, full net graph
- Server-side HUD
- Replacing `mp status` (console can stay; HUD is glanceable)

## Acceptance criteria

- Debug build can show a top banner with FPS; when joined, RTT and delay update as 029 samples.
- Release / no `debug-tools`: no HUD code path in the product surface.
- Toggle does not require leaving the input session; values refresh every frame (or lightly smoothed).
