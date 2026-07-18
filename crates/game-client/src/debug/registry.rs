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
            "remount" => {
                self.set_bool("cam.fly", false);
                "cam.fly = 0 (remount)".into()
            }
            // Handled by DebugTools (needs canvas); listed here for help/discovery.
            "screenshot" | "shot" => "__REQUEST_SCREENSHOT__".into(),
            name if self.cvars.contains_key(name) => self.cmd_cvar(name, &args),
            _ => format!("unknown: '{head}'. type 'help'."),
        }
    }

    fn cmd_help(&self) -> String {
        let mut lines = vec![
            "commands:".into(),
            "  help              this list".into(),
            "  grid [on|off|toggle]  set draw.grid".into(),
            "  flycam|fly [on|off|toggle]  debug flycam (F8)".into(),
            "  remount           leave flycam, restore self view".into(),
            "  screenshot|shot   capture frame (F9)".into(),
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
        assert_eq!(r.execute("screenshot"), "__REQUEST_SCREENSHOT__");
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
}
