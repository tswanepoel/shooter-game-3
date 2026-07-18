# Feature 003 - Debug tools

Developers need a durable in-game debug surface for inspection and on-demand intervention. The product is a **debug subsystem** (commands, knobs, read models, world draw), not a web page chrome. Interactive UI and automation hosts are thin transports over the same API.

## Acceptance Criteria

- Debug capability is a first-class, feature-gated subsystem. Production builds expose nothing (or inert stubs); dev builds own the real implementation.
- All interventions go through a **command** and **cvar** registry. Panels, keybinds, and external hosts only invoke that registry — they do not poke sim or client internals ad hoc.
- Layering is explicit: world-truth mutations are sim-facing; pure visuals (draw overlays, view helpers) stay client-side. The debug facade does not become a second source of truth.
- The primary interactive shell is an **in-engine** overlay (immediate-mode UI over the game renderer), toggled with backtick (`` ` ``). It captures input while open so game controls do not fight typing.
- A minimal console is enough for v1: open/close, command line, history, and `help`. Panels are views over commands/cvars/snapshots, not a custom widget framework.
- World-space **debug draw** is a sibling path (categories toggled via cvars), not something the UI toolkit itself renders as gameplay geometry.
- A thin **host channel** (e.g. dev-only JS bridge) calls the same registry so agents, scripts, and capture flows can intervene without driving the overlay UI.
- DOM is not the architecture. Browser chrome may assist tooling later; it must not own game intervention or debug state.
- Out of scope: scripting language, full entity property editor, networked admin/RCON, replacing browser or GPU profilers, and inventing a retained-mode UI kit.
- Relevant documentation stays aligned with this philosophy when the surface grows.
