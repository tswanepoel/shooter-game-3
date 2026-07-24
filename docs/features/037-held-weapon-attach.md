# Feature 037 - Held weapon attach (hand socket)

Client presentation needs one clear **arm → hand → blaster** story. Held weapons, lineup checks, muzzle cues, and later fire FX all parent through the same chain. This feature is a design and code cleanup: it names the joins, splits frames of reference, and collapses attach into a single composition. Armed hold presentation (level gun, muzzle character-forward, grip tables) stays the visual baseline; the refactor preserves that look while making the contract readable and reusable.

Depends on the character and blaster kits, and on the attach consumers already in tree: **011**, **012**, **013**, **014** / **015**, **020**, **038**.

## Important facts

### Character kit

- Rigid multi-part body. Limb motion is node TRS on mesh parts; there is no wrist or hand bone.
- Right limb for hold and attach is **`arm-right`**, parented under **`torso`**. Pivot is the **shoulder**.
- Bind local rotations are identity. Arm mesh extends from the shoulder along local **−Y** toward the hand end.
- Armed hold uses the kit clip **`holding-right`**: **`arm-right`** rotation ≈ **−90° about local X**, so the limb’s hand direction aligns with character **+Z**. Aim layers additional shoulder pitch on that arm (014 / 015).
- Character kit units → metres: uniform **×1/1.5** on the placement chain. Feet on **y = 0** after sole snap. Face **+Z**, Y-up.

### Blaster kit

- Authored mesh frame: muzzle along mesh **−Z**, stock **+Z**, top **+Y**.
- Blaster kit units → metres: **×1**. When positions already ride the character scale chain, the blaster mesh uses relative scale **×1.5** so authored size stays one metre per unit.
- Held facing in character space: muzzle along character **+Z**, top **+Y**. Mapping from mesh frame is **180° about Y** once the hand frame is established under hold.

### Present attach (what the product already does)

Held presentation already parents the blaster under the posed **`arm-right`** matrix (full TRS, including sprint loco when hold is dropped). Grip **position** is a per-letter offset in **`arm-right` local after `holding-right`**. Orientation on the arm is **inv(`holding-right`) · Ry(180°)**: cancel the hold rotation so the gun sits level under hold, then yaw the mesh so muzzle faces character forward. That recipe is the visual source of truth for this cleanup.

### Why this feature

The composition works, but the story is split: grip and muzzle tables live in the hold arm-attachment frame; orientation is a hold-cancel baked into the attach; docs still describe a simpler “translate grip then yaw” product; consumers reassemble pieces differently. Future work (FX, remotes, new weapons) needs **named frames** and **one matrix product**.

## What it is

### Two frames

| Frame | Owner | Space | Role |
|-------|--------|--------|------|
| **Hand socket** `H` | character / arm | local to **`arm-right`** | Where the fist is and how the palm faces. Shared by all weapons. |
| **Weapon grip** `G` | blaster letter | local to blaster mesh | Where the handle meets the hand and how the mesh sits in that hand. |

`H` is a **logic node**: code builds it; the GLB hierarchy stays as authored. Debug tools may draw the socket; the character still draws only kit parts.

### Composition (authoritative)

In kit space, then into world:

```
hand_kit     = arm_right_kit · H
held_blaster = kit_to_world · hand_kit · inv(G) · S_blaster
```

- **`arm_right_kit`:** world matrix of `arm-right` after the current pose (bind / hold / aim / loco), in character kit space, from the same pose pass that draws the body.
- **`H`:** hand socket in arm-local. For armed hold presentation, `H` equals the current effective attach so hold look is unchanged: grip translation (letter or shared hand point — see data model) plus hold-cancel orientation and the held yaw that faces the muzzle character-forward under hold.
- **`G`:** weapon grip in blaster-local. Identity grip means the blaster origin sits on the socket with mesh axes already matching the socket after `inv(G)`.
- **`S_blaster`:** uniform relative scale **BLASTER_UNITS_TO_M / CHAR_UNITS_TO_M** (**1.5**).
- **`kit_to_world`:** placement · character scale · feet snap (existing).

All of: self blaster draw, debug lineup held pair, muzzle world points, and present muzzle FX sample **`held_blaster`** (or a point under it). One function owns the product; call sites pass pose arm matrix, letter, and placement scale chain.

### Data model

**Hand socket (character side)**

- One primary socket **`H_hold`** for armed hold and aim: encodes the in-hand frame that today’s attach produces under `holding-right`.
- Socket translation places the fist at the hand end of the arm mesh (arm-local). Socket rotation establishes palm / grip axes so that under hold the weapon line is level and character-forward after grip resolve.
- When loco owns the right arm (sprint while armed), the same **`H_hold`** rides the swinging arm matrix. A second socket **`H_loco`** (or a short blend between sockets on mode change) is a later presentation option if in-hand angle under loco needs its own authoring; it is out of scope for this cleanup’s required surface.

**Weapon grip and muzzles (blaster side)**

- Per-letter **grip** `G` in **blaster-local** units (handle relative to mesh root).
- Per-letter **muzzle points** in **blaster-local** units (barrel exits in mesh space).
- World muzzle:

  ```
  muzzle_world = held_blaster · muzzle_local
  ```

- Migration from today’s tables: existing grip offsets and muzzle points are defined in the **arm-attachment frame** (`arm-right` local after `holding-right`). Convert once through the known hold attach into blaster-local (and into `H` / `G` split), freeze the new tables, and keep kit READMEs as the human-readable source. Letter pairing `a`…`r` unchanged.

### Pose ownership

- Body pose rebuild remains parent × local on kit nodes. **`arm-right`** is still the last real character node on this chain.
- Hold + aim continue to own **`arm-right`** while armed and not sprinting; sprint loco owns the arm when hold is dropped (**020**). Attach always multiplies the **current** arm matrix; it does not invent a second aim path.
- Weapon line (014 / 015) samples attitude from the mounted blaster after this attach. Aim stays on the torso / shoulder chain; the socket only places the gun on the arm.

### Code and docs surface

- Single client entry for held root and for transforming blaster-local points to world (draw, lineup, markers, fire present).
- Blaster kit README documents: hand socket under `arm-right`, grip `G`, muzzle in blaster-local, composition above, dual scale.
- Character kit README documents: shoulder pivot, `holding-right`, and that the hand is a presentation socket parented to `arm-right`.
- Feature **011** / **012** / **013** attach language is superseded by this contract for new work; those docs stay historical. Implementation and kit READMEs match **037**.

## Acceptance criteria

- **Named frames:** code and kit docs speak of **hand socket** `H` on `arm-right` and **weapon grip** `G` on the blaster; held root is exactly `kit_to_world · arm · H · inv(G) · S_blaster` (or an equivalent factorization with the same math).
- **One composition path:** self present, debug held pair, muzzle markers, and present muzzle placement all use that path. Muzzle world points are blaster-local points under the held root.
- **Hold look preserved:** under `holding-right` (lineup and armed stand / aim), grip placement, level weapon, muzzle forward, and dual kit scale match the current product baseline for letters `a`…`r`.
- **Arm follow:** when the arm matrix changes (aim pitch, torso, sprint loco), the held blaster uses the same arm matrix as the drawn `arm-right` part in that pose pass.
- **Tables and READMEs:** grip and muzzle data are documented in the spaces above; blaster README composition matches the code. Character README notes the logic hand socket.
- **Scope:** presentation attach and documentation only. Sim aim, fire cadence, ballistics, and net contracts stay as in **015** / **038** and related features. **038** combat spawn is look origin → crosshair; this chain supplies **muzzle world points for present FX** (flash, markers), not combat projectile origin.

## Kit documentation

- Characters: `assets/source/characters/README.md`
- Blasters, grip, muzzles, held composition: `assets/source/blasters/README.md`
