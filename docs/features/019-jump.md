# Feature 019 - Jump

Space makes the mounted self hop; sim owns the arc and the landing.

## Acceptance Criteria

- Session-active **Space** (press edge) while grounded jumps; peak about **1.1 m**, short air time.
- Sim leaves **y = 0** with vertical velocity, falls under constant gravity, lands on **y = 0**.
- Air is first-class locomotion; land restores stand or walk from the current wish (016).
- Horizontal velocity locks at launch (no mid-air WASD steer); look still owns turn; walk phase freezes aloft.
- Present body uses standing aim in air; camera and look origin follow sim height.
- Flycam keeps **Space** for vertical fly; jump is mounted-only.
