# Feature 047 - Kick climb tune

Kick already adds dirt and settles (**040**). No look climb. Climb under long fire comes from **dirt vs settle**, plus **fatigue** so settle is not a fixed compromise.

## Dials

| Dial | Role |
|------|------|
| **Dirt** | Per-shot pitch/yaw/back (weapon table). |
| **Settle** | Fresh-body recover (`settle_s` on table; ~50–55 ms). |
| **Fatigue** | Raw heat 0…1 rises per discharge; drains after the string. A **power curve** maps heat → weight (low early, hard late). Effective settle = `settle_s × (1 + weight × (mult − 1))`. |

Fixed settle alone can’t be both snappy taps and long-spray climb. Fatigue is that missing piece. Stack is uncapped — dirt vs settle plateau naturally.

## Dev HUD (tune)

Debug-tools top banner (`kickhud` / `hud.kick`, on by default):

| Field | Meaning |
|-------|---------|
| **fat** | Curved weight 0…1 (what scales settle) |
| **kP / kY** | Live kick pitch/yaw (degrees) |
| **set** | Effective settle (ms) |

## Acceptance criteria

- Short fire: kick settles on a snappy body-recover timescale.
- Sustained full-auto / burst: kick pitch climbs well above one shot (fatigue slows settle mid-string).
- After fire stops: fatigue drains; kick recovers without a permanent float hang.
- Still one kick layer on aim offset; camera stays look-only.
- Debug build can show fat / kick / set on the top banner for tuning.
