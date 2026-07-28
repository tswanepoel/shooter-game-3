# Feature 062 - Mouse sensitivity

One client preference scales all session mouse movement: soft pointer (**061**) and look (**007**, mounted drive, flycam). Fixed base rates at multiplier `1.0`: soft pointer 1 px → 1 px; look `LOOK_SENS_RAD_PER_PX` (`0.002` rad/px). The multiplier preserves that ratio — it is not a separate look-vs-UI tune.

Debug-tools console is the first editor; the setting is client-owned and applies in production. Product settings UI (**063**) may add another editor later.

Depends on **003** (debug registry / console), **007** (session deltas), **061** (soft pointer).

## Preference

| Fact | Draft |
|------|--------|
| Unit | Dimensionless multiplier |
| Default | `1.0` |
| Look base | `LOOK_SENS_RAD_PER_PX` = `0.002` rad/px (fixed) |
| Apply | `session_delta × multiplier`; look also × `LOOK_SENS_RAD_PER_PX` |
| Valid set | Finite and positive; otherwise keep the last good value |
| Persistence | Browser cookie, same path as room code |
| Debug | `mouse.sens` cvar; `mousesens [value]` prints or sets it |

## Ownership

| Piece | Role |
|-------|------|
| Client preference | Owns value; load from cookie on init, save on change |
| Soft pointer (**061**) | Scales session deltas when armed |
| View / play frame | Scales look session deltas (× `LOOK_SENS_RAD_PER_PX`) |
| Debug registry (**003**) | Reads/writes the same preference — not a separate store |
| Host bridge (**003**) | Same `mousesens` line as the console |

## Acceptance criteria

- Multiplier `1.0` restores pre-062 feel (`0.002` rad/px look, 1:1 soft pointer); changing it scales both immediately.
- Value survives reload via cookie.
- Help lists the command/cvar; bad values leave the prior good value.
- Host bridge sets the same preference as the shell.
