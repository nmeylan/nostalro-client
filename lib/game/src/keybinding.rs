use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HotkeyAction {
    ToggleInventory,
    ToggleEquipment,
    ToggleSkillTree,
    ToggleStatus,
    ToggleBasicInfo,
    ToggleShortcutList,
    ToggleEmotion,
    ToggleQuest,
    ToggleCart,
    ToggleGuild,
    ToggleChatRoomCreate,
    ToggleParty,
    ToggleFriends,
    ToggleHomunculus,
    ToggleMercenary,
    TogglePet,
    ToggleSoundOptions,
    ToggleGraphicOptions,
    SitStand,
    CycleMinimap,
    MercenaryFollow,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 21] = [
        HotkeyAction::ToggleInventory,
        HotkeyAction::ToggleEquipment,
        HotkeyAction::ToggleSkillTree,
        HotkeyAction::ToggleStatus,
        HotkeyAction::ToggleBasicInfo,
        HotkeyAction::ToggleShortcutList,
        HotkeyAction::ToggleEmotion,
        HotkeyAction::ToggleQuest,
        HotkeyAction::ToggleCart,
        HotkeyAction::ToggleGuild,
        HotkeyAction::ToggleChatRoomCreate,
        HotkeyAction::ToggleParty,
        HotkeyAction::ToggleFriends,
        HotkeyAction::ToggleHomunculus,
        HotkeyAction::ToggleMercenary,
        HotkeyAction::TogglePet,
        HotkeyAction::ToggleSoundOptions,
        HotkeyAction::ToggleGraphicOptions,
        HotkeyAction::SitStand,
        HotkeyAction::CycleMinimap,
        HotkeyAction::MercenaryFollow,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HotkeyAction::ToggleInventory => "Inventory",
            HotkeyAction::ToggleEquipment => "Equipment",
            HotkeyAction::ToggleSkillTree => "Skills",
            HotkeyAction::ToggleStatus => "Status",
            HotkeyAction::ToggleBasicInfo => "Basic Info",
            HotkeyAction::ToggleShortcutList => "Shortcuts",
            HotkeyAction::ToggleEmotion => "Emotion",
            HotkeyAction::ToggleQuest => "Quest",
            HotkeyAction::ToggleCart => "Cart",
            HotkeyAction::ToggleGuild => "Guild",
            HotkeyAction::ToggleChatRoomCreate => "Chat Room",
            HotkeyAction::ToggleParty => "Party",
            HotkeyAction::ToggleFriends => "Friends",
            HotkeyAction::ToggleHomunculus => "Homunculus",
            HotkeyAction::ToggleMercenary => "Mercenary",
            HotkeyAction::TogglePet => "Pet",
            HotkeyAction::ToggleSoundOptions => "Sound",
            HotkeyAction::ToggleGraphicOptions => "Graphics",
            HotkeyAction::SitStand => "Sit / Stand",
            HotkeyAction::CycleMinimap => "Minimap",
            HotkeyAction::MercenaryFollow => "Merc. Follow",
        }
    }
}

/// A key plus its modifier state. `key` is the winit `KeyCode` debug name
/// (`"KeyE"`, `"Insert"`, `"Tab"`); the client produces it with
/// `format!("{code:?}")`, keeping this crate free of a winit dependency.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyChord {
    pub key: String,
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
}

impl KeyChord {
    pub fn new(key: impl Into<String>, alt: bool, ctrl: bool, shift: bool) -> Self {
        Self {
            key: key.into(),
            alt,
            ctrl,
            shift,
        }
    }

    pub fn display(&self) -> String {
        let mut s = String::new();
        if self.ctrl {
            s.push_str("Ctrl + ");
        }
        if self.alt {
            s.push_str("Alt + ");
        }
        if self.shift {
            s.push_str("Shift + ");
        }
        s.push_str(&display_key(&self.key));
        s
    }

    /// Keys that must never be rebound to an Interface action: they drive chat
    /// typing, battle-mode letter rows, text-input navigation, or debug overlays.
    pub fn is_reserved(&self) -> bool {
        if is_function_key(&self.key) || matches!(self.key.as_str(), "Enter" | "NumpadEnter" | "Escape")
        {
            return true;
        }
        !self.alt && !self.ctrl && !self.shift && is_printable_key(&self.key)
    }

    /// Keys that must never be bound to a skill-bar slot or emotion (they are
    /// consumed by chat/text input). Function keys and bare printables ARE
    /// allowed here — F1..F9 are the skill bar's own defaults.
    pub fn is_reserved_trigger(&self) -> bool {
        matches!(self.key.as_str(), "Enter" | "NumpadEnter" | "Escape")
    }
}

fn display_key(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("Key") {
        rest.to_string()
    } else if let Some(rest) = key.strip_prefix("Digit") {
        rest.to_string()
    } else if let Some(rest) = key.strip_prefix("Numpad") {
        format!("Num {rest}")
    } else {
        key.to_string()
    }
}

fn is_function_key(key: &str) -> bool {
    key.strip_prefix('F')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

fn is_printable_key(key: &str) -> bool {
    key.starts_with("Key")
        || key.starts_with("Digit")
        || key.starts_with("Numpad")
        || matches!(
            key,
            "Backquote"
                | "Minus"
                | "Equal"
                | "BracketLeft"
                | "BracketRight"
                | "Backslash"
                | "Semicolon"
                | "Quote"
                | "Comma"
                | "Period"
                | "Slash"
                | "Space"
                | "IntlBackslash"
                | "IntlRo"
                | "IntlYen"
        )
}

pub fn is_modifier_key(key: &str) -> bool {
    matches!(
        key,
        "AltLeft"
            | "AltRight"
            | "ControlLeft"
            | "ControlRight"
            | "ShiftLeft"
            | "ShiftRight"
            | "SuperLeft"
            | "SuperRight"
            | "Meta"
            | "MetaLeft"
            | "MetaRight"
            | "Fn"
            | "FnLock"
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyBindings {
    map: HashMap<HotkeyAction, KeyChord>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::defaults()
    }
}

impl KeyBindings {
    pub fn defaults() -> Self {
        let alt = |k: &str| KeyChord::new(k, true, false, false);
        let ctrl = |k: &str| KeyChord::new(k, false, true, false);
        let plain = |k: &str| KeyChord::new(k, false, false, false);
        let map = HashMap::from([
            (HotkeyAction::ToggleInventory, alt("KeyE")),
            (HotkeyAction::ToggleEquipment, alt("KeyQ")),
            (HotkeyAction::ToggleSkillTree, alt("KeyS")),
            (HotkeyAction::ToggleStatus, alt("KeyA")),
            (HotkeyAction::ToggleBasicInfo, alt("KeyV")),
            (HotkeyAction::ToggleShortcutList, alt("KeyM")),
            (HotkeyAction::ToggleEmotion, alt("KeyL")),
            (HotkeyAction::ToggleQuest, alt("KeyU")),
            (HotkeyAction::ToggleCart, alt("KeyW")),
            (HotkeyAction::ToggleGuild, alt("KeyG")),
            (HotkeyAction::ToggleChatRoomCreate, alt("KeyC")),
            (HotkeyAction::ToggleParty, alt("KeyZ")),
            (HotkeyAction::ToggleFriends, alt("KeyH")),
            (HotkeyAction::ToggleHomunculus, alt("KeyR")),
            (HotkeyAction::ToggleMercenary, ctrl("KeyR")),
            (HotkeyAction::TogglePet, alt("KeyJ")),
            (HotkeyAction::ToggleSoundOptions, alt("KeyO")),
            (HotkeyAction::ToggleGraphicOptions, alt("KeyD")),
            (HotkeyAction::SitStand, plain("Insert")),
            (HotkeyAction::CycleMinimap, ctrl("Tab")),
            (HotkeyAction::MercenaryFollow, ctrl("KeyT")),
        ]);
        Self { map }
    }

    pub fn map(&self) -> &HashMap<HotkeyAction, KeyChord> {
        &self.map
    }

    pub fn get(&self, action: HotkeyAction) -> Option<&KeyChord> {
        self.map.get(&action)
    }

    pub fn set(&mut self, action: HotkeyAction, chord: KeyChord) {
        self.map.insert(action, chord);
    }

    pub fn action_for(&self, chord: &KeyChord) -> Option<HotkeyAction> {
        self.map.iter().find(|(_, c)| *c == chord).map(|(a, _)| *a)
    }

    pub fn conflict(&self, chord: &KeyChord, exclude: HotkeyAction) -> Option<HotkeyAction> {
        self.map
            .iter()
            .find(|(a, c)| **a != exclude && *c == chord)
            .map(|(a, _)| *a)
    }

    pub fn fill_missing_from_defaults(&mut self) {
        for (action, chord) in Self::defaults().map {
            self.map.entry(action).or_insert(chord);
        }
    }
}

/// Trigger keys for emotes, keyed by `emote_type`. Empty by default (every
/// emote starts undesignated).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmotionKeys {
    map: HashMap<u8, KeyChord>,
}

impl EmotionKeys {
    pub fn get(&self, emote_type: u8) -> Option<&KeyChord> {
        self.map.get(&emote_type)
    }

    pub fn set(&mut self, emote_type: u8, chord: KeyChord) {
        self.map.insert(emote_type, chord);
    }

    pub fn emote_for(&self, chord: &KeyChord) -> Option<u8> {
        self.map.iter().find(|(_, c)| *c == chord).map(|(e, _)| *e)
    }

    pub fn conflict(&self, chord: &KeyChord, exclude: u8) -> Option<u8> {
        self.map
            .iter()
            .find(|(e, c)| **e != exclude && *c == chord)
            .map(|(e, _)| *e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_lookup_conflict_and_reserved() {
        let bindings = KeyBindings::defaults();
        assert_eq!(
            bindings.get(HotkeyAction::ToggleInventory),
            Some(&KeyChord::new("KeyE", true, false, false))
        );
        assert_eq!(
            bindings.get(HotkeyAction::SitStand),
            Some(&KeyChord::new("Insert", false, false, false))
        );

        let alt_e = KeyChord::new("KeyE", true, false, false);
        assert_eq!(bindings.action_for(&alt_e), Some(HotkeyAction::ToggleInventory));

        let alt_g = KeyChord::new("KeyG", true, false, false);
        assert_eq!(
            bindings.conflict(&alt_g, HotkeyAction::ToggleInventory),
            Some(HotkeyAction::ToggleGuild)
        );

        assert!(KeyChord::new("F1", false, false, false).is_reserved());
        assert!(KeyChord::new("KeyA", false, false, false).is_reserved());
        assert!(!KeyChord::new("Insert", false, false, false).is_reserved());
    }
}
