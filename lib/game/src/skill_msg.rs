//! `ZC_SKILLMSG`: a fixed set of skill notices, not a message-table id. The
//! server picks one of the Gospel results or the Full Strip failure.

pub const SKILL_MSG_COLOR: [f32; 4] = [1.0, 0.608, 0.608, 1.0];

pub fn skill_msg_line(msg_no: i32) -> Option<&'static str> {
    Some(match msg_no {
        0x15 => "All abnormal status effects have been removed.",
        0x16 => "You are immune to all abnormal status effects.",
        0x17 => "Max HP +100%.",
        0x18 => "Max SP +100%.",
        0x19 => "All stats +20.",
        0x1c => "Your weapon is enchanted with the Holy element.",
        0x1d => "Your armor is enchanted with the Holy element.",
        0x1e => "DEF +25%.",
        0x1f => "ATK +100%.",
        0x20 => "HIT and Flee +50.",
        0x28 => "The coating protected the equipment from Full Strip.",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_notices_resolve_and_the_rest_stay_silent() {
        assert_eq!(skill_msg_line(0x17), Some("Max HP +100%."));
        assert_eq!(
            skill_msg_line(0x28),
            Some("The coating protected the equipment from Full Strip.")
        );
        assert_eq!(skill_msg_line(0), None);
        assert_eq!(skill_msg_line(0x1a), None);
        assert_eq!(skill_msg_line(9999), None);
    }
}
