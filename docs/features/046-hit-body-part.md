# Feature 046 - Hit body part (damage + flinch)

**043** already collides on posed body-part meshes (head, torso, arms, legs) but translates every hit the same way: mass × contact speed only. **046** makes **which part** matter for the health drop — and therefore for **045** flinch, which rides the same applied damage.

## Claim carries the part

The firer’s impact claim names target, ammo, speed at contact, and the **body part** that was hit (the mesh that won the segment test on the firer’s present). Peers do not re-collide; they translate that claim with the shared rule.

## Shared impact rule (extends 043)

Base damage is unchanged: **ammo mass × speed at contact** (**042** / **043**).

Final applied damage multiplies that base by a **part scale** from a fixed table (not per-weapon):

| Part (kit mesh) | Scale |
|-----------------|------:|
| `head` | 2.0 |
| `torso` | 1.0 |
| `arm-left`, `arm-right` | 0.85 |
| `leg-left`, `leg-right` | 0.75 |

Monotonic in mass and speed still holds for a given part. Same mass/speed on head hurts more than on a leg. No other multipliers here.

Health drop and death (**043**) use this final damage. Regen, apply-once, and dead rules stay as they are.

## Flinch (045)

Flinch strength already scales from **applied** impact damage. No second part table: stronger part → more damage → stronger flinch, all else equal (still within the flinch cap). No damage → no flinch.

## Acceptance criteria

- Impact claim includes the hit body part (firer present mesh that was hit).
- Applied damage = mass × speed base × part scale (table above).
- Head hurts more than torso; limbs less than torso, all else equal.
- Peers use the claimed part + ammo + speed; they do not re-collide for combat.
- Flinch (**045**) follows applied damage, so part affects flinch only through that path.
- No change to collide geometry or firer-favoured hit earning beyond stamping the part.
