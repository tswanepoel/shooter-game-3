//! In-game input session: fullscreen + pointer lock on canvas click; browser eject ends it.
//! Soft pointer (061) routes session deltas to menus while locked.

mod egui_bridge;
mod handlers;
mod move_input;
mod session;
mod soft_pointer;

pub use handlers::install_input_handlers;
pub use move_input::MoveInput;
pub use session::InputSession;
pub use soft_pointer::SoftPointer;
