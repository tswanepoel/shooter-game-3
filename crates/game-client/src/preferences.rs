//! Client-owned preferences (062 mouse sensitivity).

pub const DEFAULT_MOUSE_SENS: f32 = 1.0;

const MOUSE_SENS_COOKIE: &str = "sg_mouse_sens";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseSensitivity {
    multiplier: f32,
}

impl MouseSensitivity {
    pub fn new() -> Self {
        let multiplier = load_mouse_sens_cookie().unwrap_or(DEFAULT_MOUSE_SENS);
        Self { multiplier }
    }

    pub fn multiplier(&self) -> f32 {
        self.multiplier
    }

    /// Set when valid (finite, positive). Persists to cookie on wasm32.
    pub fn set_multiplier(&mut self, value: f32) -> bool {
        if !is_valid_mouse_sens(value) {
            return false;
        }
        self.multiplier = value;
        save_mouse_sens_cookie(value);
        true
    }
}

impl Default for MouseSensitivity {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_valid_mouse_sens(v: f32) -> bool {
    v.is_finite() && v > 0.0
}

pub fn parse_mouse_sens_value(s: &str) -> Option<f32> {
    let v = s.parse::<f32>().ok()?;
    is_valid_mouse_sens(v).then_some(v)
}

/// Debug console / host: `mousesens` or `mouse.sens` with optional value.
pub fn execute_mouse_sens_command(line: &str, sens: &mut MouseSensitivity) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split_whitespace();
    let head = parts.next()?;
    if head != "mousesens" && head != "mouse.sens" {
        return None;
    }
    let args: Vec<&str> = parts.collect();
    if args.is_empty() {
        return Some(format!(
            "mouse.sens = {}  — client mouse sensitivity multiplier",
            sens.multiplier()
        ));
    }
    match parse_mouse_sens_value(args[0]) {
        Some(v) => {
            sens.set_multiplier(v);
            Some(format!("mouse.sens = {v}"))
        }
        None => Some("usage: mousesens <positive finite>".into()),
    }
}

#[cfg(target_arch = "wasm32")]
fn load_mouse_sens_cookie() -> Option<f32> {
    use wasm_bindgen::JsCast;

    let doc = web_sys::window()?.document()?;
    let html_doc: web_sys::HtmlDocument = doc.dyn_into().ok()?;
    let cookie = html_doc.cookie().ok()?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(&format!("{MOUSE_SENS_COOKIE}=")) {
            if let Some(v) = parse_mouse_sens_value(rest) {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn save_mouse_sens_cookie(value: f32) {
    use wasm_bindgen::JsCast;

    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };
    let _ = html_doc.set_cookie(&format!(
        "{MOUSE_SENS_COOKIE}={value}; path=/; max-age=31536000; SameSite=Lax"
    ));
}

#[cfg(not(target_arch = "wasm32"))]
fn load_mouse_sens_cookie() -> Option<f32> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn save_mouse_sens_cookie(_value: f32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_multiplier() {
        assert_eq!(MouseSensitivity::new().multiplier(), DEFAULT_MOUSE_SENS);
    }

    #[test]
    fn rejects_bad_values() {
        let mut s = MouseSensitivity::new();
        assert!(!s.set_multiplier(0.0));
        assert!(!s.set_multiplier(-1.0));
        assert!(!s.set_multiplier(f32::NAN));
        assert!(!s.set_multiplier(f32::INFINITY));
        assert_eq!(s.multiplier(), DEFAULT_MOUSE_SENS);
    }

    #[test]
    fn accepts_and_keeps_good_value() {
        let mut s = MouseSensitivity::new();
        assert!(s.set_multiplier(2.5));
        assert_eq!(s.multiplier(), 2.5);
        assert!(!s.set_multiplier(0.0));
        assert_eq!(s.multiplier(), 2.5);
    }

    #[test]
    fn command_print_and_set() {
        let mut s = MouseSensitivity::new();
        let out = execute_mouse_sens_command("mousesens", &mut s).unwrap();
        assert!(out.contains("mouse.sens = 1"));
        let out = execute_mouse_sens_command("mouse.sens 1.5", &mut s).unwrap();
        assert_eq!(out, "mouse.sens = 1.5");
        assert_eq!(s.multiplier(), 1.5);
        let out = execute_mouse_sens_command("mousesens nope", &mut s).unwrap();
        assert!(out.contains("usage:"));
        assert_eq!(s.multiplier(), 1.5);
    }
}
