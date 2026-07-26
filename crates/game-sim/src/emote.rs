//! Emote catalog and self emote drive.

/// Wheel slot count (fixed v1).
pub const EMOTE_SLOT_COUNT: u8 = 4;

/// Centre dead-zone radius in wheel select space (normalized mouse accumulate).
/// Used by the client; documented here for the product rule.
pub const EMOTE_WHEEL_DEADZONE: f32 = 0.22;

/// One wheel slot / kit clip (039).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmoteDef {
    pub id: u8,
    pub label: &'static str,
    pub clip: &'static str,
    pub duration_s: f32,
}

/// Fixed order: N, E, S, W on the radial (Yes, No, Wave, Bow).
pub const EMOTE_CATALOG: [EmoteDef; 4] = [
    EmoteDef {
        id: 0,
        label: "Yes",
        clip: "emote-yes",
        duration_s: 0.67,
    },
    EmoteDef {
        id: 1,
        label: "No",
        clip: "emote-no",
        duration_s: 0.67,
    },
    EmoteDef {
        id: 2,
        label: "Wave",
        clip: "interact-right",
        duration_s: 0.67,
    },
    EmoteDef {
        id: 3,
        label: "Bow",
        clip: "pick-up",
        duration_s: 0.33,
    },
];

pub fn emote_def(id: u8) -> Option<&'static EmoteDef> {
    EMOTE_CATALOG.iter().find(|e| e.id == id)
}

pub fn emote_duration_s(id: u8) -> f32 {
    emote_def(id).map(|e| e.duration_s).unwrap_or(0.0)
}

pub fn emote_clip_name(id: u8) -> Option<&'static str> {
    emote_def(id).map(|e| e.clip)
}

/// Segment index from screen-ish select vector (x right, y up).
/// Angle 0 = +Y (north), increasing clockwise to match clock wedges.
/// Returns `None` inside the dead-zone.
pub fn emote_slot_from_select(dx: f32, dy: f32, deadzone: f32) -> Option<u8> {
    let r = (dx * dx + dy * dy).sqrt();
    if r < deadzone {
        return None;
    }
    // atan2(x, y): 0 at +Y, clockwise positive with x right.
    let mut ang = dx.atan2(dy);
    if ang < 0.0 {
        ang += std::f32::consts::TAU;
    }
    let sector = std::f32::consts::TAU / EMOTE_SLOT_COUNT as f32;
    let idx = ((ang + sector * 0.5) / sector).floor() as i32;
    let idx = idx.rem_euclid(EMOTE_SLOT_COUNT as i32) as u8;
    Some(idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_rejects_centre() {
        assert!(emote_slot_from_select(0.0, 0.0, EMOTE_WHEEL_DEADZONE).is_none());
        assert!(emote_slot_from_select(0.1, 0.1, EMOTE_WHEEL_DEADZONE).is_none());
    }

    #[test]
    fn north_is_yes() {
        assert_eq!(
            emote_slot_from_select(0.0, 1.0, EMOTE_WHEEL_DEADZONE),
            Some(0)
        );
    }

    #[test]
    fn east_is_no() {
        assert_eq!(
            emote_slot_from_select(1.0, 0.0, EMOTE_WHEEL_DEADZONE),
            Some(1)
        );
    }

    #[test]
    fn catalog_ids_are_dense() {
        for (i, e) in EMOTE_CATALOG.iter().enumerate() {
            assert_eq!(e.id as usize, i);
            assert!(e.duration_s > 0.0);
            assert!(!e.clip.is_empty());
        }
    }
}
