# Feature 013 - Self body and blaster

The render view mounts on the player **self**, but that self is still a pose stub: no body and no weapon (002 / 006). Character and blaster presentation are already proven on the debug lineup (008 / 011). The game path needs a real self — a standing body with a held blaster that owns the mount when not in flycam.

## Acceptance criteria

- Self is **first-class game state**, not a debug lineup slot. It exists in production and dev.
- Placement, facing, and kit identity (character letter, blaster letter) live in sim / shared ground truth. The client presents that state; it does not invent a second self. Metres and axes stay in `game-sim` (002).
- **Body + blaster:** one letter-matched pair (default e.g. `character-a` + `blaster-p`), loaded and held under the same rules and attachment contract as 008 / 011. Feet on **y = 0**; face **+Z** when yaw is identity. Identity is data on the self, not hard-coded only in the renderer.
- **Mount:** when mounted, the one render view (006) sits at the self’s **eyes** — a local 3D offset from the self origin (feet) — and looks along self facing. Not a free-floating height stub. Flycam remounts to this self.
- Assets load through cooked pack and asset ids (010). Presentation is ordinary scene geometry on the existing flat albedo path — not debug draw, not gated on debug-tools.
- Debug lineup (011 / 012) stays a separate inspection row; muzzle markers stay lineup-only.