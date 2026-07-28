//! Host unit tests for pure client logic.
//!
//! Product modules are wasm32-only. These path-includes keep sim-adjacent unit
//! tests running under host `cargo test`.
//!
//! `#[path]` is relative to `src/`. Keep includes flat (no virtual `mp/` nest) so
//! Linux does not open through missing intermediate directories.

#[path = "view.rs"]
mod view;

#[cfg(feature = "debug-tools")]
#[path = "debug/registry.rs"]
mod registry;

#[path = "mp/apply.rs"]
mod apply;
#[path = "mp/clock.rs"]
mod clock;
#[path = "mp/drive.rs"]
mod drive;
#[path = "mp/phase.rs"]
mod phase;
#[path = "mp/remotes.rs"]
mod remotes;

#[path = "input/soft_pointer.rs"]
mod soft_pointer;

#[path = "preferences.rs"]
mod preferences;
