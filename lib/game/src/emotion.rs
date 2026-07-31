use crate::entity::EmotionState;
use ragnarok_formats::act::ActFile;

/// Frame delay for an action whose ACT delay is missing or zero.
const DEFAULT_FRAME_MS: f32 = 150.0;

pub struct EmotionEntry {
    pub emote_type: u8,
    pub sprite_action: usize,
    pub command: &'static str,
}

/// Selectable emoticons in display order. `emote_type` is the value broadcast to
/// the server; `sprite_action` is its action index in `emotion.act` (the two
/// differ for many emotes); `command` is the primary chat shortcut (typed as
/// `/<command>`). Flags, dice and quest markers are excluded.
pub const EMOTION_TABLE: &[EmotionEntry] = &[
    e(0, 0, "!"),
    e(1, 1, "?"),
    e(2, 2, "ho"),
    e(3, 3, "lv"),
    e(14, 4, "lv2"),
    e(4, 5, "swt"),
    e(5, 6, "ic"),
    e(6, 7, "an"),
    e(7, 8, "ag"),
    e(8, 9, "$"),
    e(9, 10, "..."),
    e(11, 11, "rock"),
    e(10, 12, "scissors"),
    e(12, 13, "paper"),
    e(15, 15, "thx"),
    e(16, 16, "wah"),
    e(17, 17, "sry"),
    e(18, 18, "heh"),
    e(19, 19, "swt2"),
    e(20, 20, "hmm"),
    e(21, 21, "no1"),
    e(22, 22, "??"),
    e(23, 23, "omg"),
    e(24, 24, "oh"),
    e(25, 25, "X"),
    e(26, 26, "hlp"),
    e(27, 27, "go"),
    e(28, 28, "sob"),
    e(29, 29, "gg"),
    e(30, 30, "kis"),
    e(31, 31, "kis2"),
    e(32, 32, "pif"),
    e(33, 33, "ok"),
    e(36, 35, "bzz"),
    e(37, 36, "rice"),
    e(38, 37, "awsm"),
    e(39, 38, "meh"),
    e(40, 39, "shy"),
    e(41, 40, "pat"),
    e(42, 41, "mp"),
    e(43, 42, "slur"),
    e(44, 43, "com"),
    e(45, 44, "yawn"),
    e(46, 45, "grat"),
    e(47, 46, "hp"),
    e(52, 51, "fsh"),
    e(53, 52, "spin"),
    e(54, 53, "sigh"),
    e(55, 54, "dum"),
    e(56, 55, "crwd"),
    e(57, 56, "desp"),
    e(65, 64, "love"),
    e(68, 67, "mobile"),
    e(69, 68, "mail"),
    e(71, 70, "antenna1"),
    e(72, 71, "antenna2"),
    e(73, 72, "antenna3"),
    e(74, 73, "hum"),
    e(75, 74, "abs"),
    e(76, 75, "oops"),
    e(77, 76, "spit"),
    e(78, 77, "ene"),
    e(79, 78, "panic"),
    e(80, 79, "whisp"),
];

const fn e(emote_type: u8, sprite_action: usize, command: &'static str) -> EmotionEntry {
    EmotionEntry {
        emote_type,
        sprite_action,
        command,
    }
}

pub fn emote_sprite_action(emote_type: u8) -> usize {
    EMOTION_TABLE
        .iter()
        .find(|e| e.emote_type == emote_type)
        .map(|e| e.sprite_action)
        .unwrap_or(emote_type as usize)
}

/// Default Alt+1..Alt+0 bindings for the shortcut list: the first ten emotes by
/// `emote_type` (surprise..think), each as a `/command` string.
pub fn default_shortcut_commands() -> Vec<String> {
    (0u8..=9)
        .map(|emote_type| {
            let cmd = EMOTION_TABLE
                .iter()
                .find(|e| e.emote_type == emote_type)
                .map(|e| e.command)
                .unwrap_or("");
            format!("/{cmd}")
        })
        .collect()
}

/// Resolve a chat command (with or without the leading `/`) to the emote it
/// triggers, e.g. `/lv` or `lv` -> the throb emote.
pub fn emote_type_for_command(command: &str) -> Option<u8> {
    let command = command.strip_prefix('/').unwrap_or(command);
    EMOTION_TABLE
        .iter()
        .find(|e| e.command.eq_ignore_ascii_case(command))
        .map(|e| e.emote_type)
}

/// `(action index, frame delay in ms, frame count)` for an emote, or `None` when
/// `emotion.act` holds no usable action for it. Shared by the expiry clock and
/// the draw so the balloon cannot outlive or outrun its own animation.
pub fn emote_timing(act: &ActFile, emote_type: u8) -> Option<(usize, f32, usize)> {
    let action_idx = emote_sprite_action(emote_type);
    let frames = act.actions.get(action_idx)?.motions.len();
    if frames == 0 {
        return None;
    }
    let delay_ms = act
        .delays
        .get(action_idx)
        .map(|d| d * 25.0)
        .filter(|d| *d > 0.0)
        .unwrap_or(DEFAULT_FRAME_MS);
    Some((action_idx, delay_ms, frames))
}

/// How long an emote balloon shows: one pass of its action at the action's own
/// frame delay.
pub fn emote_duration(act: Option<&ActFile>, emote_type: u8) -> f32 {
    act.and_then(|act| emote_timing(act, emote_type))
        .map(|(_, delay_ms, frames)| frames as f32 * delay_ms / 1000.0)
        .unwrap_or(EmotionState::FALLBACK_DURATION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_formats::act::{Action, Motion};

    fn act_with(actions: &[(usize, f32)]) -> ActFile {
        let motion = || Motion {
            range1: [0; 4],
            range2: [0; 4],
            clips: Vec::new(),
            event_id: -1,
            attach_points: Vec::new(),
        };
        ActFile {
            version: (2, 5),
            events: Vec::new(),
            actions: actions
                .iter()
                .map(|&(frames, _)| Action {
                    motions: (0..frames).map(|_| motion()).collect(),
                })
                .collect(),
            delays: actions.iter().map(|&(_, delay)| delay).collect(),
        }
    }

    #[test]
    fn emote_duration_is_one_pass_of_its_own_action() {
        // Emote 4 draws action 5: 28 frames at a delay of 2 (50 ms) = 1.4 s.
        let act = act_with(&[(1, 2.0), (1, 2.0), (1, 2.0), (1, 2.0), (1, 2.0), (28, 2.0)]);
        assert!((emote_duration(Some(&act), 4) - 1.4).abs() < 1e-4);

        assert_eq!(
            emote_duration(None, 4),
            EmotionState::FALLBACK_DURATION,
            "no act loaded"
        );
        assert_eq!(
            emote_duration(Some(&act), 200),
            EmotionState::FALLBACK_DURATION,
            "action out of range"
        );
        assert_eq!(
            emote_duration(Some(&act_with(&[(0, 2.0)])), 0),
            EmotionState::FALLBACK_DURATION,
            "empty action"
        );
    }

    #[test]
    fn maps_mismatched_emote_to_sprite_and_falls_back_to_identity() {
        assert_eq!(emote_sprite_action(4), 5);
        assert_eq!(emote_sprite_action(14), 4);
        assert_eq!(emote_sprite_action(0), 0);
        assert_eq!(emote_sprite_action(200), 200);
    }

    #[test]
    fn resolves_chat_command_to_emote() {
        assert_eq!(emote_type_for_command("/lv"), Some(3));
        assert_eq!(emote_type_for_command("swt"), Some(4));
        assert_eq!(emote_type_for_command("/X"), Some(25));
        assert_eq!(emote_type_for_command("/nope"), None);
    }

    #[test]
    fn default_shortcuts_are_first_ten_emotes() {
        let d = default_shortcut_commands();
        assert_eq!(d.len(), 10);
        assert_eq!(d[0], "/!");
        assert_eq!(d[3], "/lv");
        assert_eq!(d[9], "/...");
    }
}
