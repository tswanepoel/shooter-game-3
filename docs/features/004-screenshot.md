# Feature 004 - Screenshot capture

Dev-only capture of the presented game frame to disk. No agent dependency.

## Acceptance Criteria

- Gated by `debug-tools`; stripped release builds expose nothing.
- One capture path: F9, console command, and host bridge all invoke it.
- Source is the presented canvas/frame (not page DOM chrome). Canvas read first; GPU readback only if needed, same API.
- Each capture writes git-ignored `debug/shots/latest.png` and a timestamped copy under `debug/shots/`.
- F9 works with the console closed; no listener or chat required for success.
- Out of scope: agent watch loops, streaming, video, Playwright-as-core, OS capture, browser download spam.
- Short README note: F9 / command, output paths, feature lever.
