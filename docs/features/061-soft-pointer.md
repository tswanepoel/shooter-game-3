# Feature 061 - Soft pointer under input session

While the [input session](007-input-session.md) is active, the client owns a **soft pointer**: an engine-drawn cursor and canvas-local absolute position fed to egui. Product Gate / Panel chrome (join, role, character, loadout) runs under the same lock as look. Session mouse deltas go to the soft pointer when it is armed, otherwise to look (and to the emote wheel when that path is open).

Depends on **007** (session edges), **051**–**053** (product chrome). Revises **007** so engine-drawn cursor/menus are in scope. Presentation kinds land in **063**.

## Session

| Edge | Behaviour |
|------|-----------|
| Enter | Canvas user gesture → pointer lock (**007**) |
| Leave | Browser eject only (Esc, blur, tab away, …) |
| Resume | Same canvas gesture as enter |

Phase and menu transitions keep the lock. One session path for look and menus.

## Soft pointer

| Fact | Draft |
|------|--------|
| Position | Canvas-local `Vec2`, clamped to the game view |
| Motion | Session `movement_x/y` while armed |
| Draw | Engine cursor while armed |
| egui | `PointerMoved` / `PointerButton` from the soft position while locked |
| Keyboard | Product UI phases take keys while the session is active |
| Armed when | Gate / Panel product chrome (**063**); debug console rides the same soft pointer when the session is active |

When soft pointer is armed, LMB is UI click. When it is not, Living / spectate look and fire use session input as today.

## Acceptance criteria

- One lock gesture covers join → role → character → loadout; soft pointer drives those controls.
- With soft pointer disarmed, mounted / flycam look and fire use session deltas.
- Browser eject ends the session; canvas click resumes it.
- Soft pointer ships in production and debug-tools builds.
