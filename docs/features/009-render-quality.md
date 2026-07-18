# Feature 009 - Render quality

The presented frame should look sharp on HiDPI displays and free of obvious edge/texture aliasing. This is a client present-path polish pass — not a lighting, material, or art change.

## Acceptance Criteria

- **HiDPI buffer:** the WebGPU surface matches CSS size × `devicePixelRatio` (clamped to sane limits). egui (and any other UI) uses the same buffer/CSS ratio so it does not double-scale. Resize stays correct when the window or DPR changes.
- **MSAA 4×:** the main scene pass (clear, lineup, grid, depth) renders at 4 samples and resolves to the swapchain. One quality path for all scene geometry — not a per-mesh toggle.
- **Albedo mips:** character (and any similar) albedo textures get a full mip chain on upload; sampling uses linear min/mag with linear mip filter. Wrap/address modes for kit UVs are unchanged (008).
- Screenshot (004) and input session (007) keep working against the presented frame; no separate “pretty” capture path.
- Unlit albedo presentation and kit facts (008) stay as-is. No lighting, PBR, anisotropy, or FXAA/TAA in this feature.
- Out of scope: lit presentation mode, shadows, post-process stacks, variable MSAA quality cvars, and supersampling beyond the HiDPI buffer + 4× MSAA above.
