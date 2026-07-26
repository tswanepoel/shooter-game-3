# Feature 005 - Shot watch (agent)

Optional agent DX on top of 004: arm a listener and react to new shots. Capture stays 004; this feature never owns pixels.

## Acceptance Criteria

- Depends on 004 artifacts only (`debug/shots/latest.png`). Game code does not call agents or require a watcher.
- “watch me” (or equivalent) starts a folder listener; “stop watching” stops it. Capture (F9) remains available either way.
- On visual dilemmas, the agent may proactively arm listen and invite F9 (e.g. “do it — I’m watching”) without the user saying “watch me” first — via skill/instructions, not game logic.
- On each `latest.png` update while listening: read the image and continue the conversation (no user paste/path).
- Solo/dev use without an agent is unchanged and must not regress (004 independence).
- Out of scope: continuous frame streaming as default, embedding agent SDKs in the game, Grok-only capture APIs.
- Skill (or equivalent) documents triggers, arm/stop, and the F9 + `latest.png` contract.
