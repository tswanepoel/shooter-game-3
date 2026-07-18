# Feature 008 - Character lineup

Developers need a debug **character lineup**: a row of Kenney blocky characters in the empty scene so import scale, facing, and paint can be checked against the metre grid. Client-only presentation — not sim bodies or gameplay spawn.

## Acceptance Criteria

- Feature-gated with the debug tools surface (003). Production builds expose nothing. Show/hide goes through the command/cvar registry (and host bridge); no ad-hoc side paths.
- Source is the shipped character kit under `assets/characters/`: one mesh per letter (`character-a` …) with its matching albedo atlas (`texture-a` …). Geometry lives in the model file; paint is the external atlas, not vertex colours or a white factor alone.
- Load must tolerate the kit’s **unlit** materials (extension may be marked required). Present as flat albedo: texture × base colour factor — not lit PBR shading that darkens the paint.
- Atlas UVs may wrap and use the full range (including negative U). Sampling must not treat negative U as “untextured.”
- Lineup stands on the ground plane in world metres (002): **feet on `y = 0`** (root translate from bounds / sole, not floating or buried). Authored units may not be metric — apply **one root scale** that maps the kit into metres (from kit inspection; documented in `assets/characters/README.md`). Standing height is then **whatever the mesh is** after that scale — do not force a target height (e.g. eye-height) or per-part scale hacks. `STANDING_EYE_HEIGHT_M` remains camera/mount ground truth only.
- Default authored face is **+Z**. Align lineup facing with the scene’s look direction (stub cam looks −Z) so characters face the default view unless a deliberate offset is documented.
- Multi-part hierarchy (body, head, limbs) draws as one character at a placement; parts keep their relative bind layout. Static bind pose is enough.
- Shared ground-truth quantities stay in `game-sim`; the client does not invent alternate units or axes for the lineup.
- Ship kit facts once at `assets/characters/README.md`: face axis, unit/scale, texture pairing, unlit/albedo, UV quirks, hierarchy/bind pose — what later loaders need, written from inspection. No engine how-tos or debug commands there. Root `README.md` only lists the lineup toggle with the other debug tools (same pattern as F9 / flycam); it does not restate model facts.
- Out of scope: animation clips and grips, weapons/blasters, skinning beyond bind pose, gameplay characters, physics, shadows, full material graph, and a general prefab/entity system.
