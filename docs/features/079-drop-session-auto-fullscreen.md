# Feature 079 - Drop session auto-fullscreen

Canvas click for the [input session](007-input-session.md) requests **pointer lock only**. It no longer calls `requestFullscreen` (**065**).

Depends on **007**. Session leave remains browser eject only.

## Behaviour

- First / resume canvas click → pointer lock (raw movement when the browser allows it).
- Fullscreen is optional / user-driven (browser UI / F11); the game does not request or exit it.
- Enter/leave edges for lock stay browser-owned.

## Acceptance

- Clicking the canvas does not enter fullscreen.
- Pointer lock still starts the input session as before.
