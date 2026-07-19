# Feature 015 - Fixed aim

Mouse look aims a standing self. Aim is where the first-person camera looks: the centre of the view. The eyes carry that view. The body holds a blaster as presentation. Feet stay put. No locomotion.

## Philosophy

- **Sim is ground truth and has no lag.** Look command and self pose truth apply immediately in the simulation.
- **Local first person is sim-synchronised.** What the local player sees for aim is the sim now — the view does not chase.
- **Aim is the view centre.** Reticle, and later hits and projectiles, use the camera forward at screen centre.
- **The eyes look; the camera mounts on the eyes.** Head and neck mesh are body presentation around that. For other players later, render may soften how their body is shown; that softness is presentation only and never lives in sim.
- **The held blaster does not define aim.** Grip and muzzle support how the gun is drawn and, later, where muzzle effects spawn. They do not steer the reticle or the shot direction.

## Look command

- While the input session is active (007), pointer-lock mouse look sets gaze as world azimuth (yaw) and elevation (pitch).
- Elevation spans the full vertical range: **±90°** (straight up and straight down).
- Ignore pathological single-frame pointer spikes (on the order of tens of pixels).
- Self position and kit identity stay as in 013; this feature does not move the feet.

## View and reticle

- When mounted (006), the one render view sits on the self’s **eyes** and rotates with the look command.
- Local mounted view orientation matches sim look every frame.
- The reticle is a **fixed screen-centre** mark (same look direction as the camera).

## Body and blaster

- The body is posed from sim look so a standing self holding a blaster reads as aiming along the view.
- Arm holds the blaster under the existing grip attachment (011 / 013).
- Muzzle points remain available for debug lineup (012) and for future effects; they are not an aim basis.

## Remotes (later)

- When other players exist, the local client may lag or smooth **their** rendered bodies for kind motion.
- Sim for every self stays immediate. A remote can act on you from sim before your render has finished showing their aim — that is an accepted consequence of render-only remote lag.

## Acceptance criteria

- Look yaw and pitch are first-class sim state on the self; they update immediately from session mouse look when the session is active.
- Local first-person aim path has no chase: mounted eye view and screen-centre reticle match sim look.
- Elevation is free through **±90°**.
- Reticle is dead-centre on the view; it does not track the bore or leave centre to show body motion.
- Held blaster remains arm-attached presentation on the self; aim direction is camera / view centre.
- Flycam remount (006) restores this eye-mounted, sim-synced look.
- No foot movement and no firing required in this feature; the aim basis is in place for both when they arrive.
- Debug lineup (011 / 012) stays its own inspection row.
