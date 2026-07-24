# Feature 045 - Aim flinch

When a hit lands on you, aim jerks. Flinch is real sim state on the same aim offset as kick (**040**) and sway (**041**): it moves the reticle and where shots go — not a body twitch while bullets still fly true.

Look is still where you point. Flinch is a short pitch/yaw spike that stacks with kick and sway, then settles. Camera stays on look.

## Cause and strength

A damaging hit (**043**) adds flinch. Strength scales with that hit’s impact (same idea as damage: speed at contact and ammo mass), within a sane cap so one pellet does not whip the view around. No damage, no flinch.

## Acceptance criteria

- Damaging hits add flinch in sim; flinch settles over time.
- Reticle and shots use look + kick + sway + flinch.
- Stronger impact → stronger flinch, all else equal (within cap).
- No damage → no flinch from that event.
