# Feature 044 - Hit marker

When **you** land a hit on a live body (**043** impact claim on your client), show a brief **X** on the aim mark so the firer feels contact. Present-only chrome — not sim state, not health, not net.

## Look

- A **perfect X** (two equal diagonals, square proportions).
- Same paint language as the reticle: bright arms with a **black border** outline.
- Sits on the **aim reticle** (look + kick + sway — same place shots go), **not touching** the disc — a clear gap so the reticle stays the aim mark and the X is only the hit flash. Not locked to geometric screen centre when aim is offset.
- On each confirmed hit: **instant full opacity**, holds a **tiny beat**, then fades **very fast** (still barely more than a blink).

Stack or restart the fade if hits land in quick succession — each hit refreshes the flash; no permanent mark, no kill-only variant here.

## Acceptance criteria

- Local firer impact claim (**043**) shows the X once per claim on that client.
- X rides the aim reticle (look + kick + sway), perfect diagonals, reticle-style black border; gap from the reticle so lines never meet.
- Full opacity on hit, tiny hold, then super-short fade; successive hits re-flash.
- Present-only; no change to damage, aim, or wire.
