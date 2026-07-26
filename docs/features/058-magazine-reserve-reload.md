# Feature 058 - Magazine, reserve, reload

Accepted [fire](../concepts.md#fire) spends the active [blaster](../concepts.md#blaster)’s [magazine](../concepts.md#magazine). Empty magazine blocks fire. [Reload](../concepts.md#reload) fills the magazine from [reserve ammo](../concepts.md#reserve-ammo). Infinite supply from **038** / **042** ends here.

Depends on **038** (discharge), **042** (ammo kinds), **021** / **053** (loadout + spawn apply).

## Ownership

| Owns | Facts |
|------|--------|
| **Blaster** | Magazine capacity; current magazine rounds of that blaster’s ammo |
| **Player** | Reserve rounds per [ammo](../concepts.md#ammo) kind (outside any magazine) |
| **Fire** | Accepted discharge spends from the active magazine |
| **Reload** | Moves rounds from reserve of that kind into the active magazine, up to capacity |

Mag and reserve counts may show on an existing **dev HUD**.

## Magazine

Each equipped blaster has a magazine: rounds of that blaster’s ammo, from 0 up to that blaster’s capacity.

- Accepted fire spends one round per projectile spawned (multi-pellet: one spend per pellet).
- Magazine empty → fire does not accept.
- Capacity and starting fill are per blaster letter (tables in code; not ontology). Draft fill at spawn: magazine full.

Unarmed active slot: fire stays blocked as today.

## Reserve

The player carries reserve rounds keyed by ammo kind (light foam, thick foam, grenade — **042**).

- Reload draws only from the active blaster’s ammo kind.
- Spawn drafts a starting reserve per kind the loadout can use (tables in code). Empty loadout slots add nothing.
- Fire spends only from the magazine.

## Reload

Input: session-active **R** while living, with an active blaster whose magazine is below capacity and whose reserve for that ammo kind has rounds.

- Fills the magazine from that reserve, up to capacity (or until reserve is empty).
- Fire cancels an in-progress reload if reload is timed; instantaneous fill is fine for this feature.
- Holster / swap / death cancel reload.

Sim counts move on R.

## Spawn and death

- **Spawn** (**053**): applies loadout, then magazine full and draft reserve for the kinds on that loadout.
- **Death**: living acts stop (**043**). Magazine and reserve on that life end with the figure; next spawn gets a fresh apply. Leftover rounds dump via **059**.

## Solo and multiplayer

Same sim rules. Magazine and reserve live on the local figure like health and fire today. **059** adds server grant of reserve from loot.

## Acceptance criteria

- Active magazine depletes on accepted fire; empty blocks further discharge.
- Multi-pellet discharge spends one round per pellet; a discharge only fires as many pellets as rounds remain.
- Reload (R) moves reserve of the active ammo kind into the magazine up to capacity.
- Reload runs only when the magazine is below capacity and reserve of that kind has rounds.
- Spawn fills magazine and draft reserve; death clears that life’s counts; next spawn resets.
- Dev HUD may show mag / reserve.
