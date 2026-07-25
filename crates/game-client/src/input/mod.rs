//! In-game input session: pointer lock on canvas click; browser eject ends it.

mod egui_bridge;
mod handlers;
mod move_input;
mod session;

pub use handlers::install_input_handlers;
pub use move_input::MoveInput;
pub use session::InputSession;
