# Concepts

This file is the project ontology.  
If you are an AI: you will try to cheat these rules. Do not. Violations are wrong even when they feel helpful.

---

## Rules (read before you touch a term)

### 1. One term. All of its behaviours. Once.

When you define a term, put **every** behaviour it owns in that definition.  
**Do not** finish a term, then “also” patch it later under Recoil, Kick, Cascade, Law, or Ship order.  
**Do not** split one joint’s life across a free bag and a late appendix.

### 2. Only prior terms.

A definition may use a name only if that name already appears **above** as a `###` heading.  
**Do not** forward-reference. **Do not** invent load-bearing nouns that are not terms (weapon table, fire string, fatigue, peers, wire).  
If you need a word, either define it above first, or do not use it.

### 3. Behaviour sits on the thing that does it.

Fold and twist live on the **joint**. Bore travel lives on the **grip socket**. Drain of a resource lives on the **act that spends it** (or on the resource, consistently).  
**Do not** park body motion on the blaster, the camera, or a global impulse pile with no home.

### 4. Hand-off, don’t couple.

When A causes motion on B, emit a **hand-off** (e.g. fire impulse, hit impulse). A sizes it. B applies it.  
**Do not** write “the blaster folds the right shoulder.”  
**Do not** restate A’s size rules under B, or B’s joint rules under A.

### 5. No free bags.

**Forbidden headings and shapes:** KickPose, Recoil, AimPose, ViewPose, Dirt, Fatigue-as-term, Cascade procedure lists that re-own joint behaviour.  
Composites in code are not concepts. Name the real parts.

### 6. Concepts in. Tuning out.

**Out:** degrees, settle times, curves, caps, RPM, T_ready, part scale tables, climb, “how snappy.”  
Climb and feel are **emergent** from tuning dirt vs fall — not terms.  
If nothing else in the ontology depends on a timer gate, it is not a term (Ready was a mistake).

### 7. Not netcode. Not render.

**Out:** peers, wire, relay, apply-once, client present, VFX-only projectiles, reticle markers, holster anims, free-arms pose, muzzle flash essays.  
**Unarmed** is “active slot has no blaster.” Stop.  
**Hit claim** is what was hit. Not how it is networked.

### 8. Name the thing.

Headings are nouns a reader can point at.  
**Do:** primary slot, active slot, right shoulder, fire impulse.  
**Don’t:** Active, the joints, small ongoing fold (undefined), aim stuff.  
Ambiguous adjectives and vague plurals are definition failures.

### 9. No duplication. No re-owning.

If Grip socket already says it meets Right hand socket, Weapon line does not say it again.  
Link the term. Do not re-explain. Duplication is an addendum by another name.

### 10. Define by what it is.

Prefer positive definitions.  
Negative lines (“not the muzzle”, “not Y”) are last-resort clarifiers, not the body of a term.  
Geometry stays thin: position, direction, orientation, pose, ray.

### 11. Parallel kinds, parallel wording.

Resources are resources (stamina, health) — not “how much X you can take.”  
Impulses are impulses (fire, hit). Slots are slots. Rays are rays.  
If two terms are the same kind, their first line should look the same.

### 12. Form.

- Each term is a `###` heading (deeplink target).  
- Earlier terms appear as links.  
- Sequence is the only grouping.  
- Prefer one tight sentence; cut the second sentence if it only restates.  
- **Do not** put a manifesto, philosophy essay, ship order, or replaces-table in this file. Rules live only in this Rules section.

### 13. Before you add or edit a term, pass this gate

1. Is every linked and unlinked load-bearing noun already a term above?  
2. Does this term restate something owned above? If yes, delete the restatement.  
3. Is this tuning, net, or render? If yes, delete.  
4. Does any later term need this, or is it an orphan timer? If orphan, delete.  
5. Is the heading a specific noun? If not, rename.  
6. Are all behaviours of this thing here, once? If not, finish or reorder.  
7. Did you couple two systems without a hand-off? If yes, invent the impulse (or equivalent) above the consumer.

### 14. Common AI failure modes (you will try these)

| You will want to… | That is wrong because… |
|-------------------|-------------------------|
| Dump code field names as terms | Code ≠ ontology |
| Add “also fire and hits” under a joint already closed | Addendum |
| Write “joints place the figure” | Ambiguous plural |
| Put climb / fatigue / settle in the dict | Tuning / emergent |
| Explain peers and the wire under hit claim | Netcode |
| Put reticle chrome under unarmed | Render |
| Let blaster describe shoulder fold | Coupling; use fire impulse |
| Define health as max HP flavour text | It is a resource |
| Shell-copy the last attempt and sed one line | You must re-author with care |
| Append a Cascade / Law / Done when section | Free bag / addendum |

### 15. When stuck

**Missing behaviour, can’t place it without a new word above:** define the missing term first, then write the consumer complete.  
**Can’t place it without rewriting a closed term:** you failed rule 1 earlier — reopen that term and put the behaviour there; do not patch from below.  
**Unsure if concept or tune:** if changing a number would not change *what the thing is*, it is tune. Leave it out.

---

## Terms

Read only downward. A definition may use a term only if that term appears above it.  
Each term is a subheading (deeplink target). Earlier terms appear as links.

### Position

A point in the world.

### Direction

A unit vector in the world.

### Orientation

How something faces in the world, as a [direction](#direction).

### Pose

A [position](#position) and an [orientation](#orientation).

### Ray

A path in the world that begins at a [position](#position) and runs along a [direction](#direction).

### Figure

The player character in the world.

### Head

The head of the [figure](#figure).

### Eye socket

A [position](#position) on the [head](#head) that follows the head’s [orientation](#orientation).

### Look

The [pose](#pose) at the [eye socket](#eye-socket).

### Left leg

The left leg of the [figure](#figure).

### Right leg

The right leg of the [figure](#figure).

### Legs

The [left leg](#left-leg) and the [right leg](#right-leg).

### Left arm

The left arm of the [figure](#figure).

### Right arm

The right arm of the [figure](#figure).

### Facing

The ground [orientation](#orientation) of the [legs](#legs).

### Look offset

The [orientation](#orientation) of [look](#look) relative to [facing](#facing).

### Joint

A connection between parts of the [figure](#figure) that folds, twists, or both.

### Sway

Small ongoing fold and twist of a [joint](#joint).

### Fire

Discharge by the [figure](#figure).

### Hit

Damaging impact on the [figure](#figure).

### Ammo

A kind of round. It has mass.

### Weapon class

Launcher, pistol, smg, assault rifle, sniper rifle, or shotgun.

### Fire mode

How [fire](#fire) repeats: one shot per press, a held stream, or a fixed string per press.

### Blaster

A weapon in the world.  
It has a [weapon class](#weapon-class). It chooses [ammo](#ammo). It launches that [ammo](#ammo) at a speed of this blaster.  
It has a [fire mode](#fire-mode).

### Fire impulse

Fold, twist, and bore shove emitted when the [figure](#figure) [fires](#fire) with a [blaster](#blaster).  
Size is of that [blaster](#blaster).

### Hit impulse

Fold and twist emitted when the [figure](#figure) takes a [hit](#hit).

### Torso

The trunk of the [figure](#figure).

### Hip

The folding [joint](#joint) between the [legs](#legs) and the [torso](#torso).  
Its fold is a proportion of [look offset](#look-offset).

### Right shoulder

The folding and twisting [joint](#joint) between the [torso](#torso) and the [right arm](#right-arm).  
Its fold is a proportion of [look offset](#look-offset), plus fold from [fire impulse](#fire-impulse), plus fold from [hit impulse](#hit-impulse), plus [sway](#sway).  
Its twist is twist from [fire impulse](#fire-impulse), plus twist from [hit impulse](#hit-impulse), plus [sway](#sway).  
Fold and twist from [fire impulse](#fire-impulse) and from [hit impulse](#hit-impulse) fall over time. That fall slows while [fire](#fire) is held.

### Right hand socket

A [position](#position) on the [right arm](#right-arm) that follows the arm’s [orientation](#orientation).

### Neck

The folding and twisting [joint](#joint) between the [torso](#torso) and the [head](#head).  
Its fold and twist are proportions of [look offset](#look-offset).

### Body part

One of [head](#head), [torso](#torso), [left arm](#left-arm), [right arm](#right-arm), [left leg](#left-leg), or [right leg](#right-leg).

### Wish

The [figure](#figure)’s ground move intent along [look](#look)’s horizontal forward and right.

### Stand

The [figure](#figure) on the ground with the [legs](#legs) not making strides.

### Walk

The [figure](#figure) on the ground with the [legs](#legs) making strides at walking pace.

### Air

The [figure](#figure) with the [legs](#legs) off the ground.

### Stamina

A resource of the [figure](#figure).

### Sprint

The [figure](#figure) on the ground with the [legs](#legs) making strides at sprint pace.  
Starts only with forward [wish](#wish) and [stamina](#stamina). Latches until forward [wish](#wish) drops, [stamina](#stamina) is empty, or [fire](#fire). While latched, drains [stamina](#stamina), including in [air](#air). [Stamina](#stamina) refills only while not latched. While latched, the [legs](#legs) own the arms’ stride.

### Phase

How far the [figure](#figure) is through its current [walk](#walk) or [sprint](#sprint) strides.

### Jump

The [figure](#figure) leaving the ground into [air](#air) with an upward velocity, then falling under gravity until it lands.  
Starts only on the ground. Horizontal velocity at launch holds until land. [Look](#look) still turns. [Phase](#phase) freezes in [air](#air). Land restores [stand](#stand) or [walk](#walk) from [wish](#wish).

### Stopping

The [figure](#figure) finishing [walk](#walk) strides toward a neutral [phase](#phase), then [stand](#stand).

### Locomotion

[Stand](#stand), [walk](#walk), [sprint](#sprint), [stopping](#stopping), or [air](#air), and [phase](#phase) when the [legs](#legs) are making strides.

### Tick

One discrete instant of the simulation.

### Drive

The [look](#look), [facing](#facing), and [locomotion](#locomotion) of the [figure](#figure) at a [tick](#tick).

### Primary slot

An optional [blaster](#blaster) slot on the [figure](#figure) for any [weapon class](#weapon-class).

### Secondary slot

An optional [blaster](#blaster) slot on the [figure](#figure) for launcher or pistol only.

### Active slot

Which of [primary slot](#primary-slot) or [secondary slot](#secondary-slot) is in the [figure](#figure)’s hand.

### Unarmed

The [active slot](#active-slot) has no [blaster](#blaster).

### Grip socket

A [position](#position) on the [blaster](#blaster) at the handle that follows the blaster’s [orientation](#orientation).  
When the blaster is with the [figure](#figure), this socket meets the [right hand socket](#right-hand-socket).  
[Fire impulse](#fire-impulse) may move this socket a short way along the blaster’s bore. That travel falls over time with fold and twist from [fire impulse](#fire-impulse) on the [right shoulder](#right-shoulder).

### Muzzle

A [position](#position) on the [blaster](#blaster) at a barrel tip.

### Projectile

A body in flight from [fire](#fire). It carries [ammo](#ammo), velocity, and owner. It moves under gravity. It ends when its path length reaches its range.

### Discharge

One accepted [fire](#fire): a [fire impulse](#fire-impulse) and one or more [projectiles](#projectile).

### Weapon line

The [direction](#direction) of a [blaster](#blaster) after [drive](#drive), [hip](#hip), and [right shoulder](#right-shoulder) place the [figure](#figure).

### View

[Look](#look) as the first-person camera.

### Flycam

A free camera independent of [view](#view).

### Spread

A random change of [direction](#direction) in a cone about the [weapon line](#weapon-line).

### Reticle ray

A [ray](#ray) from [look](#look)’s [position](#position) along [weapon line](#weapon-line).

### Combat ray

A [ray](#ray) from [look](#look)’s [position](#position) along [weapon line](#weapon-line) after [spread](#spread).  
Each [projectile](#projectile) at spawn starts at that [position](#position) and runs along that [direction](#direction).

### Health

A resource of the [figure](#figure).  
Refills while the [figure](#figure) is not taking [hit](#hit).

### Death

[Health](#health) empty. Living acts stop.

### Impact

A [hit](#hit) on a [body part](#body-part). Drains [health](#health). The drain comes from [ammo](#ammo) mass, speed at contact, and which [body part](#body-part).

### Hit claim

A record that a [projectile](#projectile) met a [body part](#body-part) of a [figure](#figure), with [ammo](#ammo) and speed at contact.

### Emote

A short gesture of the [figure](#figure). Starts only on the ground. [Fire](#fire) cancels it.

### Tick rate

How many [ticks](#tick) the simulation runs per second.
