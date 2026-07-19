//! Movement slice: client intents for server sim apply.

use crate::types::{Seq, SessionKey, Tick};

/// C→S: one frame of movement intent under session key echo.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Input {
    pub seq: Seq,
    pub echo_key: SessionKey,
    pub echo_issued_tick: Tick,
    /// Look-relative forward wish (−1…1).
    pub wish_forward: f32,
    /// Look-relative strafe wish (−1…1).
    pub wish_strafe: f32,
    pub look_yaw: f32,
    pub look_pitch: f32,
    pub jump: bool,
    pub sprint_tap: bool,
    /// −1 / 0 / +1 weapon cycle.
    pub weapon_cycle: i8,
}
