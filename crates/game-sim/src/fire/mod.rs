//! Weapon fire gates, modes, and projectile motion (038/042/048/049).
//!
//! Cadence and discharge live here. Fire / hit / sway residual live on [SelfState].

mod equip;
mod projectile;
mod state;
mod sway;

#[cfg(test)]
mod tests;

pub use equip::*;
pub use projectile::*;
pub use state::*;
