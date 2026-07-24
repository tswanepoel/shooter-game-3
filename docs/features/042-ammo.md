# Feature 042 - Ammo (first class)

A shot is not only “whatever this blaster letter is.” **Ammo** is its own sim notion: what the round *is* (at least how heavy). The **blaster** chooses which ammo it fires and how hard it launches it. Projectiles carry ammo identity and velocity in flight.

That same noun is what **shoot and loot** share later: kinds you fire can also sit in the world as pickups (Kenney already has spare round / grenade props — e.g. foam bullets, grenade A/B). This feature only defines the kinds and the fire path. Infinite supply for now — no magazines, no scene loot, no drawing those props yet.

## Ownership

| Owns | Facts |
|------|--------|
| **Blaster** | Which ammo kind; **initial velocity** (and other gun-side fire tune already on the letter) |
| **Ammo** | **Mass** (and later other round-only facts). Shared: two blasters can fire the same kind. |

No double bookkeeping: mass does not live on the blaster; muzzle speed does not live on the ammo.

## Blaster → ammo

Every blaster letter resolves to an ammo kind. Firing spawns projectile(s) of that ammo at the blaster’s initial velocity. Multi-pellet letters: several projectiles of the same kind.

| Ammo | Mass tier | Letters / class | Later loot prop |
|------|-----------|-----------------|-----------------|
| light foam | light | pistol b,i · smg c,g,h,l,m,p · AR d,n,q,r · shotgun j,k,o (per pellet) | `bullet-foam` |
| thick foam | heavy slug | sniper e,f | `bullet-foam-thick` |
| grenade | heaviest | launcher a | `grenade-a` (`grenade-b` optional twin later) |

No per-letter ammo overrides in this feature.

## Projectile

A live projectile knows its ammo and its velocity (and enough to attribute the shot: owner, origin, etc.). Mass is looked up from ammo, not stored as a second inventing source of truth.

## Acceptance criteria

- Ammo kinds exist as first-class data, including mass.
- Every blaster letter maps to an ammo kind and still owns initial velocity.
- Spawned projectiles carry that ammo identity; launch speed comes from the blaster.
- No magazine / reserve / scene loot / prop draw in this feature (kinds stay loot-ready).
