use models::enums::skill_enums::SkillEnum;
use ragnarok_effects::merc_skill_base;
use ragnarok_formats::act::SpriteActionType;

/// Who owns a skill, derived purely from its id. Mercenary and homunculus skills
/// occupy fixed, disjoint id ranges, so the caster is known without any runtime
/// companion state — this must hold even for hotkeys restored at login before a
/// companion exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillCaster {
    Player,
    Mercenary,
    Homunculus,
}

pub fn skill_caster(skill: SkillEnum) -> SkillCaster {
    let id = skill.id();
    if (SkillEnum::HlifHeal.id()..=SkillEnum::MhVolcanicAsh.id()).contains(&id) {
        SkillCaster::Homunculus
    } else if (SkillEnum::MsBash.id()..=SkillEnum::MerInvincibleoff2.id()).contains(&id) {
        SkillCaster::Mercenary
    } else {
        SkillCaster::Player
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillMotionType {
    Attack,
    Throw,
    Attack2,
    Pickup,
    Sing,
    Dance,
    Stand,
    Walk,
    Skill,
    /// Freezes on a single frame, see [`skill_pose`].
    Pose,
}

/// A stance: one frame of one action group, held still for `hold_secs`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkillPose {
    pub action: usize,
    pub frame: usize,
    pub hold_secs: f32,
}

const STANCE_HOLD_SECS: f32 = 2.0;

const fn stance(action: SpriteActionType, frame: usize) -> SkillPose {
    SkillPose {
        action: action as usize,
        frame,
        hold_secs: STANCE_HOLD_SECS,
    }
}

/// The taekwon kick stances, which pose the body instead of animating it.
pub fn skill_pose(skill: SkillEnum) -> Option<SkillPose> {
    use SkillEnum as S;
    use SpriteActionType::{Pickup, Skill};

    match merc_skill_base(skill) {
        S::TkReadystorm => Some(stance(Skill, 0)),
        S::TkReadydown => Some(stance(Skill, 2)),
        S::TkReadyturn => Some(stance(Skill, 3)),
        S::TkReadycounter => Some(stance(Skill, 4)),
        S::TkDodge => Some(stance(Pickup, 1)),
        _ => None,
    }
}

pub fn skill_motion_type(skill: SkillEnum) -> SkillMotionType {
    use SkillEnum as S;
    use SkillMotionType::*;

    let skill = merc_skill_base(skill);
    if skill_pose(skill).is_some() {
        return Pose;
    }
    match skill {
        S::SmBash
        | S::SmMagnum
        | S::McMammonite
        | S::AcDouble
        | S::AcShower
        | S::AcChargearrow
        | S::KnPierce
        | S::KnBrandishspear
        | S::KnSpearstab
        | S::KnBowlingbash
        | S::KnAutocounter
        | S::KnChargeatk
        | S::BsSkintemper
        | S::BsHammerfall
        | S::HtPower
        | S::HtPhantasmic
        | S::CrHolycross
        | S::RgBackstap
        | S::RgRaid
        | S::RgIntimidate
        | S::RgCloseconfine
        | S::AsSonicblow
        | S::MoInvestigate
        | S::MoFingeroffensive
        | S::MoTripleattack
        | S::PaPressure
        | S::PaSacrifice
        | S::ChPalmstrike
        | S::ChChaincrush
        | S::AscBreaker
        | S::AscMeteorassault
        | S::HwMagicpower
        | S::SnSharpshooting
        | S::LkSpiralpierce
        | S::LkHeadcrush
        | S::LkJointbeat => Attack,

        S::BaMusicalstrike | S::DcThrowarrow | S::CgArrowvulcan => Attack2,

        S::KnSpearboomerang
        | S::CrShieldcharge
        | S::CrShieldboomerang
        | S::PaShieldchain
        | S::AmPotionpitcher
        | S::AmAcidterror
        | S::AmDemonstration
        | S::AmCannibalize
        | S::TfThrowstone
        | S::TfSprinklesand
        | S::AsVenomknife => Throw,

        S::HtSkidtrap
        | S::HtLandmine
        | S::HtAnklesnare
        | S::HtShockwave
        | S::HtSandman
        | S::HtFlasher
        | S::HtFreezingtrap
        | S::HtBlastmine
        | S::HtClaymoretrap
        | S::HtRemovetrap
        | S::HtTalkiebox
        | S::BsGreed => Pickup,

        S::BaAppleidun | S::BaDissonance | S::BaWhistle | S::BaAssassincross | S::BaPoembragi => {
            Sing
        }

        S::DcWinkcharm
        | S::DcFortunekiss
        | S::DcUglydance
        | S::DcHumming
        | S::DcDontforgetme
        | S::DcServiceforyou
        | S::BdLullaby
        | S::BdRichmankim
        | S::BdEternalchaos
        | S::BdDrumbattlefield
        | S::BdSiegfried
        | S::CgHermode
        | S::BdRingnibelungen
        | S::BdRokisweil
        | S::BdIntoabyss
        | S::CgMoonlit
        | S::CgMarionette => Dance,

        S::AlIncagi
        | S::CrAutoguard
        | S::CrReflectshield
        | S::CrDefender
        | S::MoSteelbody
        | S::MoBladestop
        | S::BdAdaptation
        | S::LkParrying
        | S::PaGospel
        | S::SnSight
        | S::WsMeltdown
        | S::WsCartboost
        | S::ChSoulcollect => Stand,

        S::TkRun => Walk,

        _ => Skill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_skills_return_attack() {
        assert_eq!(
            skill_motion_type(SkillEnum::SmBash),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::AcDouble),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::AsSonicblow),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::LkSpiralpierce),
            SkillMotionType::Attack
        );
    }

    #[test]
    fn bard_musical_strike_returns_attack2() {
        assert_eq!(
            skill_motion_type(SkillEnum::BaMusicalstrike),
            SkillMotionType::Attack2
        );
        assert_eq!(
            skill_motion_type(SkillEnum::CgArrowvulcan),
            SkillMotionType::Attack2
        );
    }

    #[test]
    fn throw_skills_return_throw() {
        assert_eq!(
            skill_motion_type(SkillEnum::KnSpearboomerang),
            SkillMotionType::Throw
        );
        assert_eq!(
            skill_motion_type(SkillEnum::TfThrowstone),
            SkillMotionType::Throw
        );
    }

    #[test]
    fn trap_skills_return_pickup() {
        assert_eq!(
            skill_motion_type(SkillEnum::HtLandmine),
            SkillMotionType::Pickup
        );
        assert_eq!(
            skill_motion_type(SkillEnum::HtAnklesnare),
            SkillMotionType::Pickup
        );
    }

    #[test]
    fn bard_songs_return_sing() {
        assert_eq!(
            skill_motion_type(SkillEnum::BaPoembragi),
            SkillMotionType::Sing
        );
        assert_eq!(
            skill_motion_type(SkillEnum::BaAppleidun),
            SkillMotionType::Sing
        );
    }

    #[test]
    fn dancer_skills_return_dance() {
        assert_eq!(
            skill_motion_type(SkillEnum::DcFortunekiss),
            SkillMotionType::Dance
        );
        assert_eq!(
            skill_motion_type(SkillEnum::BdLullaby),
            SkillMotionType::Dance
        );
    }

    #[test]
    fn stand_skills_return_stand() {
        assert_eq!(
            skill_motion_type(SkillEnum::CrAutoguard),
            SkillMotionType::Stand
        );
        assert_eq!(
            skill_motion_type(SkillEnum::LkParrying),
            SkillMotionType::Stand
        );
    }

    #[test]
    fn taekwon_stances_hold_a_pose_and_running_walks() {
        for (s, action, frame) in [
            (SkillEnum::TkReadystorm, 12, 0),
            (SkillEnum::TkReadydown, 12, 2),
            (SkillEnum::TkReadyturn, 12, 3),
            (SkillEnum::TkReadycounter, 12, 4),
            (SkillEnum::TkDodge, 3, 1),
        ] {
            assert_eq!(skill_motion_type(s), SkillMotionType::Pose, "{s:?}");
            let pose = skill_pose(s).expect("{s:?} poses");
            assert_eq!((pose.action, pose.frame), (action, frame), "{s:?}");
            assert_eq!(pose.hold_secs, 2.0, "{s:?}");
        }
        // Running plays the walk motion, not an idle stand or a cast.
        assert_eq!(skill_motion_type(SkillEnum::TkRun), SkillMotionType::Walk);
        assert!(skill_pose(SkillEnum::TkRun).is_none());
    }

    #[test]
    fn mercenary_bow_skills_animate_as_ranged_attack() {
        for s in [
            SkillEnum::MaDouble,
            SkillEnum::MaShower,
            SkillEnum::MaChargearrow,
        ] {
            assert_eq!(
                skill_motion_type(s),
                SkillMotionType::Attack,
                "{s:?} should animate as a ranged attack"
            );
        }
        assert_eq!(
            skill_motion_type(SkillEnum::MerQuicken),
            SkillMotionType::Skill
        );
    }

    #[test]
    fn heal_defaults_to_skill() {
        assert_eq!(skill_motion_type(SkillEnum::AlHeal), SkillMotionType::Skill);
    }
}
