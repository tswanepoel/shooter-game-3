# Feature 049 - Residual homes

Fire impulse and hit impulse leave motion on the **figure**: fold on hip, right shoulder, and neck; twist on the right shoulder; short bore travel on the grip socket. Those residuals fall over time. Fall from fire impulse slows while **fire continues**. Ontology and names live in [`docs/concepts.md`](../concepts.md); this feature is the ship contract to match that home for residual motion.

**048** already put residual on local joints and weapon line. **049** finishes alignment: drop the parallel **fire heat** climb layer from **047**, keep hit flinch when unarmed, present remotes with the same joint and grip residual path as self, and rename code that still speaks in kick / aim-residual bags so it matches the ontology.

Concepts with this feature: neck is fold-only; fall wording is “while fire continues”; grip socket owns bore travel and that travel falls, without re-stating joint impulse rules.

Heat made long sprays stick by multiplying fall time from a hidden meter. After 049, stickiness comes from residual still on the joints and from slower fall while fire continues (including an unfinished fixed string after the press ends). Restoring a heat-like layer later needs a real home in concepts first.

Muzzle indices on discharge and spawn stay. They are deterministic flash ground truth for present and net, not body motion.

## Acceptance criteria

- Fire residual and hit residual live on the figure’s hip, right shoulder, neck (fold), right-shoulder twist, and grip bore travel, with fall as above; fire control keeps cadence and gates.
- Fire heat (rise, recover, fall multipliers) is removed; tests track stack and continue-fall, not a heat meter.
- While fire continues, fire residual falls more slowly than when fire has ended; a burst string still counts after the press ends.
- Unarmed clears fire residual, sway, and grip bore; hit residual remains so an unarmed figure still flinches.
- Remotes apply peer fire residual on the same joint and grip homes as self (same proportion split as local fire impulse), including grip bore on the held mesh.
- Public sim names for fire-impulse size and residual fall match concept language (`WeaponKick` / `kick` / `aim_residual` and similar are renamed at call sites).
- Local 048 behaviour holds: weapon line after placement, view on the head chain, unison apply per discharge, reticle and projectiles on weapon line.
- Concepts mend for neck fold-only, fall “continues,” and thin grip travel ships with this feature.
- Tests cover continue-fall (including burst tail), unarmed hit flinch, remote residual on the full chain, and the renames.
