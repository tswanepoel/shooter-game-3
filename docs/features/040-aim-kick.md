# Feature 040 - Aim kick

When you fire, the gun kicks. That kick is real sim state: it moves where shots go, where the reticle sits, and how the held blaster is drawn. One truth — not a separate visual jolt while bullets fly straight.

Look is still where you point. Kick is a short pitch/yaw offset (plus a little mesh shove on the grip) that builds on each shot and settles back. Projectiles and reticle use look plus kick. The old present-only jolt path goes away.

Kick strength and settle time come from the weapon table (per blaster / class), same place fire data already lives.

## Acceptance criteria

- Firing adds kick in sim; kick settles over time.
- Shots and reticle follow look + kick.
- Held weapon present follows the same kick.
- Present-only jolt is removed.
