//! Convex Mirror boss tracking: `ZC_BOSS_INFO` reports whether the map's MVP is
//! alive, where it stands, or how long until it respawns.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossInfoKind {
    NotOnMap,
    Alive,
    AliveAnnounced,
    Dead,
}

impl BossInfoKind {
    pub fn from_packet(info_type: u8) -> Option<Self> {
        match info_type {
            0 => Some(BossInfoKind::NotOnMap),
            1 => Some(BossInfoKind::Alive),
            2 => Some(BossInfoKind::AliveAnnounced),
            3 => Some(BossInfoKind::Dead),
            _ => None,
        }
    }
}

/// Where the tracked boss stands, for the minimap marker.
#[derive(Debug, Clone)]
pub struct BossMark {
    pub x: u16,
    pub y: u16,
    pub name: String,
}

/// The chat line a report is worth, if any: a position refresh is silent, so
/// only the first sighting, a death and an empty map say anything.
pub fn boss_info_line(kind: BossInfoKind, name: &str, hour: u16, minute: u16) -> Option<String> {
    match kind {
        BossInfoKind::Alive => None,
        BossInfoKind::NotOnMap => Some("There is no MVP monster on this map.".to_string()),
        BossInfoKind::AliveAnnounced => Some(format!("{name} is on this map.")),
        BossInfoKind::Dead => Some(match (hour, minute) {
            (0, 0) => format!("{name} will respawn shortly."),
            (0, m) => format!("{name} will respawn in {m} minute(s)."),
            (h, m) => format!("{name} will respawn in {h} hour(s) and {m} minute(s)."),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_report_says_its_piece_and_a_position_refresh_says_nothing() {
        assert_eq!(
            boss_info_line(BossInfoKind::AliveAnnounced, "Baphomet", 0, 0).as_deref(),
            Some("Baphomet is on this map.")
        );
        assert_eq!(boss_info_line(BossInfoKind::Alive, "Baphomet", 0, 0), None);
        assert_eq!(
            boss_info_line(BossInfoKind::Dead, "Baphomet", 1, 5).as_deref(),
            Some("Baphomet will respawn in 1 hour(s) and 5 minute(s).")
        );
        assert_eq!(
            boss_info_line(BossInfoKind::Dead, "Baphomet", 0, 12).as_deref(),
            Some("Baphomet will respawn in 12 minute(s).")
        );
        assert!(
            boss_info_line(BossInfoKind::NotOnMap, "", 0, 0).is_some_and(|l| l.contains("no MVP"))
        );
    }
}
