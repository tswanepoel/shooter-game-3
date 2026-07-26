//! Baked weapon fire table.
//!
//! Blaster owns initial velocity and ammo kind (via class). Ammo mass is on
//! [`crate::AmmoKind`]. Fire-impulse size and base fall live here; continue-fall
//! scaling is on the figure.

mod class;
mod table;
mod types;

#[cfg(test)]
mod tests;

pub use class::*;
pub use table::*;
pub use types::*;
