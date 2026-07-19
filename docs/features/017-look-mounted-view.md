# Feature 017 - Look-mounted view

Mounted first person sits on a **look origin** rebuilt only from feet and look. Walk still drives how the body is drawn. Aim and the reticle stay a pure skill channel; stride still reads in the world.

Walk locomotion detail (modes, settle, stride presentation) lives in **016**. This feature is the mount and aim split only.

## Philosophy

- **One sim drive, two pure poses.** Root, look, locomotion mode, and walk phase remain the sole inputs (016). From that drive the game builds a **look pose** and a **present pose**. Same inputs always rebuild the same joints for each.
- **Look parents the view.** The mounted camera and the aim ray mount on the look pose’s eyes. Shoulder, neck, and head work from look (015) still shape that mount.
- **Walk parents presentation.** Locomotion mode and walk phase shape the present pose only — the body others see, and the local mesh under the view. Stride is body motion, not aim motion.
- **Aim is the view centre.** Reticle and later hits and projectiles use look origin and look direction at screen centre (015). That line does not ride the walk cycle.
- **Arcade skill stays clean.** Translation cost and look skill earn the shot. Gait phase does not move the crosshair.

## Pivot from 015 / 016

Through 016, “eyes” meant the face point on the **one** full body pose. After walks, that pose includes stride, so the mounted view and reticle rose and fell with every step.

This feature keeps head-mount and fixed aim. It splits **which pose** the mount samples:

| Consumer | Pose | Channels |
|----------|------|----------|
| Mounted view, reticle, aim ray | **Look pose** | Root placement + look (locomotion held at stand) |
| Drawn body (local and remote) | **Present pose** | Root + look + mode + phase |

The pivot is the second evaluation, not a second self or a free-floating height stub.

## Terms

- **Sim drive** — position, look (yaw and pitch), locomotion mode, walk phase (016).
- **Look pose** — body rebuilt from root and look with locomotion at **stand**. Head, neck, and upper aim chain match 015. No walk phase contribution.
- **Present pose** — body rebuilt from the full drive (016), including the walk clip whenever locomotion uses it.
- **Look origin** — face point on the look pose (same face offset idea as the mounted eyes in 013–015). Viewpoint and aim start here.
- **Look direction** — unit forward from look yaw and pitch. View orientation and aim ray.

Look origin is not a constant height above the feet alone, and not the present head during a stride. It is the eyes of the look pose: head-mounted on the look chain only.

## Look pose and mount

- Build the look pose from current root and look every frame, with locomotion fixed at stand.
- Place the mounted view at the look origin; orient it with look direction.
- Screen-centre reticle lies on that ray (015). Depth and paint rules stay as today.
- Upper-body aim from look still runs on this pose, so the viewpoint keeps shoulder and neck work from look under the camera.

## Present pose

- Build the present pose from the full sim drive as in 016 (stand, walk, and any in-between locomotion 016 defines), with look aim layered on top.
- Draw the self — body and held blaster — from the present pose.
- Local first person may show a small difference between present body and look mount while the walk clip is active; that is intended. Remote viewers always see the present pose.

## Sim and reconstructibility

- Sim still owns drive only. It does not store two skeletons.
- Both poses remain pure functions of drive plus shared character data. Part-accurate hits and markers later choose the pose that matches the question: aim ray → look pose; body part → present pose (or a time-matched rebuild of the same).
- Flycam remount restores the look-mounted path at the current drive.

## Acceptance criteria

- While the walk clip is active, the screen-centre reticle and mounted view hold steady in height against walk phase; look still aims freely through ±90°.
- While looking, the mounted view still reads as head-mounted on the look chain (torso / neck / head from look continue to change what the camera sees).
- The drawn self follows present pose from 016; stand uses the look aim pose when locomotion is stand.
- Look origin and look direction are defined only from root and look (look pose). Present pose alone may use mode and phase.
- Aim remains view centre: reticle, and later shots, follow look origin + look direction — not the present head during a stride.
- This feature adds the look-pose mount split; walk integration and stride presentation remain 016.
- Flycam remount restores look-mounted view at the current self drive.
- Kit identity, blaster hold, and fixed aim (013 / 015) continue under both poses.
