# Feature 007 - Input session

The game owns pointer and keyboard for a single **in-game session**, not per mode. Browser rules only define the edges: a user gesture is required to enter, and the browser may eject (e.g. Esc). Inside the session the paradigm is fixed — no mode-specific browser lock/unlock. Flycam and other variants only change how we interpret input (006); they do not participate in browser capture.

## Acceptance Criteria

- **Session active** after a canvas user gesture (first click / click-to-resume). While active, the client owns relative look and game keyboard handling; the system cursor is not part of the product surface.
- **Session inactive** when the browser releases ownership (Esc, blur, tab away, etc.). Game modes (mount, flycam, console) do not change. Resume is the same gesture path as first enter — not a special flycam or debug flow.
- Enter/leave session is only those browser edges. Toggling flycam, remount, console, screenshot, or other game features must not request or release browser pointer lock.
- One input path for all consumers: mount, flycam (006), and later gameplay read the same session-relative mouse and keys. No parallel “debug capture” vs “game capture.”
- Applies in production and dev alike (not gated on debug-tools). Debug tools remain feature-gated; the session does not.
- Out of scope: engine-drawn cursor/menus, Keyboard Lock API, treating Esc as a game command that means “leave flycam,” and mode-specific lock policies.
