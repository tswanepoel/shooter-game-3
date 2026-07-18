# Feature 014 - Mounted look and aim pose

Mouse look aims a standing self. The first-person view turns with the eyes; the body chases; the held blaster and reticle show where the weapon actually points. Feet stay put. No locomotion.

## Philosophy

- The player aims with a body, not a free-floating camera-gun.
- Looking and aiming are related but not identical.
- Lag is intentional: the body trails the intended look; the reticle shows where the weapon actually points.
- One weapon line always: reticle attitude, held blaster attitude, and (when shots exist) projectile direction are the same line.
- First person turns the view instantly; lag lives in the body so the reticle can leave screen centre while the torso and arm catch up.
- At rest, horizontal lag dies out; vertical lag does not fully die out near extreme look-up/down — that is intended.

## Two lines

- **Ocular line:** where the eyes look. First-person view rotation and screen centre.
- **Weapon line:** where the blaster points. Reticle, weapon mesh, and shots all follow this.

## Two tracks

- **Functional chain** (drives weapon line and reticle lag): input → ocular line → torso → shoulder → weapon line.
- **Presentation track** (neck/head only): input → neck/head cosmetic.
- The weapon line does not run through the neck. Torso and shoulder carry functional aim.

## Input / ocular command

- While the input session is active (007), pointer-lock mouse look commands gaze as world azimuth (yaw) and elevation (pitch).
- Ocular line snaps to the command immediately.
- Elevation is capped short of straight up/down, about ±80°, so the weapon can still reach past the view toward the screen edge.
- Ignore pathological single-frame pointer spikes (on the order of tens of pixels).
- Self position and kit identity stay as in 013; this feature does not move the feet.

## Per-axis behaviour

### Azimuth (horizontal)

1. Ocular snaps to command.
2. Torso yaws toward ocular, with lag.
3. Arm azimuth equals torso azimuth (no separate functional shoulder yaw).
4. At rest with input held: torso meets ocular; weapon–ocular azimuth separation → 0; reticle recentres horizontally.

### Elevation (vertical)

1. Ocular snaps to command, within the ±80° cap.
2. Torso pitches toward its share of the command, with lag.
3. Shoulder pitches toward its share, with lag (the main elevation worker).
4. Shoulder keeps its full budget even when ocular is hard-capped, so the weapon line can reach steeper elevations than the eyes.
5. At rest: links settle into a partitioned pose. Weapon–ocular elevation separation is generally non-zero and grows large near the cap. Reticle above centre at max look-up while the view stays below vertical is intended.

## Range budget (elevation, settled pose)

- Each functional link has a maximum bend at full look, scaled linearly with commanded elevation / ocular cap.
- At full look, use “outward” (look up) or “inward” (look down) maxima separately — they are not symmetric.
- Settled maxima (radians, at full ocular command):
  - torso: inward ≈ 0.262 (~15°), outward ≈ 0.131 (~7.5°)
  - shoulder: inward ≈ 1.309 (~75°), outward ≈ 1.440 (~82.5°)
- Settled weapon elevation ≈ torso + shoulder → about ±90° at full command while eyes stop near ±80°.
- Head/neck cosmetic has its own bend budget and rates; it is presentation only and sits outside the weapon-line budget.
- On each axis, functional shares of the commanded rotation (ocular included) are meant to sum to one. Lagged total on elevation must match lagged total on azimuth so diagonal flicks do not arc or slide on screen. Tune both axes together.

## Lag / chase feel

- Body links approach their targets with frame-rate-stable exponential chase (higher rate = snappier).
- Chase rate softens when look input is fast and stiffens when input is slow or held, so flicks trail and settle catches up.
- Rough rate bands (1/s): head cosmetic snappy/laggy ~368/96; torso yaw ~256/32; torso and shoulder pitch ~48/10.
- Smooth the measured look speed before feeding it into rate selection (slow smoothing, order of a few 1/s).

## First-person view

- Rotation follows the ocular command directly (instant look).
- Position is on the face: the image of a fixed local offset under the posed `head` node (after hold, functional chain, and head cosmetic). Offset is chosen once so the point sits in the face volume; it is not a free offset from the feet.
- Camera lag and weapon-line lag are separate: view turns now; torso and shoulder create reticle offset from centre.
- When mounted (006), this is the sole view pose. Flycam remount restores it.

## Weapon line

- Sample the weapon line from the arm-mounted blaster after torso and shoulder aim are applied (`holding-right`, `arm-right`, blaster grip — same attachment contract as 011 / 013).
- If building the line analytically from the ocular line, apply separations in this order only:
  1. azimuth separation about world vertical
  2. elevation separation about the ocular’s lateral axis  
  Reversing order mixes axes on diagonal input.
- Azimuth separation is transient (only while the chain catches up).
- Elevation separation is set by the pose budget and persists at rest.
- Reticle, blaster mesh, and shots all sit on this line.

## Reticle

World marker on the weapon line — not screen centre.

**Position**
- First valid bore hit from the muzzle (world + characters; skip local body and near-muzzle self hits). Ray length = blaster max range when known, else 100 m. Miss → 100 m along the bore.
- Nudge ~3 cm toward the camera so it sits just in front of surfaces.
- Hide when dead, unarmed, or when the view of that point is blocked (camera-through-pixel check; not an eye→aim world ray). Local body and the aim surface itself do not count as blockers.

**Paint**
- World billboard on that point, depth-ignored.
- Constant ~6 px on-screen disc: white fill, black ring. Same size at any range (scale with distance / FOV). No spread, no ADS variant.

## Held blaster presentation

- Base character pose is `holding-right`; this feature layers aim on that hold.
- Blaster is parented to `arm-right` at the grip; orientation maps the blaster’s authored forward onto the held-arm forward with a consistent grip roll so the muzzle exits correctly.
- Weapon mesh expresses the same weapon–ocular separation the reticle shows.
- Rotation only for aim on the weapon; one consistent azimuth sign convention.

## Body hierarchy (no corkscrew)

- Root carries absolute gaze azimuth once.
- Child joints apply relative offsets only: torso pitch, neck pitch/yaw (cosmetic), arm/shoulder pitch (functional).
- A full turn reads as one body rotation, not stacked absolute yaws.

## Hard rules

1. Reticle = weapon attitude (= projectile direction when shots exist). One weapon line always.
2. Reticle and weapon inherit lag only from the torso–shoulder functional chain.
3. Functional elevation/azimuth budgets stay balanced across axes; lagged totals match so diagonals feel even.
4. Ocular elevation cap is below ±90°; weapon budget may reach about ±90°.
5. At rest: horizontal reticle recentres; vertical may stay offset near the look limit.
6. Replicated basis is **position + ocular**; body lag is derived per client (multiplayer later).

## Acceptance criteria

- Self remains first-class state (013). Ocular azimuth and elevation are part of that state; the client presents the cascaded pose and does not invent a second self.
- Reticle is the bore-hit world billboard above (hide/block/paint rules).
- Session-active mouse look drives ocular; session inactive does not accumulate look.
- Turn then stop: azimuth separation decays to zero; reticle recentres horizontally.
- Fast diagonal flick: screen-space lag comparable in X and Y.
- Max look-up: reticle sits above centre while the view is still below vertical.
- Observed 90° turn: one coherent body turn, no corkscrew.
- Mounted camera sits on the posed head (face offset) and rotates with ocular; remount after flycam restores that path.
- No foot movement, no fire, no debug-only-only path for the cascade (production and dev both run it).

## Out of scope

- Locomotion (WASD, translation, physics capsule).
- Firing, recoil, projectiles.
- Debug lineup pose changes (011 / 012 stay their own row).
