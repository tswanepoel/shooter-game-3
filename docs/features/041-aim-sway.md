# Feature 041 - Aim sway

While you hold a blaster, aim breathes a little. That sway is real sim state on the same aim offset as kick (**040**): it moves the reticle and where shots go — not decoration on a dead-centre pin.

Look is still where you point. Sway is a small ongoing offset that stacks with kick. It is **blaster-specific**: each gun has its own resting feel (busier SMG, near-frozen sniper, heavier slow hold on big guns).

The motion is stacked bands, not a single loop the eye can learn:

- **Breath** — slow, mostly vertical
- **Tremor** — soft micro-band (keep amp tiny if faster)
- **Drift** — slow wander that mean-reverts so the path never closes neatly

Optional: damp sway while look rate is high, ease back when still. Sway is an aim-hold signal only; gait owns body present, not the pin.

Reticle and shots use full sway. The held mesh uses the same offset at lower gain so near-field draw does not read as thrash; kick stays full on the mesh.

## Acceptance criteria

- Armed hold advances blaster-specific sway in sim.
- Reticle and shots use look + kick + sway.
- Held mesh follows kick fully and sway at reduced gain.
- Unarmed / flycam: no gameplay sway.
- Sway stays slight; multi-band, not a simple figure‑8.
