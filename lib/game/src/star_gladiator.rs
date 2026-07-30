use models::enums::effect_id::EffectId;

/// Prompt shown before a Feeling place is written, asking to confirm that the
/// designation cannot be changed afterwards.
pub const FEEL_PLACE_CONFIRM_MSG: u16 = 1028;

/// Which name the message takes, and in which order relative to the character
/// name and the progress percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarSubject {
    /// (character name, feel map)
    FeelPlace,
    /// (character name, hate monster)
    HateMonster,
    /// (mission monster, progress percent)
    MissionProgress,
    /// (mission monster)
    Mission,
    /// (collector item)
    MissionItem,
    /// no arguments
    Nothing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarNotice {
    pub msg_id: u16,
    pub subject: StarSubject,
    pub effect: Option<EffectId>,
    pub chime: bool,
}

/// Target HP readout, the one ZC_STARSKILL notice with no msgstringtable entry.
pub const TARGET_HP_RESULT: u8 = 40;

pub fn star_notice(result: u8, star: u8) -> Option<StarNotice> {
    let by_star = |first: u16| -> Option<u16> {
        if star < 3 {
            Some(first + star as u16)
        } else {
            None
        }
    };
    let notice = |msg_id: u16, subject: StarSubject, effect, chime| {
        Some(StarNotice {
            msg_id,
            subject,
            effect,
            chime,
        })
    };
    match result {
        0 => notice(by_star(837)?, StarSubject::FeelPlace, None, false),
        1 => notice(by_star(840)?, StarSubject::FeelPlace, None, true),
        10 => notice(by_star(843)?, StarSubject::HateMonster, None, false),
        11 => notice(
            by_star(846)?,
            StarSubject::HateMonster,
            Some(EffectId::Hated),
            true,
        ),
        20 => notice(
            927,
            StarSubject::MissionProgress,
            Some(EffectId::Hated),
            false,
        ),
        21 => notice(1190, StarSubject::Mission, Some(EffectId::Hated), false),
        22 => notice(1265, StarSubject::MissionItem, Some(EffectId::Hated), false),
        30 => notice(1033, StarSubject::Nothing, Some(EffectId::Electric), false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use StarSubject as S;

    const HATE: Option<EffectId> = Some(EffectId::Hated);
    const ELEC: Option<EffectId> = Some(EffectId::Electric);

    #[test]
    fn every_result_code_maps_to_its_message() {
        let rows = [
            (0, 0, 837, S::FeelPlace, None, false),
            (0, 2, 839, S::FeelPlace, None, false),
            (1, 1, 841, S::FeelPlace, None, true),
            (10, 2, 845, S::HateMonster, None, false),
            (11, 0, 846, S::HateMonster, HATE, true),
            (20, 40, 927, S::MissionProgress, HATE, false),
            (21, 0, 1190, S::Mission, HATE, false),
            (22, 0, 1265, S::MissionItem, HATE, false),
            (30, 0, 1033, S::Nothing, ELEC, false),
        ];
        for (result, star, msg_id, subject, effect, chime) in rows {
            assert_eq!(
                star_notice(result, star),
                Some(StarNotice {
                    msg_id,
                    subject,
                    effect,
                    chime
                }),
                "result {result} star {star}"
            );
        }

        assert_eq!(star_notice(TARGET_HP_RESULT, 0), None);
        assert_eq!(star_notice(0, 3), None);
        assert_eq!(star_notice(7, 0), None);
    }
}
