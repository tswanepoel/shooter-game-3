# Feature 045 - Aim flinch

When a hit lands on you, aim jerks. Flinch is real sim state on the same aim offset as kick (**040**) and sway (**041**): it moves the reticle and where shots go — not a body twitch while bullets still fly true.

Look is still where you point. Flinch is a short pitch/yaw spike that stacks with kick and sway, then settles. Camera stays on look.

## Cause and strength

Flinch is driven by the **same impact claim** that drops health (**043**): firer-claimed hit, applied once in this present when you are the target.

**Strength uses the same inputs as the health drop** — not a parallel per-weapon table:

- **Ammo mass** (**042**)
- **Speed at contact** (stamped on the claim)

Same shared impact rule as **043** (`impact_damage`: mass × speed). Flinch gain is a function of that damage (or the identical mass×speed product). If the claim applies **no damage** (already dead, zero impact), it adds **no flinch**.

Scale within a sane cap so one pellet does not whip the view around. Stronger applied impact → stronger flinch, all else equal (within cap). Gravity / long flight that softens damage softens flinch the same way.

## Acceptance criteria

- Damaging hits on you add flinch in sim; flinch settles over time.
- Reticle and shots use look + kick + sway + flinch.
- Flinch strength uses the same mass × contact-speed inputs as **043** health drop (shared impact rule).
- Stronger applied impact → stronger flinch, all else equal (within cap).
- No damage from that claim → no flinch from that event.
