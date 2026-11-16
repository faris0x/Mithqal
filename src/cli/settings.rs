//! REPL settings.

#[derive(Copy, Clone, Debug)]
pub(super) struct Settings {
    pub timing: bool,
    pub pretty: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            timing: false,
            pretty: true,
        }
    }
}

pub(super) fn show(settings: &Settings) {
    println!(
        "settings:\n  timing {}\n  pretty {}",
        on_off(settings.timing),
        on_off(settings.pretty)
    );
}

pub(super) fn set(rest: &str, settings: &mut Settings) -> bool {
    let mut parts = rest.split_whitespace();
    let name = parts.next().unwrap_or("");
    let value = parts.next().unwrap_or("");
    if parts.next().is_some() {
        eprintln!("error: usage: set NAME on|off");
        return false;
    }
    let enabled = match value {
        "on" | "true" => true,
        "off" | "false" => false,
        _ => {
            eprintln!("error: setting value must be on or off");
            return false;
        }
    };
    match name {
        "timing" => settings.timing = enabled,
        "pretty" => settings.pretty = enabled,
        "" => {
            show(settings);
            return true;
        }
        _ => {
            eprintln!("error: unknown setting {name}");
            return false;
        }
    }
    println!("{name}: {}", on_off(enabled));
    true
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}
