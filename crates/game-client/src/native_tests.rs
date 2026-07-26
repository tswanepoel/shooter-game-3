//! Host unit tests for pure client logic.
//!
//! Product modules are wasm32-only. These path-includes keep sim-adjacent unit
//! tests running under host `cargo test`.
//!
//! Paths are relative to the virtual `native_tests/mp/` directory for the inline
//! `mp` module (hence `../../mp/...`).

#[path = "view.rs"]
mod view;

#[cfg(feature = "debug-tools")]
#[path = "debug/registry.rs"]
mod registry;

mod mp {
    #[path = "../../mp/apply.rs"]
    mod apply;
    #[path = "../../mp/clock.rs"]
    mod clock;
    #[path = "../../mp/drive.rs"]
    mod drive;
    #[path = "../../mp/phase.rs"]
    mod phase;
    #[path = "../../mp/remotes.rs"]
    mod remotes;
}
