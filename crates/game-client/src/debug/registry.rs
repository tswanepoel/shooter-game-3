//! Command and cvar registry — the only intervention API.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
enum CVarValue {
    Bool(bool),
}

#[derive(Debug, Clone)]
struct CVar {
    value: CVarValue,
    help: &'static str,
}

/// Named commands and console variables. UI and host only call into this.
#[derive(Debug, Default)]
pub struct DebugRegistry {
    cvars: BTreeMap<&'static str, CVar>,
}

impl DebugRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_defaults(&mut self) {
        self.cvars.insert(
            "draw.grid",
            CVar {
                value: CVarValue::Bool(true),
                help: "client: world-space debug grid on y = 0",
            },
        );
        self.cvars.insert(
            "cam.fly",
            CVar {
                value: CVarValue::Bool(false),
                help: "client: debug flycam (view-only unmount; F8)",
            },
        );
        self.cvars.insert(
            "draw.lineup",
            CVar {
                value: CVarValue::Bool(false),
                help: "client: blaster lineup (armed Kenney row; grip/import check)",
            },
        );
        self.cvars.insert(
            "hud.net",
            CVar {
                value: CVarValue::Bool(true),
                help: "client: top FPS / tick banner",
            },
        );
        self.cvars.insert(
            "hud.residual",
            CVar {
                value: CVarValue::Bool(true),
                help: "client: fire residual fold / fall on top banner",
            },
        );
        self.cvars.insert(
            "hud.look",
            CVar {
                value: CVarValue::Bool(true),
                help: "client: look isolation (dx aY pre/post/end why) on top banner",
            },
        );
        self.cvars.insert(
            "draw.tracers",
            CVar {
                value: CVarValue::Bool(false),
                help: "client: debug projectile tracers (038)",
            },
        );
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.cvars.get(name)?.value {
            CVarValue::Bool(v) => Some(v),
        }
    }

    /// Parse and run a console line. Returns a human-readable response.
    pub fn execute(&mut self, line: &str) -> String {
        let line = line.trim();
        if line.is_empty() {
            return String::new();
        }

        let mut parts = line.split_whitespace();
        let head = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match head {
            "help" | "?" => self.cmd_help(),
            "grid" => self.cmd_grid(&args),
            "flycam" | "fly" => self.cmd_flycam(&args),
            "lineup" => self.cmd_lineup(&args),
            "nethud" => self.cmd_nethud(&args),
            "residualhud" | "kickhud" => self.cmd_residualhud(&args),
            "lookhud" => self.cmd_lookhud(&args),
            "remount" => {
                self.set_bool("cam.fly", false);
                "cam.fly = 0 (remount)".into()
            }
            "screenshot" | "shot" => "__REQUEST_SCREENSHOT__".into(),
            "mp" => self.cmd_mp(&args),
            "blaster" => self.cmd_blaster(&args),
            name if self.cvars.contains_key(name) => self.cmd_cvar(name, &args),
            _ => format!("unknown: '{head}'. type 'help'."),
        }
    }

    fn cmd_blaster(&self, args: &[&str]) -> String {
        let Some(raw) = args.first().copied() else {
            return "usage: blaster <a-r>".into();
        };
        let letter = raw.as_bytes().first().copied().unwrap_or(0);
        if raw.len() != 1 || !(b'a'..=b'r').contains(&letter) {
            return format!("usage: blaster <a-r> (got '{raw}')");
        }
        format!("__REQUEST_BLASTER_{}", letter as char)
    }

    fn cmd_mp(&self, args: &[&str]) -> String {
        match args.first().copied() {
            None | Some("status") => "__REQUEST_MP_STATUS__".into(),
            Some("join") => "__REQUEST_MP_JOIN__".into(),
            Some("leave") => "__REQUEST_MP_LEAVE__".into(),
            Some(other) => format!("usage: mp [join|leave|status] (unknown '{other}')"),
        }
    }

    fn cmd_help(&self) -> String {
        let mut lines = vec![
            "commands:".into(),
            "  help              this list".into(),
            "  grid [on|off|toggle]  set draw.grid".into(),
            "  flycam|fly [on|off|toggle]  debug flycam (F8)".into(),
            "  lineup [on|off|toggle]  blaster lineup (armed characters)".into(),
            "  nethud [on|off|toggle]  top FPS / tick banner".into(),
            "  residualhud [on|off|toggle]  fire residual fold / fall".into(),
            "  lookhud [on|off|toggle]  look isolation: dx aY pre/post/end".into(),
            "  remount           leave flycam, restore self view".into(),
            "  screenshot|shot   capture frame (F9)".into(),
            "  mp join|leave|status  WebTransport shared tick + remotes".into(),
            "  blaster <a-r>     equip letter on active slot (038; flips if needed)".into(),
            "cvars (get: name · set: name <value>):".into(),
        ];
        for (name, cvar) in &self.cvars {
            let val = match cvar.value {
                CVarValue::Bool(v) => {
                    if v {
                        "1"
                    } else {
                        "0"
                    }
                }
            };
            lines.push(format!("  {name} = {val}  — {}", cvar.help));
        }
        lines.join("\n")
    }

    fn cmd_grid(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("draw.grid", args, "grid")
    }

    fn cmd_flycam(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("cam.fly", args, "flycam")
    }

    fn cmd_lineup(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("draw.lineup", args, "lineup")
    }

    fn cmd_nethud(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("hud.net", args, "nethud")
    }

    fn cmd_residualhud(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("hud.residual", args, "residualhud")
    }

    fn cmd_lookhud(&mut self, args: &[&str]) -> String {
        self.cmd_bool_toggle("hud.look", args, "lookhud")
    }

    fn cmd_bool_toggle(&mut self, cvar: &str, args: &[&str], usage_name: &str) -> String {
        match args.first().copied() {
            None | Some("toggle") => {
                let cur = self.get_bool(cvar).unwrap_or(false);
                self.set_bool(cvar, !cur);
                format!("{cvar} = {}", !cur as u8)
            }
            Some(v) => match parse_bool(v) {
                Some(b) => {
                    self.set_bool(cvar, b);
                    format!("{cvar} = {}", b as u8)
                }
                None => format!("usage: {usage_name} [on|off|toggle]"),
            },
        }
    }

    fn cmd_cvar(&mut self, name: &str, args: &[&str]) -> String {
        if args.is_empty() {
            return self.format_cvar(name);
        }
        match self.cvars.get(name).map(|c| c.value.clone()) {
            Some(CVarValue::Bool(_)) => match parse_bool(args[0]) {
                Some(b) => {
                    self.set_bool(name, b);
                    format!("{name} = {}", b as u8)
                }
                None => format!("usage: {name} <0|1|on|off>"),
            },
            None => format!("unknown cvar: {name}"),
        }
    }

    fn set_bool(&mut self, name: &str, value: bool) {
        if let Some(cvar) = self.cvars.get_mut(name) {
            cvar.value = CVarValue::Bool(value);
        }
    }

    fn format_cvar(&self, name: &str) -> String {
        match self.cvars.get(name) {
            Some(cvar) => {
                let val = match cvar.value {
                    CVarValue::Bool(v) => format!("{}", v as u8),
                };
                format!("{name} = {val}  — {}", cvar.help)
            }
            None => format!("unknown cvar: {name}"),
        }
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_cvar_and_help() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("draw.grid"), Some(true));
        assert!(r.execute("grid off").contains("0"));
        assert_eq!(r.get_bool("draw.grid"), Some(false));
        assert!(r.execute("help").contains("draw.grid"));
        assert!(r.execute("help").contains("screenshot"));
        assert!(r.execute("help").contains("mp join"));
        assert_eq!(r.execute("screenshot"), "__REQUEST_SCREENSHOT__");
        assert_eq!(r.execute("mp join"), "__REQUEST_MP_JOIN__");
        assert_eq!(r.execute("mp leave"), "__REQUEST_MP_LEAVE__");
        assert_eq!(r.execute("mp"), "__REQUEST_MP_STATUS__");
        assert!(r.execute("draw.grid 1").contains("1"));
        assert_eq!(r.get_bool("draw.grid"), Some(true));
    }

    #[test]
    fn flycam_cvar_commands() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("cam.fly"), Some(false));
        assert!(r.execute("flycam on").contains("1"));
        assert_eq!(r.get_bool("cam.fly"), Some(true));
        assert!(r.execute("remount").contains("0"));
        assert_eq!(r.get_bool("cam.fly"), Some(false));
        assert!(r.execute("fly").contains("1")); // toggle
        assert_eq!(r.get_bool("cam.fly"), Some(true));
        assert!(r.execute("help").contains("cam.fly"));
        assert!(r.execute("help").contains("flycam"));
    }

    #[test]
    fn lineup_cvar_commands() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("draw.lineup"), Some(false));
        assert!(r.execute("lineup on").contains("1"));
        assert_eq!(r.get_bool("draw.lineup"), Some(true));
        assert!(r.execute("lineup off").contains("0"));
        assert!(r.execute("draw.lineup 1").contains("1"));
        assert!(r.execute("help").contains("draw.lineup"));
        assert!(r.execute("help").contains("lineup"));
    }

    #[test]
    fn nethud_cvar_commands() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("hud.net"), Some(true));
        assert!(r.execute("nethud off").contains("0"));
        assert_eq!(r.get_bool("hud.net"), Some(false));
        assert!(r.execute("nethud").contains("1"));
        assert_eq!(r.get_bool("hud.net"), Some(true));
        assert!(r.execute("help").contains("nethud"));
        assert!(r.execute("help").contains("hud.net"));
    }

    #[test]
    fn residualhud_cvar_commands() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("hud.residual"), Some(true));
        assert!(r.execute("residualhud off").contains("0"));
        assert_eq!(r.get_bool("hud.residual"), Some(false));
        assert!(r.execute("residualhud").contains("1"));
        assert_eq!(r.get_bool("hud.residual"), Some(true));
        assert!(r.execute("help").contains("residualhud"));
        assert!(r.execute("help").contains("hud.residual"));
    }

    #[test]
    fn lookhud_cvar_commands() {
        let mut r = DebugRegistry::new();
        r.register_defaults();
        assert_eq!(r.get_bool("hud.look"), Some(true));
        assert!(r.execute("lookhud off").contains("0"));
        assert_eq!(r.get_bool("hud.look"), Some(false));
        assert!(r.execute("lookhud").contains("1"));
        assert_eq!(r.get_bool("hud.look"), Some(true));
        assert!(r.execute("help").contains("lookhud"));
        assert!(r.execute("help").contains("hud.look"));
    }
}
