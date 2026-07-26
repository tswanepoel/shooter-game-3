//! Game simulation logic crate.
//!
//! Shared ground-truth world data and pure sim rules.
//! World space: 1 unit = 1 metre, Y-up, XZ ground plane.

mod ammo;
mod config;
mod emote;
mod fire;
mod health;
mod self_state;
mod weapons;

pub use ammo::*;
pub use config::*;
pub use emote::*;
pub use fire::*;
pub use health::*;
pub use self_state::*;
pub use weapons::*;
