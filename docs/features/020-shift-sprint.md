# Feature 020 - Shift sprint

Shift tap latches a mounted sprint while stamina lasts; empty bar walks, and a minimum fill is required to start the next sprint.

## Acceptance Criteria

- Session-active **Shift** (press edge, not hold) while grounded with **forward wish (W)** latches sprint at a higher constant ground speed than walk (016). Strafe-only (A/D) or back (S) never sprints; W+strafe is fine.
- Sprint **stays on without holding Shift**; further Shift taps while latched do nothing (no cancel).
- Sprint drains stamina while active; dropping forward wish (or any zero wish) clears the latch and returns to walk/stand immediately.
- Stamina is limited; when it hits zero, sprint ends, latch clears, and motion falls back to walk (or stand) without stuttering on/off.
- While latched, stamina **keeps draining in air** (jump does not pause or refund the bar).
- Stamina replenishes over time only when **not** latched.
- Starting a sprint requires a **minimum stamina** threshold (not just any non-zero scrap) so premature re-engage cannot flicker.
- Tap below that threshold does not latch (no partial/weak sprint).
- Air (019) does not start a sprint; look still owns turn; flycam Shift sprint stays its own free-look path.
- While sprinting, drop the arm-up / aimed upper-body pose; the sprint locomotion owns the arms and they swing with the stride.
- Leaving sprint restores walk/stand aim layering as before (015 / 016).
- Sim owns stamina, sprint latch/active, and ground speed; present uses the faster locomotion drive (sprint clip if available, else walk at sprint rate).
