# Feature 054 - Drive, look offset, and placed weapon line

After **050**, [view](../concepts.md#view) mounts [look](../concepts.md#look) on the self’s posed head. Drive and cascade still do not match the ontology: sim keeps an **ocular** free-angle bag, collapses [facing](../concepts.md#facing) into that bag, never stores [look offset](../concepts.md#look-offset), and builds [weapon line](../concepts.md#weapon-line) from parallel trig instead of after [hip](../concepts.md#hip) and [right shoulder](../concepts.md#right-shoulder) place the figure. Wish and net still speak ocular / look-as-angles.

This feature is the ship contract to align **drive and cascade** with [`docs/concepts.md`](../concepts.md). Concepts are not rewritten here. **050** stands for mount: one body, look at the eye socket, view is that look. This feature is what *drives* that body so look and weapon line are honest.

**Does not redefine:** loadout, loco modes, ammo, projectiles, health, hit claim networking, reticle chrome, muzzle flash, flycam, residual homes (**049**).

---

## Intent

Mouse (session) only feeds **drive**. Drive is [look](../concepts.md#look), [facing](../concepts.md#facing), and [locomotion](../concepts.md#locomotion) at a [tick](../concepts.md#tick). [Look offset](../concepts.md#look-offset) is look relative to facing. [Hip](../concepts.md#hip), [right shoulder](../concepts.md#right-shoulder), and [neck](../concepts.md#neck) take **proportions of look offset** (plus fire/hit residual and sway on the joints that already own them). After that placement, **look** is the pose at the [eye socket](../concepts.md#eye-socket); **view** mounts it (**050**). **Weapon line** is the blaster direction after drive, hip, and right shoulder place the figure — not a free aim vector beside the body. [Wish](../concepts.md#wish) is along look’s horizontal forward and right.

No public **ocular** command bag as combat or camera truth. No second analytical “look direction” used as view. Code names match concept terms where a term exists (prefer rename over dual vocabulary).

---

## Out of scope

- Independent facing *lag controllers*, torso twist lag curves, or two-hand IK (structure must allow facing ≠ look azimuth later; this feature may still ship with facing tracking look’s ground azimuth if look-offset **yaw is zero**, as long as facing and look offset are first-class and not one ocular field).
- Rewriting [`docs/concepts.md`](../concepts.md) (mend only if a term is wrong; open that as a separate concepts change).
- Dual look-pose vs present-pose (**017** history; **050** already ended the local mount split).
- New HUD chrome, net protocol redesign beyond renaming drive fields that already mean look/facing.
- Changing residual homes or fall rules (**049**).

---

## Current debt (why this feature)

| Concepts | Code today (pre-054) |
|----------|----------------------|
| Drive = look + facing + loco | `ocular_yaw` / `ocular_pitch` + loco; facing forced `torso_yaw = ocular_yaw` |
| Look offset | Not stored; elevation split from raw ocular pitch |
| Hip / neck fold | Composed as `torso_pitch` / `head_pitch` |
| Weapon line after place | `weapon_line_dir()` = ocular + residual sums (parallel to joints) |
| Wish along look horizontal | `look_forward_xz()` from ocular yaw only |
| Look as eye-socket pose | Client sample only; sim never owns look as pose |
| Names | ocular*, OCULAR_*, comments that call free angles “look” |

Mount path (**050**) is not the main debt. Drive and weapon line are.

---

## Must

1. **Drive shape** — Sim drive is look orientation intent + facing + locomotion. Public fields and net samples do not expose an ocular bag as the name of truth. Prefer concept names (`facing`, look-offset components or equivalent that *are* look offset, loco). Absolute world yaw/pitch may exist only as derived helpers, not as the ontology.

2. **Facing** — First-class ground orientation of the legs (placement root yaw). Not a silent alias of a free “look yaw” field with no facing term.

3. **Look offset** — First-class orientation of look relative to facing. Hip, right shoulder, and neck **look share** is a proportion of this offset (same proportion idea as today for elevation; residual still adds on those joints per **049**). Look-offset **yaw may ship as zero** (facing tracks look ground azimuth) if documented; elevation offset must still drive the cascade.

4. **Compose then place** — One compose path writes joint folds/twists used by present. No second formula that re-sums ocular + residuals for a parallel “aim”.

5. **Weapon line** — Direction of the active blaster **after** drive + hip + right shoulder placement (composed joint state that places the hold chain). Unarmed → no weapon line. Neck residual does not invent a separate weapon-line law (weapon line ignores neck, as today and as concepts allow by listing only hip + right shoulder).

6. **Look** — After present places the figure, look is only the eye-socket pose on that head (position + head orientation). View mounts that look (**050**). Sim must not offer an analytical `look_forward` (or equivalent) as view or as a substitute for that pose.

7. **Wish** — Ground move intent along **look**’s horizontal forward and right (from look after placement, or from the same ground azimuth look uses when look-offset yaw is zero — must not be a third private yaw).

8. **Reticle and combat rays** — Origin at look’s position (eye socket). Direction along weapon line (combat after spread). No ocular fallback direction for armed fire.

9. **Mouse / session** — Updates drive (facing and/or look offset) only. Never sets camera pose directly. Flycam remains independent of view.

10. **Names** — Public sim and net identifiers prefer concept terms: facing, look offset (or explicit yaw/pitch *of look offset*), hip fold, neck fold, right-shoulder fold/twist. Drop or privatize `ocular_*` at the API boundary. Mesh may still map hip fold onto the kit torso node and neck fold onto the head node; comments say hip/neck, not “torso is look”.

11. **Remotes / wire** — Drive samples carry enough to rebuild the same facing + look offset + loco (and residual already on the figure). Field renames on the wire are allowed in this alpha; document the mapping once.

12. **Tests** — Cover: look offset → joint cascade; facing on placement; weapon line moves with hip/shoulder residual not with a free bag; wish turns with look ground azimuth; view still only from set_mounted_look / head sample; no production path from free angles → view matrix.

---

## Feel (architecture, not tuning)

| Intent | Form |
|--------|------|
| Mouse turns what you see | Drive → joints → head → look → view |
| Body and gun agree | Same compose; weapon line after hip + shoulder place |
| Walk where you look | Wish from look horizontal |
| Kick moves shots and reticle | Residual on joints moves weapon line |
| Camera rides body | **050** mount unchanged |

Numbers (caps, proportion tables, sens) stay in code/tables, not in this file.

---

## Implementation notes (non-normative)

Suggested order so two aim truths never ship:

1. **Introduce facing + look offset** on `SelfState` (even if look-offset yaw = 0 and facing copies previous ocular yaw). Route `apply_look` / net `set_look` into those fields; recompose joints from look offset only.
2. **Rename composed channels** at the sim API: hip fold, neck fold, shoulder fold/twist (mesh binding keeps kit node names).
3. **Weapon line from composed placement** (facing + shoulder twist, hip+shoulder fold from composed fields — not a re-read of a deleted ocular bag).
4. **Wish** from look ground azimuth (same as facing while offset yaw is 0; switch to look sample later only if needed).
5. **Strip** `ocular_*` public names, analytical look-as-view helpers, and fire `ocular_forward` fallbacks for armed aim.
6. **Net / tests / lookhud** rename; HUD may show facing, look-offset, cam yaw from head sample.
7. **Present / 050** path unchanged in contract: `look_from_head` → `set_mounted_look`.

Ship facing-tracks-look (zero azimuth look offset) is an acceptable **first land** if (1)–(6) hold and a follow-up can open azimuth offset without another ontology flip.

---

## Done when

- Drive is look + facing + loco in structure and names; ocular is gone from the public story.
- Joints fold from look offset (plus residual homes); one compose feeds present and weapon line.
- Weapon line is after hip + right shoulder place the figure; reticle/combat use look origin + weapon line.
- Wish follows look horizontal; view still only mounts head-sampled look.
- Tests and debug HUD speak the same chain; slow L/R and pitch bugs are debuggable as drive → offset → joints → head → look → view with no parallel free-angle camera path.

**Reference:** [`docs/concepts.md`](../concepts.md) — look, eye socket, facing, look offset, hip, right shoulder, neck, drive, wish, weapon line, view. **050** for mount. **049** for residual homes.
