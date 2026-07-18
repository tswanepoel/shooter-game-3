# Feature 008 - Character lineup

Developers need a debug **character lineup**: Kenney blocky characters in the empty scene so import scale, facing, and paint can be checked against the metre grid. Presentation is client-only.

**Status:** character load rules, kit facts, and unlit presentation remain in force. The primary debug row in the product is the **blaster lineup** (011), which reuses this character path with a hold pose and per-blaster grip attachment.

## Acceptance criteria

- Gated with the debug tools surface (003). Dev builds own the implementation; release builds strip debug tools. Show/hide goes through the command and cvar registry (and host bridge).
- Source is the character kit under `assets/source/characters/`: one mesh per letter (`character-a` …) with its matching albedo atlas (`texture-a` …). Geometry lives in the model file; paint is the external atlas.
- Load accepts the kit’s **unlit** materials (extension may be required). Present as flat albedo: texture × base colour factor.
- Atlas UVs use the full range (including negative U) with **repeat/wrap** sampling.
- Lineup stands on the ground plane in world metres (002): **feet on y = 0**. One uniform root scale maps kit units into metres (character kit README). Standing height is the mesh height after that scale. `STANDING_EYE_HEIGHT_M` remains camera/mount ground truth only.
- Default authored face is **+Z**, aligned with the stub view (camera looks −Z) so characters face the default view.
- Multi-part hierarchy (body, head, limbs) draws as one character at a placement; parts keep their relative bind layout. Static bind pose is sufficient for paint and scale checks.
- Shared ground-truth quantities stay in `game-sim`; the client consumes them for placement.
- Kit facts ship once at `assets/source/characters/README.md`. Root README lists the lineup toggle with other debug tools and leaves model facts to the kit README.
