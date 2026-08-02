# Feature 094 - Map a tractor step (+ tank nose collide)

**091** / **092** stage ground lumber south of the first empty flatbed as the hop onto the consist. Flatbed deck (~1.12 m) to lumber-car load roof (~2.24 m) is a max-height jump under the **1.1 m** peak. This feature parks a static Kenney tractor beside that cargo as a mid step so the filled cart is a reliable follow-on, and reshapes the tank’s **our** collide so the lumber → tank follow-on matches the round silhouette.

Client present + map-def pose for the tractor; collide/support via kit volumes → `MapWorld` (**092**). No motion or driving. Tank mesh unchanged.

Depends on **092** (cargo-jump escape + train collide), **091** (ground cargo pose), **010** (cook / packs), Kenney Car Kit `tractor` (CC0).

## Intent

| Aim | Meaning |
| --- | --- |
| Role | Yard tractor beside the unload pile; parkour mid-step |
| Place | South of home rail (`z = −8`), next to `ground_cargo`, toward the lumber car |
| Beat | Cargo → first flatbed stays; flatbed → tractor → lumber load; lumber → **tank nose** → up the barrel |
| Tractor collide | One standable AABB (hood / body band); shared root with draw |
| Tank collide | Replace the tall tank AABB with collide-only dome fidelity: low nose toward lumber and low rear tip, both rising to mid-barrel (undrawn `MapRamp`s + thin mid seal). No extra present mesh |
| Foot SFX | Tractor stand → **steel** (SFX-only) |

Kenney ships visuals only — tank clash was our naive kit AABB (**092**). This feature adds fidelity to **that** volume so the curved front is attainable (under lumber top + jump peak) and the rest is a short incline / step climb.

## Ownership / delivery

| Concern | Owner |
| --- | --- |
| Tractor pose / scale / seat | Map **a** def (`train.tractor` beside `ground_cargo`) |
| Tractor mesh + car-kit materials | Cooked pack **`maps-a`** |
| Tractor draw / collide | Client present; lit under **089**; AABB from the same root |
| Tank collide shape | Client `MapWorld` build from the tank unit root (ramp / bands); present mesh unchanged |

Do not shift consist order, cargo pose, rails, or yard solids.

## Out of scope

- Driving, animation, towing, hazards.
- Drawn ramp / second tank mesh; per-triangle collide.
- Relocating cargo / consist or changing jump peak.
- Second vehicle or car-kit fleet.

## Acceptance

- Tractor sits south of the home track beside the ground lumber, readable as unload gear.
- Its top is standable; a normal jump reaches it from the mid empty flatbed (and from cargo if useful).
- From the tractor, a normal jump reaches the lumber-car load roof.
- From the lumber load, a normal jump reaches the tank’s **nose** support (not the old full-height AABB roof).
- Walking / short hops up either dome collide toward mid-barrel work without a max-height leap.
- Tank rear tip is the same low collide treatment as the nose (mirror ramp).
- Tank present is still the single Kenney mesh; no new drawn collide props.
- Layout truth is the cooked map def + authored kit collide; present does not invent a second placement.
- Lineup / non-map paths unchanged.
