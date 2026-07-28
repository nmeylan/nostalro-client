use models::enums::class::JobName;
use models::enums::element::Element;
use models::enums::skill_enums::SkillEnum;
use models::enums::weapon::WeaponType;
use ragnarok_effects::merc_skill_base;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillSoundPos {
    /// Full volume, no coordinates.
    NonPositional,
    /// Non-positional but with the original's dy volume knob (negative value).
    Depth(f32),
    /// Positional at the skill target.
    TargetPositional,
}

pub fn skill_use_sound(skill: SkillEnum) -> Option<(&'static str, SkillSoundPos)> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::BsGreed => ("effect\\ef_entry.wav", NonPositional),
        S::AllCatcry => ("effect\\2008cat.wav", NonPositional),
        S::MgStonecurse => ("_stonecurse.wav", TargetPositional),
        S::MoExplosionspirits => ("effect\\mon_폭기.wav", NonPositional),
        S::MoSteelbody => ("effect\\mon_bash3d.wav", NonPositional),
        S::LkAurablade => ("effect\\오라 블레이드.wav", NonPositional),
        S::LkBerserk | S::LkFury => ("effect\\버서크.wav", NonPositional),
        S::NpcPowerup => ("effect\\mon_폭기.wav", NonPositional),
        S::DcWinkcharm => ("effect\\vallentine.wav", NonPositional),
        S::BaPangvoice => ("amon_ra_die01.wav", NonPositional),
        S::BaAssassincross => ("effect\\석양의 어쌔신.wav", NonPositional),
        S::BaPoembragi => ("effect\\브라기의 시.wav", NonPositional),
        S::BaAppleidun => ("effect\\이둔의 사과.wav", NonPositional),
        S::DcHumming => ("effect\\흥얼거림.wav", NonPositional),
        S::DcDontforgetme => ("effect\\나를잊지말아요.wav", NonPositional),
        S::DcServiceforyou => ("effect\\당신을 위한 서비스.wav", NonPositional),
        S::BdLullaby => ("effect\\자장가.wav", NonPositional),
        S::CgHermode => ("effect\\헤르모드의 지팡이.wav", NonPositional),
        S::BdEternalchaos => ("effect\\영원의 혼돈.wav", NonPositional),
        S::BdDrumbattlefield => ("effect\\전장의.wav", NonPositional),
        S::BdRingnibelungen => ("effect\\니벨룽겐의 반지.wav", NonPositional),
        S::BdIntoabyss => ("effect\\심연속으로.wav", NonPositional),
        S::BdSiegfried => ("effect\\불사신.wav", NonPositional),
        S::BdRichmankim => ("effect\\김서방돈.wav", NonPositional),
        S::BdRokisweil => ("effect\\로키.wav", NonPositional),
        S::DcFortunekiss => ("effect\\행운의.wav", NonPositional),
        S::CgTarotcard => ("effect\\priest_slowpoison.wav", NonPositional),
        S::CgLongingfreedom => ("effect\\ac_concentration.wav", NonPositional),
        S::HwMagicpower => ("effect\\마법력 증폭.wav", NonPositional),
        S::PfDoublecasting => ("effect\\마법력 증폭.wav", NonPositional),
        S::CgMoonlit => ("effect\\달빛.wav", NonPositional),
        S::TkDodge => ("effect\\t_낙법.wav", Depth(-150.0)),
        S::TkSevenwind => ("effect\\t_바람방출.wav", NonPositional),
        S::TkMission => ("effect\\t_피링.wav", Depth(-100.0)),
        S::SgFusion => ("effect\\t_변신.wav", NonPositional),
        S::SgHate => ("effect\\t_등록.wav", NonPositional),
        S::SlSwoo => ("effect\\t_슈웃.wav", Depth(-50.0)),
        S::SlSke => ("effect\\t_공격력.wav", NonPositional),
        S::SlSka => ("effect\\t_방어형.wav", NonPositional),
        S::SlKaizel => ("effect\\priest_resurrection.wav", NonPositional),
        S::SlKaahi => ("effect\\t_보조마법.wav", NonPositional),
        S::SlKaupe => ("effect\\t_치잉.wav", NonPositional),
        S::SlKaite => ("effect\\t_마법반사.wav", NonPositional),
        S::HamiDefence => ("effect\\h_defence.wav", NonPositional),
        S::HamiCastle => ("effect\\h_castling.wav", NonPositional),
        S::NjTatamigaeshi => ("effect\\다다미뒤집기.wav", NonPositional),
        S::GsCracker => ("effect\\크래커.wav", NonPositional),
        S::GsGlittering => ("effect\\플립.wav", NonPositional),
        S::PaGospel => ("effect\\가스펠.wav", NonPositional),
        S::HpBasilica => ("effect\\바실리카.wav", NonPositional),
        S::CrAutoguard | S::MlAutoguard | S::LkParrying | S::MsParrying => {
            ("effect\\kyrie_guard.wav", NonPositional)
        }
        _ => return None,
    })
}

pub fn skill_cast_begin_sound(skill: SkillEnum) -> Option<(&'static str, SkillSoundPos)> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::MoCombofinish => ("effect\\mon_bash3d.wav", NonPositional),
        S::ChPalmstrike => ("effect\\맹호경파산.wav", NonPositional),
        S::CrSlimpitcher => ("assulter_attack.wav", Depth(-150.0)),
        S::HwGanbantein => ("effect\\EF_FireWall.wav", NonPositional),
        S::HwGravitation => ("effect\\wizard_earthspike.wav", NonPositional),
        S::NjSuiton => ("effect\\수둔.wav", NonPositional),
        S::GsGrounddrift => ("effect\\그라운드.wav", NonPositional),
        _ => return None,
    })
}

pub fn skill_projectile_sound(skill: SkillEnum) -> Option<&'static str> {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    Some(match skill {
        S::NjSyuriken | S::NjKunai | S::NjHuuma | S::NjZenynage => "effect\\닌자_던지기.wav",
        S::MoExtremityfist => "effect\\mon_아수라 패황권.wav",
        _ => return None,
    })
}

pub fn swing_sound(weapon: Option<WeaponType>) -> &'static str {
    use WeaponType as W;
    match weapon {
        Some(W::Sword1H | W::Sword2H) => "_attack_sword.wav",
        Some(W::Bow) => "_attack_bow.wav",
        Some(W::Spear1H | W::Spear2H) => "_attack_spear.wav",
        Some(W::Axe1H | W::Axe2H) => "_attack_axe.wav",
        Some(W::Staff | W::Staff2H) => "_attack_rod.wav",
        _ => "_attack_mace.wav",
    }
}

pub fn weapon_hit_sound(weapon: Option<WeaponType>, roll: u32, is_taekwon: bool) -> String {
    use WeaponType as W;
    let one_of = |base: &str, n: u32| format!("{base}{}.wav", 1 + (roll % n));
    match weapon {
        None | Some(W::Fist) => {
            if is_taekwon {
                "_hit_mace.wav".to_string()
            } else {
                one_of("_hit_fist", 4)
            }
        }
        Some(W::Sword1H | W::Sword2H) => "_hit_sword.wav".to_string(),
        Some(W::Bow) => "_hit_arrow.wav".to_string(),
        Some(W::Spear1H | W::Spear2H) => "_hit_spear.wav".to_string(),
        Some(W::Axe1H | W::Axe2H) => "_hit_axe.wav".to_string(),
        Some(W::Mace | W::Mace2H) => "_hit_mace.wav".to_string(),
        Some(W::Staff | W::Staff2H) => "_hit_rod.wav".to_string(),
        Some(W::Book) => "_hit_mace.wav".to_string(),
        Some(W::Revolver) => "_hit_권총.wav".to_string(),
        Some(W::Gatling) => "_hit_개틀링한발.wav".to_string(),
        Some(W::Shotgun) => "_hit_샷건.wav".to_string(),
        Some(W::Grenade) => "_hit_그레네이드런쳐.wav".to_string(),
        Some(W::Rifle) => "_hit_라이플.wav".to_string(),
        _ => "_hit_mace.wav".to_string(),
    }
}

pub fn skill_hit_sound(roll: u32) -> String {
    format!("_enemy_hit_normal{}.wav", 1 + (roll % 4))
}

/// PC-victim body-material hit wave (overrides the weapon table for PC targets).
pub fn job_hit_sound(job: JobName) -> &'static str {
    use JobName as J;
    match job {
        J::Archer
        | J::ArcherHigh
        | J::BabyArcher
        | J::Thief
        | J::ThiefHigh
        | J::BabyThief
        | J::Hunter
        | J::Sniper
        | J::BabyHunter
        | J::Assassin
        | J::AssassinCross
        | J::BabyAssassin
        | J::Bard
        | J::Clown
        | J::BabyBard
        | J::Dancer
        | J::Gypsy
        | J::BabyDancer
        | J::Rogue
        | J::Stalker
        | J::BabyRogue
        | J::Gunslinger
        | J::Ninja
        | J::Taekwon => "player_wooden_male.wav",

        J::Swordsman
        | J::SwordsmanHigh
        | J::BabySwordsman
        | J::Knight
        | J::LordKnight
        | J::BabyKnight
        | J::Crusader
        | J::Paladin
        | J::BabyCrusader
        | J::Monk
        | J::Champion
        | J::BabyMonk
        | J::StarGladiator => "player_metal.wav",

        _ => "player_clothes.wav",
    }
}

pub fn attr_hit_sound(element: Element, roll: u32) -> String {
    match element {
        Element::Fire => format!("_enemy_hit_fire{}.wav", 1 + (roll % 2)),
        Element::Wind => format!("_enemy_hit_wind{}.wav", 1 + (roll % 2)),
        _ => skill_hit_sound(roll),
    }
}

/// A status/ailment transition sound. `enter` distinguishes onset from clear.
pub fn status_sound(kind: StatusSoundKind) -> Option<&'static str> {
    use StatusSoundKind as S;
    Some(match kind {
        S::FreezeEnter => "_stonecurse.wav",
        S::FreezeExit => "_frozen_explosion.wav",
        S::StoneCurseExit => "_stone_explosion.wav",
        S::StunEnter => "_stun.wav",
        S::PoisonSet => "_poison.wav",
        S::CurseSet => "_curse.wav",
        S::SilenceSet => "_silence.wav",
        S::ConfusionSet => "_confusion.wav",
        S::BlindSet => "_blind.wav",
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusSoundKind {
    FreezeEnter,
    FreezeExit,
    StoneCurseExit,
    StunEnter,
    PoisonSet,
    CurseSet,
    SilenceSet,
    ConfusionSet,
    BlindSet,
}

pub mod ui {
    pub const LOGIN: &str = "login.wav";
    pub const BUTTON: &str = "\u{BC84}\u{D2BC}\u{C18C}\u{B9AC}.wav"; // 버튼소리.wav
    pub const REPAIR: &str = "repair.wav";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_and_job_tables_pick_expected_waves() {
        assert_eq!(
            weapon_hit_sound(Some(WeaponType::Bow), 0, false),
            "_hit_arrow.wav"
        );
        assert_eq!(
            weapon_hit_sound(Some(WeaponType::Sword2H), 0, false),
            "_hit_sword.wav"
        );
        assert_eq!(weapon_hit_sound(None, 0, false), "_hit_fist1.wav");
        assert_eq!(weapon_hit_sound(None, 2, false), "_hit_fist3.wav");
        assert_eq!(weapon_hit_sound(None, 0, true), "_hit_mace.wav");
        assert_eq!(job_hit_sound(JobName::Novice), "player_clothes.wav");
        assert_eq!(job_hit_sound(JobName::Archer), "player_wooden_male.wav");
        assert_eq!(job_hit_sound(JobName::Knight), "player_metal.wav");
        assert_eq!(swing_sound(Some(WeaponType::Axe2H)), "_attack_axe.wav");
        assert_eq!(swing_sound(None), "_attack_mace.wav");
        assert_eq!(attr_hit_sound(Element::Fire, 1), "_enemy_hit_fire2.wav");
    }
}
