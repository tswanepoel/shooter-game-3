# Feature 048 - Body aim cascade

Ship contract for aim residual after 040–047. Ontology stays in [`docs/concepts.md`](../concepts.md); this file is **requirements and acceptance**, not a second dictionary.

**Supersedes the aim path of 040–047** as shipped history: free aim offset bag (`KickPose`), dual mesh/aim stacks, combat as look + bag. That path is wrong after 048.

**Does not redefine:** loadout, loco, ammo, projectiles, health, hit claim networking, reticle chrome, muzzle flash.

---

## Intent

When the figure fires (or takes a damaging hit), the **body** takes the kick — **blaster → arm → torso → head** — in **one** response (same apply, not a delayed spill up the spine). **Partial** amounts along that chain use the **same look-elevation proportions** the body already uses so ratios stay coherent. The **camera rides the head**; it is not a separate kick channel. **Bullets and reticle** follow the aimed blaster after that placement (**weapon line**), not a free pitch/yaw bag.

Grip shove along the bore stays a short residual on the held blaster (presentation + sim bore travel), not a second full aim stack.

Climb under sustained fire and snappy recover after short strings (**047** feel) should **emerge** from residual fall (including slower fall while fire is held), not from a parallel fatigue aim layer.

---

## Out of scope

- Next-tick / serial “impulse walks up the spine over frames”
- Independent facing controller, two-hand IK, left-shoulder aim
- Netcode redesign, new HUD chrome, blood
- Replacing or rewriting `concepts.md` in this feature (open a concepts mend separately if terms lag)

---

## Must

1. **No free aim bag** as combat truth — no public `KickPose` / `aim_pose` / `mesh_pose` path that replaces joints for shots and reticle.
2. **Fire / hit residual on the figure** — discharge and applied damage hand off residual; fire control keeps cadence/gates only.
3. **Chain coverage** — residual affects arm, torso, and head placement (not shoulder-only while head/camera stay frozen).
4. **Proportions** — look-elevation share rules stay; body kick does not invent a second fold law that breaks those ratios.
5. **Unison** — one discharge’s kick is applied together; no spillover cascade over later ticks.
6. **Weapon line** — direction of the active blaster after the figure is placed; unarmed = no combat weapon line.
7. **Reticle and projectiles** — origin at the view/look eye; direction from weapon line (combat after spread). Muzzle remains FX for combat origin (038).
8. **View** — first-person camera follows the head/eyes after that placement (passenger on the body), not a parallel camera-only kick bag.
9. **Grip bore** — short back travel from fire, falls with fire residual; held mesh reads it.
10. **One present truth** — body, gun, weapon line, and view mount from the same placement; no dual combat/mesh aim stacks.
11. **Clear on unarmed / dead** (and emote holster policy unchanged for arms).
12. **Tests** — assert residual on the figure, weapon line, and view/reticle behaviour; not look+KickPose.

---

## Feel (047, architecture not required)

| Intent | Form |
|--------|------|
| Fire moves shots | Residual placement moves **weapon line** |
| Reticle and bullets agree | Both use **weapon line** (+ spread for combat) |
| Camera with the body | View follows head after kick placement |
| Climb on long strings | Residual fall slows while fire held; recovers after |
| Grip shove | Bore travel on the held gun |

Tuning numbers live in tables/code, not here.

---

## Implementation notes (non-normative)

Suggested order so two aim truths never ship:

1. Residual + fall on the figure; gates stay on fire control  
2. Place hip / shoulder / neck with look proportions + residual (same proportion idea)  
3. Grip bore; weapon line; reticle/combat/spawn  
4. Present + view from that placement  
5. Strip KickPose public path; fix tests/HUD  

Remotes may approximate present; local self must match this intent.

---

## Done when

A player can fire and take hits with residual on the body chain (arm through head), camera moving with the head, bullets and reticle on weapon line, free aim bag gone, and short-fire / sustained-fire recover still feel in the 047 ballpark.

**Reference:** [`docs/concepts.md`](../concepts.md) — prefer those names in code when they fit; mend concepts in a separate change if they still describe shoulder-only residual or a frozen camera under kick.
