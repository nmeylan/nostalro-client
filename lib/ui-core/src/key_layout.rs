#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "linux")]
mod x11;
#[cfg(target_os = "linux")]
mod xkb;
#[cfg(target_os = "windows")]
mod windows;

use std::collections::HashMap;
use std::sync::OnceLock;
use winit::keyboard::KeyCode;

/// What the current keyboard layout prints on a physical key. Resolved from the
/// OS once per process; empty when the platform has no resolver yet, in which
/// case `display` falls back to the US name of the position.
#[derive(Clone, Debug, Default)]
pub struct KeyLabels {
    map: HashMap<KeyCode, String>,
}

static RESOLVED: OnceLock<HashMap<KeyCode, String>> = OnceLock::new();

impl KeyLabels {
    pub fn resolve() -> Self {
        Self {
            map: RESOLVED.get_or_init(resolve_from_os).clone(),
        }
    }

    pub fn from_map(map: HashMap<KeyCode, String>) -> Self {
        Self { map }
    }

    pub fn label(&self, code: KeyCode) -> Option<&str> {
        self.map.get(&code).map(|s| s.as_str())
    }

    pub fn display(&self, code: KeyCode) -> String {
        match self.label(code) {
            Some(label) => label.to_uppercase(),
            None => us_name(code),
        }
    }
}

#[cfg(target_os = "linux")]
fn resolve_from_os() -> HashMap<KeyCode, String> {
    wayland::keymap_string()
        .and_then(|keymap| xkb::labels_from_keymap_string(&keymap))
        .or_else(|| x11::rule_names().and_then(|names| xkb::labels_from_rule_names(&names)))
        .unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn resolve_from_os() -> HashMap<KeyCode, String> {
    windows::labels()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn resolve_from_os() -> HashMap<KeyCode, String> {
    HashMap::new()
}

fn us_name(code: KeyCode) -> String {
    let name = format!("{code:?}");
    if let Some(rest) = name.strip_prefix("Key") {
        rest.to_string()
    } else if let Some(rest) = name.strip_prefix("Digit") {
        rest.to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_layout_wins_over_the_us_name() {
        let labels = KeyLabels::from_map(HashMap::from([
            (KeyCode::KeyQ, "a".to_string()),
            (KeyCode::Digit1, "&".to_string()),
        ]));
        assert_eq!(labels.display(KeyCode::KeyQ), "A");
        assert_eq!(labels.display(KeyCode::Digit1), "&");
        assert_eq!(labels.display(KeyCode::KeyW), "W");
        assert_eq!(labels.display(KeyCode::F1), "F1");
    }
}
