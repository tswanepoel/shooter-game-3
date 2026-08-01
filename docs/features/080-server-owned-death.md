# Feature 080 - Server-owned health and death

Firer clients still **claim** hit contact. The server alone owns each living member’s **health**, applies damage from accepted claims, and **calls death** when health reaches zero. Clients treat that call as the only death event (bench, corpse, stop living acts, roster `living`).

Revises **043** (every present applied lethal claims on its own) and the kill path of **051** / **053** (membership `living` already server-side, but present death was still claim-driven). Depends on **043**, **053**, **057**, and length-prefixed reliable framing (unstaged / ship with or before this).

## Why

Under WiFi, claim datagrams and unconditional relay of **rejected** impacts let a client die (or keep shooting) while the server’s `living` flag disagrees. Spawn already follows ask → grant. Death must match that shape.

## Authority

| Event | Who decides |
| --- | --- |
| Hit landed (part, ammo, speed, target) | Firer claim (unchanged; no server raycast) |
| Health after a claim | **Server only** |
| Death (time / tick, killer, corpse) | **Server only**, when its health hits zero |
| Respawn | Unchanged: client `Spawn` → server `YouSpawned` + roster |

Clients may still show speculative hit markers / flinch from their own claims. They **must not** go to bench, dump death loot, or clear a remote living body from claim math alone.

## Wire / channel

- `ImpactHit` stays a firer claim (datagram is fine for contact).
- Server applies damage only when the firer is living, match started, target living, claim legal.
- **Do not relay** a claim the server rejected.
- On lethal accept: reliable death announce to the room (framed control stream), then corpse / roster as today. Victim and observers act on that announce — not on `PeerImpactHit` lethality.
- Non-lethal accepted hits: server may relay a damage/ack for FX, or fold health into the death path only for v1; either way, zero health without a death announce is illegal.

## Client

- Stop applying lethal (and, for membership truth, damaging) outcomes from raw `PeerImpactHit` as sole death authority.
- On server death: victim → **053** bench + death dumps; remotes → corpse / not living; stop claiming as firer once local phase is not living **or** as soon as server says you died (prefer server).
- Roster `living` remains membership paint for remotes; it must not race ahead of or lag behind the death announce.

## Out of scope

- Server raycast / lag compensation rewind.
- Changing drive, projectile VFX, or spawn grant shape.
- Fair last-hit policy beyond “server serializes health; first lethal accept wins.”

## Acceptance

- Two clients on LAN: kill → victim bench and upright corpse on firer view in the same server death; no “bench + upright living body” split.
- Rejected claims never drop the victim’s local health to zero.
- After server death, further claims from that firer are rejected and **not** presented as damage on peers.
- Spawn after death still requires server grant; no client self-revive.
- Firer-favoured hit contact (**043**) preserved: mesh collide stays on the firer client.
