# Feature 021 - Loadout secondary + active weapon

Self loadout has primary and secondary slots; mouse wheel toggles which slot is active (instant swap, no draw anim). Unarmed only by leaving a slot empty.

## Acceptance Criteria

- Self owns optional **primary** and **secondary** blaster letters plus **active** selection (**primary** or **secondary** only). Neither slot is mandatory.
- **Primary** may be any class. **Secondary** may be only **launcher** or **pistol** (reject other classes on assign). Class map:

| Class | Letters |
| --- | --- |
| launcher | a |
| pistol | b, i |
| smg | c, g, h, l, m, p |
| assaultRifle | d, n, q, r |
| sniperRifle | e, f |
| shotgun | j, k, o |

- Default spawn: primary `p`, secondary `b`, active primary (same armed feel as today). Equip UI out of scope.
- Session-active **mouse wheel** (mounted) cycles **primary ↔ secondary** only (both slots always in the cycle, empty or not). No third always-unarmed step.
- **Unarmed** only when the **active slot is empty** — free arms require foregoing primary or secondary (typically leave secondary empty and select it). Cannot be unarmed while both slots are filled.
- Swap is **instant** (no time cost, no holster/draw anim).
- Active slot filled: present that blaster and hold/aim as today (015 / 016 / 017), subject to sprint arms (020).
- Active slot empty: no blaster mesh, no reticle, **no hold pose** — arms free (rest / locomotion only; later speed advantage).
- Client presents active identity from sim; does not invent a second loadout.
