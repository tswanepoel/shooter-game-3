//! Held WASD + Shift sprint edge + jump edge + weapon-cycle edge + LMB fire for mounted play.

/// Held WASD + Shift sprint edge + jump edge + weapon-cycle edge + LMB fire for mounted play.
#[derive(Debug, Default, Clone)]
pub struct MoveInput {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    jump_edge: bool,
    sprint_edge: bool,
    /// Accumulated wheel steps: +1 next weapon, −1 previous (021).
    weapon_cycle: i8,
    /// Session LMB held (038 fire).
    fire_held: bool,
    /// Session B held (039 emote wheel).
    emote_held: bool,
    /// Rising edge of B this frame window.
    emote_press: bool,
    /// Falling edge of B this frame window.
    emote_release: bool,
}

impl MoveInput {
    pub fn set_key(&mut self, code: &str, pressed: bool) {
        match code {
            "KeyW" => self.forward = pressed,
            "KeyS" => self.back = pressed,
            "KeyA" => self.left = pressed,
            "KeyD" => self.right = pressed,
            _ => {}
        }
    }

    pub fn note_jump_press(&mut self) {
        self.jump_edge = true;
    }

    pub fn take_jump(&mut self) -> bool {
        let j = self.jump_edge;
        self.jump_edge = false;
        j
    }

    pub fn note_sprint_press(&mut self) {
        self.sprint_edge = true;
    }

    pub fn take_sprint(&mut self) -> bool {
        let s = self.sprint_edge;
        self.sprint_edge = false;
        s
    }

    /// One wheel notch: positive `delta_y` (scroll down) advances primary→secondary→unarmed.
    pub fn note_weapon_wheel(&mut self, delta_y: f64) {
        if delta_y > 0.0 {
            self.weapon_cycle = self.weapon_cycle.saturating_add(1);
        } else if delta_y < 0.0 {
            self.weapon_cycle = self.weapon_cycle.saturating_sub(1);
        }
    }

    /// Signed cycle steps since last take (+ next, − previous).
    pub fn take_weapon_cycle(&mut self) -> i8 {
        let c = self.weapon_cycle;
        self.weapon_cycle = 0;
        c
    }

    pub fn set_fire_held(&mut self, held: bool) {
        self.fire_held = held;
    }

    pub fn fire_held(&self) -> bool {
        self.fire_held
    }

    pub fn set_emote_held(&mut self, held: bool) {
        if held && !self.emote_held {
            self.emote_press = true;
        }
        if !held && self.emote_held {
            self.emote_release = true;
        }
        self.emote_held = held;
    }

    pub fn emote_held(&self) -> bool {
        self.emote_held
    }

    pub fn take_emote_press(&mut self) -> bool {
        let v = self.emote_press;
        self.emote_press = false;
        v
    }

    pub fn take_emote_release(&mut self) -> bool {
        let v = self.emote_release;
        self.emote_release = false;
        v
    }

    pub fn is_move_key(code: &str) -> bool {
        matches!(code, "KeyW" | "KeyA" | "KeyS" | "KeyD")
    }

    pub fn is_sprint_key(code: &str) -> bool {
        matches!(code, "ShiftLeft" | "ShiftRight")
    }

    pub fn is_emote_key(code: &str) -> bool {
        code == "KeyB"
    }

    pub fn clear_keys(&mut self) {
        *self = Self::default();
    }

    /// Digital forward / strafe in −1…1 (W/S and A/D).
    pub fn axes(&self) -> (f32, f32) {
        let forward = (self.forward as i8 - self.back as i8) as f32;
        let strafe = (self.right as i8 - self.left as i8) as f32;
        (forward, strafe)
    }
}
