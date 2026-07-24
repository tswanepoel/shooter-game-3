# Feature 043 - Health, hits, damage, death

FPS combat: **you shoot someone else; they take damage; enough hits, they die.** Health is real sim state — not chrome while the bar stays full.

**Present in this feature:** kit **`die`** collapse (self and remotes). **Later:** HUD, hit markers, blood, and other chrome.

Depends on **038** (combat projectiles: look origin → crosshair; muzzle is FX only), **042** (ammo mass on the projectile), and living remotes from the MP drive path (**035+**).

## Health

Every client’s **local present** tracks health for **each known player** (self and remotes): current, max, alive, and die age when dead. Sim owns the numbers. Drive pose alone does not carry health — remotes’ dead/alive and die age come from this present’s health after impact claims.

While below max, healing does not start immediately: after the last damage, a short cooldown must pass, then health regens until full or another hit resets the cooldown.

## Hits (firer earns the shot)

**Favour the firer:** a hit on **their** screen is earned. Their client decides collide in its local present on **own** combat projectiles (**038** path).

- Live own projectiles test against **other** living bodies as **posed body-part meshes** (same present pose as draw: head, torso, limbs — triangle collide, not a capsule). Not the firer’s own body. All parts use the same impact rule (no head multiplier in this feature).
- Contact is projectile / mesh collide at a moment. The firer claims that **impact hit** (target + ammo + speed at contact, stamped on the shared clock). Server **relays**; it does not re-simulate the shot.
- **Apply once** in each present: when the firer claims (local), peers apply when the relay arrives. No second apply of the same claim (server does not echo the firer).
- **Projectiles on the wire are VFX only** (**038** tracers / flash). Peers do **not** collide those for combat. Damage rides the hit claim.

Two shooters racing a kill: first accepted claims win in each present; fair credit / last-hit rules are **out of scope** for this feature.

## Impact → health

Receivers **translate** the impact hit with the shared rule: damage from **speed at contact** and **ammo mass** (**042**). Monotonic in mass and speed — not a flat “this gun always does N.” Gravity / long flight can soften hits.

That damage drops the **target player’s** health in that present. At zero → dead.

## Death

Dead: no walking, firing, or other living actions. Dead stays dead here — full respawn later.

Every present that accepted the lethal impact treats that player as dead (stop living actions / stop scoring further hits on them).

**Present:** Kenney character clip **`die`** (full-body collapse). Age (`die_age_s`) from death; sample the clip by age and **hold the last frame** when the clip ends. Holster blaster. Self and remotes in each present (remotes from local health, not drive).

## Acceptance criteria

- Firer’s fire can land hits on **other** living body-part meshes (remotes); hit on firer screen is the earned outcome.
- Near-miss past the mesh does not score; only triangle collide on the posed body.
- Impact hits are **claims** (target + ammo + speed); peers translate to health drop — not by colliding relayed projectiles.
- Each claim applies **once** per present (no double application).
- Damage from impact (mass × speed rule), not a fixed per-shot constant alone.
- Slower impact → less damage than faster, same ammo; heavier ammo → more damage at same speed.
- Health regens after a quiet cooldown unless hit again.
- Health at zero → dead; living actions stop; presents that accepted the claim observe dead.
- Dead bodies play the kit `die` collapse and hold last frame; gun holstered; remotes use local health for that present.
- No full respawn; no multi-killer credit policy required.
