# Feature 024 - MP remotes

Joined clients see other players move. Server snapshots carry every relevant peer; the local client draws remotes with the same present-pose path the self uses for body and locomotion.

Depends on **022** and **023**.

## Snapshot peers

- Each **Snapshot** includes `you` (023) and **`others`**: zero or more remote entries.
- A remote entry is a net drive view, enough to rebuild **present pose** for walk, sprint, jump, and stand (016 / 019 / 020):

| Field group | Content |
| --- | --- |
| Identity | `PlayerId`, character, active weapon (and slots if mesh needs them) |
| Placement | position, ocular yaw/pitch |
| Locomotion | mode, phase, air/jump state as required for present |

- **PlayerLeft** `{ id }` removes a remote when a peer disconnects.
- Server is source of peer set and poses; client `mp/remotes` holds the table and feeds presentation.

## Presentation

- Remotes draw with **present pose** only (017): body and active blaster from net drive.
- Local joined self stays first-person look-mounted (017); remotes are third-person bodies in the world.
- Same kit rebuild rules as solo self: pure function of drive + shared character data.

## Acceptance criteria

- Two joined clients each see the other’s body at the server pose.
- Remote walk, sprint, jump, and stand match the locomotion the peer is driving through server sim.
- Disconnect removes the remote from the other client’s world.
- Remote meshes use existing present/kit paths; `mp/` supplies drive only.
