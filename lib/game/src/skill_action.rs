use models::enums::skill_enums::SkillEnum;
use ragnarok_effects::merc_skill_base_id;
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

pub fn skill_caster(skill_id: u16) -> SkillCaster {
    let id = skill_id as u32;
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
pub fn skill_pose(skill_id: u16) -> Option<SkillPose> {
    let id = merc_skill_base_id(skill_id);
    use SpriteActionType::{Pickup, Skill};

    if id == SkillEnum::TkReadystorm.id() as u16 {
        Some(stance(Skill, 0))
    } else if id == SkillEnum::TkReadydown.id() as u16 {
        Some(stance(Skill, 2))
    } else if id == SkillEnum::TkReadyturn.id() as u16 {
        Some(stance(Skill, 3))
    } else if id == SkillEnum::TkReadycounter.id() as u16 {
        Some(stance(Skill, 4))
    } else if id == SkillEnum::TkDodge.id() as u16 {
        Some(stance(Pickup, 1))
    } else {
        None
    }
}

pub fn skill_motion_type(skill_id: u16) -> SkillMotionType {
    use SkillMotionType::*;

    let id = merc_skill_base_id(skill_id);
    if skill_pose(id).is_some() {
        return Pose;
    }
    if id == SkillEnum::SmBash.id() as u16
        || id == SkillEnum::SmMagnum.id() as u16
        || id == SkillEnum::McMammonite.id() as u16
        || id == SkillEnum::AcDouble.id() as u16
        || id == SkillEnum::AcShower.id() as u16
        || id == SkillEnum::AcChargearrow.id() as u16
        || id == SkillEnum::KnPierce.id() as u16
        || id == SkillEnum::KnBrandishspear.id() as u16
        || id == SkillEnum::KnSpearstab.id() as u16
        || id == SkillEnum::KnBowlingbash.id() as u16
        || id == SkillEnum::KnAutocounter.id() as u16
        || id == SkillEnum::KnChargeatk.id() as u16
        || id == SkillEnum::BsSkintemper.id() as u16
        || id == SkillEnum::BsHammerfall.id() as u16
        || id == SkillEnum::HtPower.id() as u16
        || id == SkillEnum::HtPhantasmic.id() as u16
        || id == SkillEnum::CrHolycross.id() as u16
        || id == SkillEnum::RgBackstap.id() as u16
        || id == SkillEnum::RgRaid.id() as u16
        || id == SkillEnum::RgIntimidate.id() as u16
        || id == SkillEnum::RgCloseconfine.id() as u16
        || id == SkillEnum::AsSonicblow.id() as u16
        || id == SkillEnum::MoInvestigate.id() as u16
        || id == SkillEnum::MoFingeroffensive.id() as u16
        || id == SkillEnum::MoTripleattack.id() as u16
        || id == SkillEnum::PaPressure.id() as u16
        || id == SkillEnum::PaSacrifice.id() as u16
        || id == SkillEnum::ChPalmstrike.id() as u16
        || id == SkillEnum::ChChaincrush.id() as u16
        || id == SkillEnum::AscBreaker.id() as u16
        || id == SkillEnum::AscMeteorassault.id() as u16
        || id == SkillEnum::HwMagicpower.id() as u16
        || id == SkillEnum::SnSharpshooting.id() as u16
        || id == SkillEnum::LkSpiralpierce.id() as u16
        || id == SkillEnum::LkHeadcrush.id() as u16
        || id == SkillEnum::LkJointbeat.id() as u16
    {
        return Attack;
    }

    if id == SkillEnum::BaMusicalstrike.id() as u16
        || id == SkillEnum::DcThrowarrow.id() as u16
        || id == SkillEnum::CgArrowvulcan.id() as u16
    {
        return Attack2;
    }

    if id == SkillEnum::KnSpearboomerang.id() as u16
        || id == SkillEnum::CrShieldcharge.id() as u16
        || id == SkillEnum::CrShieldboomerang.id() as u16
        || id == SkillEnum::PaShieldchain.id() as u16
        || id == SkillEnum::AmPotionpitcher.id() as u16
        || id == SkillEnum::AmAcidterror.id() as u16
        || id == SkillEnum::AmDemonstration.id() as u16
        || id == SkillEnum::AmCannibalize.id() as u16
        || id == SkillEnum::TfThrowstone.id() as u16
        || id == SkillEnum::TfSprinklesand.id() as u16
        || id == SkillEnum::AsVenomknife.id() as u16
    {
        return Throw;
    }

    if id == SkillEnum::HtSkidtrap.id() as u16
        || id == SkillEnum::HtLandmine.id() as u16
        || id == SkillEnum::HtAnklesnare.id() as u16
        || id == SkillEnum::HtShockwave.id() as u16
        || id == SkillEnum::HtSandman.id() as u16
        || id == SkillEnum::HtFlasher.id() as u16
        || id == SkillEnum::HtFreezingtrap.id() as u16
        || id == SkillEnum::HtBlastmine.id() as u16
        || id == SkillEnum::HtClaymoretrap.id() as u16
        || id == SkillEnum::HtRemovetrap.id() as u16
        || id == SkillEnum::HtTalkiebox.id() as u16
        || id == SkillEnum::BsGreed.id() as u16
    {
        return Pickup;
    }

    if id == SkillEnum::BaAppleidun.id() as u16
        || id == SkillEnum::BaDissonance.id() as u16
        || id == SkillEnum::BaWhistle.id() as u16
        || id == SkillEnum::BaAssassincross.id() as u16
        || id == SkillEnum::BaPoembragi.id() as u16
    {
        return Sing;
    }

    if id == SkillEnum::DcWinkcharm.id() as u16
        || id == SkillEnum::DcFortunekiss.id() as u16
        || id == SkillEnum::DcUglydance.id() as u16
        || id == SkillEnum::DcHumming.id() as u16
        || id == SkillEnum::DcDontforgetme.id() as u16
        || id == SkillEnum::DcServiceforyou.id() as u16
        || id == SkillEnum::BdLullaby.id() as u16
        || id == SkillEnum::BdRichmankim.id() as u16
        || id == SkillEnum::BdEternalchaos.id() as u16
        || id == SkillEnum::BdDrumbattlefield.id() as u16
        || id == SkillEnum::BdSiegfried.id() as u16
        || id == SkillEnum::CgHermode.id() as u16
        || id == SkillEnum::BdRingnibelungen.id() as u16
        || id == SkillEnum::BdRokisweil.id() as u16
        || id == SkillEnum::BdIntoabyss.id() as u16
        || id == SkillEnum::CgMoonlit.id() as u16
        || id == SkillEnum::CgMarionette.id() as u16
    {
        return Dance;
    }

    if id == SkillEnum::AlIncagi.id() as u16
        || id == SkillEnum::CrAutoguard.id() as u16
        || id == SkillEnum::CrReflectshield.id() as u16
        || id == SkillEnum::CrDefender.id() as u16
        || id == SkillEnum::MoSteelbody.id() as u16
        || id == SkillEnum::MoBladestop.id() as u16
        || id == SkillEnum::BdAdaptation.id() as u16
        || id == SkillEnum::LkParrying.id() as u16
        || id == SkillEnum::PaGospel.id() as u16
        || id == SkillEnum::SnSight.id() as u16
        || id == SkillEnum::WsMeltdown.id() as u16
        || id == SkillEnum::WsCartboost.id() as u16
        || id == SkillEnum::ChSoulcollect.id() as u16
    {
        return Stand;
    }

    if id == SkillEnum::TkRun.id() as u16 {
        return Walk;
    }

    Skill
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_skills_return_attack() {
        assert_eq!(
            skill_motion_type(SkillEnum::SmBash.id() as u16),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::AcDouble.id() as u16),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::AsSonicblow.id() as u16),
            SkillMotionType::Attack
        );
        assert_eq!(
            skill_motion_type(SkillEnum::LkSpiralpierce.id() as u16),
            SkillMotionType::Attack
        );
    }

    #[test]
    fn bard_musical_strike_returns_attack2() {
        assert_eq!(
            skill_motion_type(SkillEnum::BaMusicalstrike.id() as u16),
            SkillMotionType::Attack2
        );
        assert_eq!(
            skill_motion_type(SkillEnum::CgArrowvulcan.id() as u16),
            SkillMotionType::Attack2
        );
    }

    #[test]
    fn throw_skills_return_throw() {
        assert_eq!(
            skill_motion_type(SkillEnum::KnSpearboomerang.id() as u16),
            SkillMotionType::Throw
        );
        assert_eq!(
            skill_motion_type(SkillEnum::TfThrowstone.id() as u16),
            SkillMotionType::Throw
        );
    }

    #[test]
    fn trap_skills_return_pickup() {
        assert_eq!(
            skill_motion_type(SkillEnum::HtLandmine.id() as u16),
            SkillMotionType::Pickup
        );
        assert_eq!(
            skill_motion_type(SkillEnum::HtAnklesnare.id() as u16),
            SkillMotionType::Pickup
        );
    }

    #[test]
    fn bard_songs_return_sing() {
        assert_eq!(
            skill_motion_type(SkillEnum::BaPoembragi.id() as u16),
            SkillMotionType::Sing
        );
        assert_eq!(
            skill_motion_type(SkillEnum::BaAppleidun.id() as u16),
            SkillMotionType::Sing
        );
    }

    #[test]
    fn dancer_skills_return_dance() {
        assert_eq!(
            skill_motion_type(SkillEnum::DcFortunekiss.id() as u16),
            SkillMotionType::Dance
        );
        assert_eq!(
            skill_motion_type(SkillEnum::BdLullaby.id() as u16),
            SkillMotionType::Dance
        );
    }

    #[test]
    fn stand_skills_return_stand() {
        assert_eq!(
            skill_motion_type(SkillEnum::CrAutoguard.id() as u16),
            SkillMotionType::Stand
        );
        assert_eq!(
            skill_motion_type(SkillEnum::LkParrying.id() as u16),
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
            let id = s.id() as u16;
            assert_eq!(skill_motion_type(id), SkillMotionType::Pose, "{s:?}");
            let pose = skill_pose(id).expect("{s:?} poses");
            assert_eq!((pose.action, pose.frame), (action, frame), "{s:?}");
            assert_eq!(pose.hold_secs, 2.0, "{s:?}");
        }
        // Running plays the walk motion, not an idle stand or a cast.
        assert_eq!(
            skill_motion_type(SkillEnum::TkRun.id() as u16),
            SkillMotionType::Walk
        );
        assert!(skill_pose(SkillEnum::TkRun.id() as u16).is_none());
    }

    #[test]
    fn unknown_skill_defaults_to_skill() {
        assert_eq!(skill_motion_type(9999), SkillMotionType::Skill);
    }

    #[test]
    fn mercenary_bow_skills_animate_as_ranged_attack() {
        for s in [
            SkillEnum::MaDouble,
            SkillEnum::MaShower,
            SkillEnum::MaChargearrow,
        ] {
            assert_eq!(
                skill_motion_type(s.id() as u16),
                SkillMotionType::Attack,
                "{s:?} should animate as a ranged attack"
            );
        }
        assert_eq!(
            skill_motion_type(SkillEnum::MerQuicken.id() as u16),
            SkillMotionType::Skill
        );
    }

    #[test]
    fn heal_defaults_to_skill() {
        assert_eq!(
            skill_motion_type(SkillEnum::AlHeal.id() as u16),
            SkillMotionType::Skill
        );
    }
}
