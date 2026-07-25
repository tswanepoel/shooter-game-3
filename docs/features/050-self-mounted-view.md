# Feature 050 - Self-mounted view

The first-person camera mounts the **self** that is already in the world: the eye socket on that figure’s head. Ontology already names this: [view](../concepts.md#view) is [look](../concepts.md#look) as the camera; look is the pose at the [eye socket](../concepts.md#eye-socket) on the [head](../concepts.md#head). This feature is the ship contract to mount that instance, not a second standing stand-in rebuilt from feet and look alone.

One body poses the self each frame (drive, clips, residual, death, emote — whatever already moves those joints). The camera reads **look** on **that** head: position at the eye socket, orientation the head’s. When fire impulse, hit impulse, locomotion clips, die, or any other effect moves the head, the view follows with no separate mount path per effect.

Reticle ray and combat ray still start at look. After this feature, that origin is the same eye socket on the self. Weapon line stays as concepts define it. Flycam still unmounts; remount returns to this self eye.

**017** split a look pose (stand + look for mount and aim) from a present pose (drawn body). **050** ends that split for the local view: one self, one head, one mount. Remotes stay third-person bodies as today; this feature is local mounted view and the look origin used for aim.

## Acceptance criteria

- Mounted view is look on the self’s posed head: eye-socket position, head orientation; same body the local client draws for that figure.
- Fire residual, hit residual, walk or sprint clip motion on the head chain, die collapse, and emote that move the head all move the camera without a second mount rebuild or per-effect camera code.
- Look origin for reticle and combat is that same eye socket.
- While alive and standing with no residual and no clip motion on the head, view still reads as head-mounted look (mouse look turns the view).
- Flycam remount restores this self eye mount.
- First-person may still hide the local head shell for draw; hide is draw only and does not invent a different mount origin.
- Concepts are unchanged; this feature implements the existing view / look / eye socket chain on the self instance.
