use models::enums::skill_enums::SkillEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillMotionType {
    /// Weapon-dependent attack (resolved via attack_action_for_weapon)
    Attack,
    /// Action index 5 - throwing without visible weapon
    Throw,
    /// Action index 10 - secondary attack
    Attack2,
    /// Action index 3 - item pickup / trap placement
    Pickup,
    /// Singing animation (bard songs) - maps to 12 for now
    Sing,
    /// Dancing animation (dancer/ensemble) - maps to 12 for now
    Dance,
    /// Action index 0 - stay standing (passives/buffs)
    Stand,
    /// Action index 12 - generic skill cast (default)
    Skill,
}

/// Maps a skill ID to its motion type, following the original game's GetSkillActionInfo().
pub fn skill_motion_type(skill_id: u16) -> SkillMotionType {
    use SkillMotionType::*;

    let id = skill_id;
    // Attack - weapon-dependent animation (same as regular attack)
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

    // Attack2 - secondary attack animation
    if id == SkillEnum::BaMusicalstrike.id() as u16
        || id == SkillEnum::DcThrowarrow.id() as u16
        || id == SkillEnum::CgArrowvulcan.id() as u16
    {
        return Attack2;
    }

    // Throw - throwing without visible weapon
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

    // Pickup - trap placement / item pickup
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

    // Sing - bard songs
    if id == SkillEnum::BaAppleidun.id() as u16
        || id == SkillEnum::BaDissonance.id() as u16
        || id == SkillEnum::BaWhistle.id() as u16
        || id == SkillEnum::BaAssassincross.id() as u16
        || id == SkillEnum::BaPoembragi.id() as u16
    {
        return Sing;
    }

    // Dance - dancer/ensemble skills
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

    // Stand - passives/buffs that don't change pose
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

    // Everything else: generic skill animation
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
    fn unknown_skill_defaults_to_skill() {
        assert_eq!(skill_motion_type(9999), SkillMotionType::Skill);
    }

    #[test]
    fn heal_defaults_to_skill() {
        assert_eq!(
            skill_motion_type(SkillEnum::AlHeal.id() as u16),
            SkillMotionType::Skill
        );
    }
}
