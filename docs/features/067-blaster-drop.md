# Feature 067 - Blaster drop and F pickup

On [death](../concepts.md#death), the dead [figure](../concepts.md#figure)’s **active** [blaster](../concepts.md#blaster) leaves the body as a **blaster drop**: letter plus [magazine](../concepts.md#magazine) (including empty), pinned near the [corpse](../concepts.md#corpse). Unarmed active → no drop. Active slot on the dead life is cleared. The drop is a visible Kenney mesh; it ends when taken or when its short timer ends.

**Ammo drop ≠ blaster drop.** [Ammo drop](../concepts.md#ammo-drop) (**059**) dumps [reserve](../concepts.md#reserve-ammo) of the active kind only; magazine rounds ride the blaster.

Living players take a blaster drop with **F** while overlapping its radius **and looking at it** — not walk-over. Client **claims**; server **elects** one winner and **grants** (same compete as **059**). Grant fills a free slot (prefer [primary](../concepts.md#primary-slot), then [secondary](../concepts.md#secondary-slot)) and makes that slot [active](../concepts.md#active-slot). If none free, **swap** the active slot; the displaced blaster+mag becomes a new short-lived floor drop **in front of the figure**. Floor pickup may place **any** [weapon class](../concepts.md#weapon-class) in secondary — **021** class laws stay on [loadout](../concepts.md#loadout) / spawn choose only. Living slots are not loadout: pickup does not stage into the next life; the bench keeps the picker choice.

Death pins the blaster to the **right** of the corpse (right-handed hold) and slightly **behind** (die falls backwards), laid on its side — not inside the torso and not upright.

Depends on **059**, **021** / **053**, **058**, **057**.

## Acceptance criteria

- Death with an armed active slot spawns one blaster drop (letter + magazine) beside the corpse (right/back, on its side); that slot on the dead life is cleared.
- Ammo drop from death carries reserve of the active kind only; magazine stays with the blaster drop.
- Unarmed active → no blaster drop.
- Living F + overlap **and look-at** claims; walk-over / sky-gaze does not take it; server elects one winner.
- Grant equips a free slot (primary then secondary, becomes active) or swaps the active slot; pickup may put any class in secondary; displaced blaster+mag is a new short-lived floor drop in front of the figure.
- Living pickup does not change staged loadout; next spawn reapplies the picker choice only.
- Floor blaster ends on grant or its short timer.
- Client presents the floor letter from sim; does not invent a parallel weapon.
