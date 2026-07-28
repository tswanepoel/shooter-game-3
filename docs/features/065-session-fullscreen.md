# Feature 065 - Session fullscreen

Canvas enter/resume for the [input session](007-input-session.md) also requests browser fullscreen on the canvas. Pointer lock runs **after** fullscreen settles; locking in the same turn as the fullscreen request is cancelled by the transition and leaves the session without raw capture until a second click.

Depends on **007**. Session leave remains browser eject only (Esc / blur / …); the game does not treat Esc as a command.

## Acceptance criteria

- First canvas click enters fullscreen and then pointer lock (raw movement when the browser allows it).
- When already fullscreen, canvas click requests pointer lock only.
- Fullscreen denial still attempts pointer lock.
- Enter/leave edges stay browser-owned; game modes do not request or release fullscreen or lock on their own.
