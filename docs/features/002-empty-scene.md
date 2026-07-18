# Feature 002 - Empty scene

The game will show an empty 3D scene from a fixed eye-height vantage. Units and axes must match what later physics and characters will use.

## Acceptance Criteria
- World space uses metres (1 unit = 1 m) and Y-up (XZ ground plane).
- Configuration is pure ground-truth data (named real-world quantities), not magic numbers or behaviour encoded as values. Client consumes shared constants; it does not redefine them.
- A fixed perspective camera sits at standing eye height (~1.7 m), looking straight ahead. No movement or camera control.
- An empty scene is rendered (clear + minimal 3D path).
- A world-space debug grid is drawn on y = 0 as a client-only overlay (not a sim floor, collider, or gameplay surface): 1 m minor lines, 10 m major lines.
- Rapier is out of scope until something needs physics. `game-sim` remains the home for shared ground truth and, later, physics ownership.
- Movement, character models, and a real floor mesh are out of scope.
