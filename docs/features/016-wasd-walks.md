# Feature 016 - WASD walks

The self walks on the ground under session keyboard control. Sim owns where the feet stand and the drive state that rebuilds the body. Presentation plays the character walk so the legs stride in step with that motion. Look still owns all turning. Mounted view and body placement follow sim immediately.

## Philosophy

- **Sim is ground truth and has no lag.** Walk wish, walk phase, and position apply immediately in the simulation.
- **Pose is a pure function of sim drive.** Root placement, look, locomotion mode, and walk phase determine the full body. The same inputs always rebuild the same joints. That is the foundation for part-accurate hits, markers, and later host or server resolve.
- **Presentation follows sim.** The client draws the Kenney character walk from sim drive only. It does not invent a second position or a private stride.
- **Stride matches speed.** Walk playback rate and stride length lock to move speed so feet plant cleanly on the ground.
- **Keys translate; look rotates.** WASD only changes horizontal motion. Mouse look (015) remains the sole source of yaw and pitch.
- **Arcade response.** Constant speed, instant start and stop. Feel leads; realism follows the clip match only.

## Walk command

- While the input session is active (007), **W A S D** set a horizontal walk wish.
- Wish is **look-yaw relative** on the ground plane: W and S along the horizontal look direction, A and D strafe.
- Combined directions **normalize** to unit length so diagonals keep the same speed as cardinals.
- Wish is live only while session-active; leaving the session clears the wish.
- Pitch does not tilt the path into the air: motion stays on the ground plane.

## Sim drive

- Walk speed is **\(2\sqrt{3}\) m/s** (~3.46), constant while the wish is non-zero — Kenney `walk` at 1× (stance sole slip from leg length and swing).
- Sim integrates **position** on the ground plane from the wish each step. Feet stay on **y = 0**.
- Sim keeps **locomotion mode** (stand when wish is zero, walk when non-zero) and **walk phase** advancing with distance so phase is reconstructible at any time.
- Position, mode, and phase are first-class self state — reportable later for multiplayer and rebuildable for hits.
- Body facing and mounted eyes stay sim-snapped to look and position as in 015 / 013.

## Stride presentation

- While mode is walk, the character plays the **Kenney walk** loop from the character kit (010 / 013).
- Playback rate locks to walk speed so **one full cycle covers \(v \cdot T\)** (clip at 1× when moving at walk speed). Fine-tune by inspection if feet skate.
- While mode is stand, the body holds the standing aim pose from look (015).
- Upper-body aim from look continues to apply on top of the locomotion pose so walking and aiming read together.

## View and modes

- When mounted (006), the render view stays on the self’s eyes and moves with sim position and look.
- Flycam keeps its own free inspection move; the mounted self uses this walk path only.
- Debug lineup (011 / 012) stays its own inspection row.

## Later hits (foundation)

- Full body at time *t* is rebuildable from sim drive: root, look, mode, phase, and the shared character clips.
- Future collision smarts target **Kenney parts** from that rebuilt pose (bone-accurate volumes on the real skeleton).
- Local markers may resolve on the presenting client from that same pose function. Competitive resolve later uses the same function on host or server with time history.

## Acceptance criteria

- Session-active WASD drives a look-yaw-relative horizontal wish; diagonals are normalized; speed is a constant **\(2\sqrt{3}\) m/s** with instant response.
- Sim position updates on the ground plane (**y = 0**) from that wish; position is first-class self state.
- Sim carries locomotion mode and walk phase locked to move speed so pose is a pure function of sim drive plus shared clips.
- While walking, the self presents the Kenney walk with stride locked to walk speed so feet plant cleanly.
- While standing, the self holds the aim pose from look (015).
- WASD changes translation only; look remains the only rotation command.
- Mounted eye view and body placement follow sim position and look immediately.
- Flycam remount restores the walked self at its sim pose.
- Kit identity, blaster hold, and fixed aim (013 / 015) continue to apply while walking.
