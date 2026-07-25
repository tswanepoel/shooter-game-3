//! Player self: position, look, walk drive, and body joints.

mod loco;
mod pose;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub use loco::*;
pub use pose::*;
pub use state::SelfState;
pub use types::*;
