# Feature 076 - Drop bang-c; four contiguous bangs

Remove the assault-rifle-only bang. AR shares the sniper bang. Remaining discharge clips are contiguous `bang-a`…`bang-d` (no letter gap). Client present only.

Depends on **074** (letter bang banks), **068** (Web Audio / `sfx` pack). Same AR/sniper pairing as **071**.

## Change

| Before (**074**) | After |
| --- | --- |
| Five bangs `a`…`e`; AR → `bang-c`, sniper → `bang-d`, shotgun/launcher → `bang-e` | Four bangs `a`…`d`; AR + sniper → `bang-c`, shotgun/launcher → `bang-d` |

Former AR clip deleted. Former sniper clip becomes `bang-c`; former shotgun/launcher clip becomes `bang-d`.

## Assignment

| Bang | Classes | Letters |
| --- | --- | --- |
| `bang-a` | Pistol | `b` `i` |
| `bang-b` | SMG | `c` `g` `h` `l` `m` `p` |
| `bang-c` | Assault rifle, sniper | `d` `n` `q` `r` · `e` `f` |
| `bang-d` | Shotgun, launcher | `j` `k` `o` · `a` |

Chamber voices unchanged (**074**).

## Acceptance

- Cooked `sfx` pack has `bang-a`…`bang-d` only — no fifth bang letter.
- AR and sniper share one bang; pistol / SMG / shotgun+launcher stay distinct.
- Missing / failed SFX load still does not block fire.
