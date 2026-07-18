# Feature 006 - Flycam

The camera is a view attached to the player self, not a free-standing world object. Until a body exists, that mount is a stub at the shared eye-height pose (002). Developers need a debug **flycam** that unmounts the view so they can inspect the scene anywhere, then remount to the self vantage.

## Acceptance Criteria

- One render view; pose comes from a **mount** (self) or a debug **flycam** controller — not parallel camera systems.
- The default/game path is mounted. Today's fixed eye-height, look-ahead pose is the stub self until a real body owns the mount.
- Flycam is a debug-only unmount for free inspection. It is feature-gated with the debug tools surface; production builds do not expose it.
- Flycam is view-only: it does not move, spawn, or substitute a player body in sim. Body noclip or possess is a separate concern later if needed.
- Enter/leave flycam and restore the self pose go through the debug command/cvar registry (003). Panels, keybinds, and host bridges only invoke that registry.
- Shared ground-truth quantities (units, axes, eye height) stay in `game-sim`; the client consumes them for the mount and does not redefine them.
- Out of scope: real character body and gameplay look/move, orbit-around-target editors, multiplayer spectator/admin, and networked free-cam/RCON.
