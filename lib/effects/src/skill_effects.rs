use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CasterSkillEffects {
    pub cast: &'static [EffectId],
}

/// Everything decided at or before cast start: the begin-cast visual and
/// whether the cast bar / begin aura are suppressed. Distinct from
/// [`CasterSkillEffects`], which is the execution-time (cast-END) visual.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CastingSkill {
    pub begin: &'static [EffectId],
    pub hide_cast_bar: bool,
    pub hide_cast_aura: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TargetSkillEffects {
    pub on_target: &'static [EffectId],
    pub before_hit: &'static [EffectId],
    pub hit: &'static [EffectId],
    pub hit_extra_delay_secs: f32,
}

impl CasterSkillEffects {
    const fn cast(cast: &'static [EffectId]) -> Self {
        Self { cast }
    }
}

impl TargetSkillEffects {
    const fn on_target(on_target: &'static [EffectId]) -> Self {
        Self {
            on_target,
            before_hit: &[],
            hit: &[],
            hit_extra_delay_secs: 0.0,
        }
    }
    const fn hit(hit: &'static [EffectId]) -> Self {
        Self {
            on_target: &[],
            before_hit: &[],
            hit,
            hit_extra_delay_secs: 0.0,
        }
    }
}

/// Mercenary skills reuse the visuals and sounds of the base class skill they
/// mirror. Callers of the effect/sound tables resolve a merc skill to its base
/// first; skills with no mapping (including support buffs that have no dedicated
/// effect in the original game) pass through unchanged.
pub fn merc_skill_base(skill: SkillEnum) -> SkillEnum {
    use SkillEnum as S;
    match skill {
        S::MsBash => S::SmBash,
        S::MsMagnum => S::SmMagnum,
        S::MsBowlingbash => S::KnBowlingbash,
        S::MsParrying => S::LkParrying,
        S::MsReflectshield => S::CrReflectshield,
        S::MsBerserk => S::LkBerserk,
        S::MaDouble => S::AcDouble,
        S::MaShower => S::AcShower,
        S::MaSharpshooting => S::SnSharpshooting,
        S::MaChargearrow => S::AcChargearrow,
        S::MaSkidtrap => S::HtSkidtrap,
        S::MaLandmine => S::HtLandmine,
        S::MaSandman => S::HtSandman,
        S::MaFreezingtrap => S::HtFreezingtrap,
        S::MaRemovetrap => S::HtRemovetrap,
        S::MlPierce => S::KnPierce,
        S::MlBrandish => S::KnBrandishspear,
        S::MlSpiralpierce => S::LkSpiralpierce,
        S::MlDefender => S::CrDefender,
        S::MlAutoguard => S::CrAutoguard,
        S::MlDevotion => S::CrDevotion,
        S::MerMagnificat => S::PrMagnificat,
        S::MerProvoke => S::SmProvoke,
        S::MerSight => S::MgSight,
        S::MerDecagi => S::AlDecagi,
        S::MerIncagi => S::AlIncagi,
        S::MerBlessing => S::AlBlessing,
        S::MerKyrie => S::PrKyrie,
        S::MerLexdivina => S::PrLexdivina,
        other => other,
    }
}

/// Id-based [`merc_skill_base`] for callers that hold a raw skill id. Only the
/// contiguous mercenary id block is resolved (every id in it is defined);
/// everything else passes through untouched, so unknown ids never reach the
/// panicking `SkillEnum::from_id`.
pub fn merc_skill_base_id(skill_id: u16) -> u16 {
    let merc_range = SkillEnum::MsBash.id()..=SkillEnum::MerInvincibleoff2.id();
    if merc_range.contains(&(skill_id as u32)) {
        merc_skill_base(SkillEnum::from_id(skill_id as u32)).id() as u16
    } else {
        skill_id
    }
}

pub fn is_ground_cast(skill: SkillEnum) -> bool {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    matches!(
        skill,
        S::MgSafetywall
            | S::MgFirewall
            | S::MgThunderstorm
            | S::AlPneuma
            | S::AlWarp
            | S::AcShower
            | S::PrBenedictio
            | S::PrSanctuary
            | S::PrMagnus
            | S::WzFirepillar
            | S::WzMeteor
            | S::WzVermilion
            | S::WzIcewall
            | S::WzStormgust
            | S::WzHeavendrive
            | S::WzQuagmire
            | S::BsHammerfall
            | S::HtSkidtrap
            | S::HtLandmine
            | S::HtAnklesnare
            | S::HtShockwave
            | S::HtSandman
            | S::HtFlasher
            | S::HtFreezingtrap
            | S::HtBlastmine
            | S::HtClaymoretrap
            | S::HtTalkiebox
            | S::AsVenomdust
            | S::RgGraffiti
            | S::AmDemonstration
            | S::AmCannibalize
            | S::AmSpheremine
            | S::MoBodyrelocation
            | S::SaVolcano
            | S::SaDeluge
            | S::SaViolentgale
            | S::SaLandprotector
            | S::WsSystemcreate
            | S::PfFogwall
            | S::CrSlimpitcher
            | S::HwGanbantein
            | S::HwGravitation
            | S::CrCultivation
            | S::GsGrounddrift
            | S::NjShadowjump
            | S::NjSuiton
    )
}

pub fn caster_cast_on_use(skill: SkillEnum) -> bool {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    matches!(
        skill,
        S::AscMeteorassault | S::SmMagnum | S::WzSightrasher | S::RgRaid
    )
}

pub fn ground_placed_effect(skill: SkillEnum, level: i16) -> &'static [EffectId] {
    let skill = merc_skill_base(skill);
    use EffectId as E;
    use SkillEnum as S;
    match skill {
        S::WzStormgust => &[E::Stormgust],
        S::WzMeteor => &[E::Meteorstorm],
        S::WzVermilion => &[E::Lord],
        S::WzHeavendrive => &[E::Heavensdrive],
        S::MgThunderstorm => &[E::Thunderstorm],
        S::BsHammerfall => &[E::Crashearth],
        S::HwGanbantein => &[E::Ganbantein],
        S::HtDetecting => &[E::Detecting],
        S::PrBenedictio => &[E::Benedictio],
        S::CrSlimpitcher if level < 6 => &[E::Slim],
        S::CrSlimpitcher if level < 10 => &[E::Slim2],
        S::CrSlimpitcher => &[E::Slim3],
        S::CrCultivation => &[E::Beginspell6],
        _ => &[],
    }
}

/// Potion index for a pitcher throw (see `throw_item::potion_throw_params`),
/// or `None` for skills that don't throw a potion. Potion Pitcher's five levels
/// map to red/orange/yellow/white/blue; Slim Potion Pitcher's tiers to
/// red/yellow/white slim potions.
pub fn potion_throw_index(skill: SkillEnum, level: i16) -> Option<u8> {
    use SkillEnum as S;
    Some(match skill {
        S::AmPotionpitcher => level.clamp(1, 5) as u8,
        S::CrSlimpitcher if level < 6 => 6,
        S::CrSlimpitcher if level < 10 => 7,
        S::CrSlimpitcher => 8,
        _ => return None,
    })
}

pub fn begin_cast_effect(skill: SkillEnum) -> &'static [EffectId] {
    let skill = merc_skill_base(skill);
    use EffectId as E;
    use SkillEnum as S;
    match skill {
        S::AcDouble
        | S::McMammonite
        | S::HtPower
        | S::HtPhantasmic
        | S::HtBlitzbeat
        | S::SnFalconassault
        | S::AmSpheremine
        | S::AmDemonstration
        | S::HwMagicpower
        | S::WsMeltdown
        | S::BaMusicalstrike
        | S::DcThrowarrow => &[E::Bash],

        S::MgNapalmbeat
        | S::MgSoulstrike
        | S::MgColdbolt
        | S::MgFrostdiver
        | S::MgStonecurse
        | S::MgFireball
        | S::MgFirewall
        | S::MgFirebolt
        | S::MgLightningbolt
        | S::MgThunderstorm
        | S::MgSafetywall
        | S::MgSrecovery
        | S::MgSight
        | S::MgEnergycoat
        | S::WzFirepillar
        | S::WzSightrasher
        | S::WzMeteor
        | S::WzJupitel
        | S::WzVermilion
        | S::WzIcewall
        | S::WzSightblaster
        | S::WzFrostnova
        | S::WzStormgust
        | S::WzEarthspike
        | S::WzHeavendrive
        | S::WzQuagmire
        | S::WzWaterball
        | S::MerMagnificat
        | S::PrImpositio
        | S::PrSuffragium
        | S::PrAspersio
        | S::PrBenedictio
        | S::PrSanctuary
        | S::PrStrecovery
        | S::PrKyrie
        | S::PrMagnificat
        | S::PrGloria
        | S::PrLexdivina
        | S::PrTurnundead
        | S::PrLexaeterna
        | S::PrMagnus
        | S::AlHeal
        | S::AlBlessing
        | S::CashBlessing
        | S::AlIncagi
        | S::CashIncagi
        | S::AlPneuma
        | S::AlRuwach
        | S::AlHolywater
        | S::AlCrucis
        | S::AlAngelus
        | S::AlCure
        | S::AlHolylight
        | S::AllResurrection
        | S::BsRepairweapon
        | S::CrGrandcross
        | S::CrProvidence
        | S::CrDevotion
        | S::CrFullprotection
        | S::MoSteelbody
        | S::MoCallspirits
        | S::MoAbsorbspirits
        | S::MoFingeroffensive
        | S::MoInvestigate
        | S::AmPotionpitcher
        | S::AmAcidterror
        | S::AmCannibalize
        | S::CrSlimpitcher
        | S::CrAciddemonstration
        | S::HwMagiccrasher
        | S::HwNapalmvulcan
        | S::PfHpconversion
        | S::PfSoulchange
        | S::PfMemorize
        | S::AlDecagi
        | S::AlWarp
        | S::HwGravitation
        | S::ChSoulcollect
        | S::MoKitranslation
        | S::SaAutospell
        | S::PfDoublecasting
        | S::SaFlamelauncher
        | S::SaFrostweapon
        | S::SaLightningloader
        | S::SaSeismicweapon
        | S::SaVolcano
        | S::SaDeluge
        | S::SaViolentgale
        | S::SaLandprotector
        | S::AmCpWeapon
        | S::AmCpShield
        | S::AmCpArmor
        | S::AmCpHelm
        | S::SaSpellbreaker
        | S::SaDispell => &[E::Beginspell],

        S::SaElementwater | S::NjHyousensou | S::NjHyousyouraku | S::NjSuiton => &[E::Beginspell2],
        S::SaElementfire | S::NjKouenka | S::NjBakuenryu | S::NjKaensin => &[E::Beginspell3],
        S::SaElementground => &[E::Beginspell4],
        S::SaElementwind | S::NjHuujin | S::NjKamaitachi | S::NjRaigekisai => &[E::Beginspell5],

        S::PaPressure
        | S::HpAssumptio
        | S::HpBasilica
        | S::SnSharpshooting
        | S::CgArrowvulcan
        | S::SnWindwalk
        | S::PaShieldchain
        | S::CgTarotcard
        | S::KnChargeatk
        | S::PrRedemptio
        | S::HwGanbantein
        | S::BaPangvoice => &[E::Beginspell6],

        S::AsSplasher | S::StPreserve | S::AscBreaker | S::AscMeteorassault => &[E::Beginspell7],

        S::AlDemonbane => &[E::Beginspellwhite],
        S::AcConcentration => &[E::Incagidex],
        S::MoExtremityfist => &[E::Beginasura],

        S::AmCallhomun | S::AmRest | S::AmResurrecthomun | S::WeMale | S::WeBaby | S::WeCallparent
        | S::WeCallbaby => &[E::Couplecasting],
        S::WeFemale => &[E::Heartcasting],

        S::AmTwilight1 => &[E::Twilight1],
        S::AmTwilight2 => &[E::Twilight2],
        S::AmTwilight3 => &[E::Twilight3],

        S::LkSpiralpierce => &[E::Piercebody],

        S::TkHighjump => &[E::Jumpbody, E::Peong],

        S::TkRun
        | S::SgFeel
        | S::SgHate
        | S::SgFusion
        | S::TkSevenwind
        | S::TkMission
        | S::SlStin
        | S::SlStun
        | S::SlSma
        | S::SlKaahi
        | S::SlKaupe
        | S::SlKaite
        | S::SlKaizel
        | S::GsBullseye
        | S::GsTracking
        | S::GsPiercingshot
        | S::GsDust
        | S::GsMadnesscancel
        | S::GsAdjustment
        | S::GsGrounddrift
        | S::NjHuuma
        | S::NjBunsinjyutsu
        | S::NjNen
        | S::SlSwoo
        | S::SlSke
        | S::SlSka
        | S::SlAlchemist
        | S::SlMonk
        | S::SlStar
        | S::SlSage
        | S::SlCrusader
        | S::SlSupernovice
        | S::SlKnight
        | S::SlWizard
        | S::SlPriest
        | S::SlBarddancer
        | S::SlRogue
        | S::SlAssasin
        | S::SlBlacksmith
        | S::SlHunter
        | S::SlSoullinker
        | S::SlHigh => &[E::Bluecasting],

        _ => &[],
    }
}

pub fn fire_glyph_effect(skill: SkillEnum) -> &'static [EffectId] {
    let skill = merc_skill_base(skill);
    use EffectId as E;
    use SkillEnum as S;
    match skill {
        S::KnBrandishspear => &[E::Brandish2],
        S::AcChargearrow => &[E::Bash],
        _ => &[],
    }
}

pub fn casting_skill(skill: SkillEnum) -> CastingSkill {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    CastingSkill {
        begin: begin_cast_effect(skill),
        hide_cast_bar: matches!(
            skill,
            S::KnBowlingbash | S::KnBrandishspear | S::LkSpiralpierce | S::TkHighjump
        ),
        hide_cast_aura: matches!(
            skill,
            S::KnBowlingbash | S::KnBrandishspear | S::LkSpiralpierce
        ),
    }
}

pub fn is_cast_circle(id: EffectId) -> bool {
    use EffectId as E;
    matches!(
        id,
        E::Beginspell
            | E::Beginspell2
            | E::Beginspell3
            | E::Beginspell4
            | E::Beginspell5
            | E::Beginspell6
            | E::Beginspell7
            | E::Beginspellwhite
            | E::Bluecasting
            | E::Beginasura
            | E::Couplecasting
            | E::Heartcasting
            | E::Castspin
            | E::Incagidex
            | E::Brandish2
    )
}

pub fn beginspell_for_element(property: u32) -> EffectId {
    match property {
        1 => EffectId::Beginspell2,
        3 => EffectId::Beginspell3,
        2 => EffectId::Beginspell4,
        4 => EffectId::Beginspell5,
        6 => EffectId::Beginspell6,
        5 => EffectId::Beginspell7,
        _ => EffectId::Beginspell,
    }
}

pub fn caster_skill_effects(skill: SkillEnum) -> CasterSkillEffects {
    let skill = merc_skill_base(skill);
    use EffectId as E;
    use SkillEnum as S;
    type C = CasterSkillEffects;

    match skill {
        S::SmMagnum => C::cast(&[E::Magnumbreak]),
        S::AlHolywater => C::cast(&[E::Aqua]),
        S::AlCrucis => C::cast(&[E::Signum]),
        S::AlAngelus => C::cast(&[E::Angelus]),
        S::McCartrevolution => C::cast(&[E::Cartrevolution]),
        S::AcConcentration => C::cast(&[E::Concentration]),
        S::NvFirstaid => C::cast(&[E::Firstaid]),
        S::WeCallpartner => C::cast(&[E::Couplecasting]),

        S::KnPierce => C::cast(&[E::Pierceself]),
        S::KnSpearboomerang => C::cast(&[E::Spearbmrself]),
        S::KnBowlingbash => C::cast(&[E::Bowlingself]),
        S::KnBrandishspear => C::cast(&[]),
        S::SmEndure => C::cast(&[E::Endure]),
        S::KnTwohandquicken | S::KnOnehand => C::cast(&[E::Twohandquicken]),
        S::PrMagnificat | S::MerMagnificat => C::cast(&[E::Magnificat]),
        S::PrGloria => C::cast(&[E::Gloria]),
        S::PrKyrie => C::cast(&[E::Kyrie]),
        S::PrTurnundead => C::cast(&[E::Turnundead]),
        S::WzFirepillar => C::cast(&[E::Firepillar]),
        S::WzSightrasher => C::cast(&[E::Sightrasher]),
        S::WzStormgust => C::cast(&[E::Stormgust]),
        S::HtSpringtrap => C::cast(&[E::Springtrap]),
        S::HtRemovetrap => C::cast(&[E::Removetrap]),
        S::AsSonicblow => C::cast(&[E::Sonicblow2]),

        S::MoAbsorbspirits => C::cast(&[E::Absorbspirits]),
        S::MoExplosionspirits => C::cast(&[E::Gumgang, E::Gumgang2]),
        S::MoSteelbody => C::cast(&[E::Steelbody, E::Gumgang2]),
        S::MoChaincombo => C::cast(&[E::Gumgang3]),
        S::MoCombofinish => C::cast(&[E::Gumgang3, E::Hitline]),
        S::ChTigerfist => C::cast(&[E::Bash3d2, E::Gumgang3]),
        S::ChChaincrush => C::cast(&[E::Gumgang3]),

        S::CrGrandcross => C::cast(&[E::Grandcross]),
        S::CrDevotion => C::cast(&[E::Devotion]),
        S::CrSpearquicken => C::cast(&[E::Spearquicken]),
        S::CrReflectshield => C::cast(&[E::Reflectshield]),
        S::CrDefender => C::cast(&[E::Defender]),
        S::CrShrink => C::cast(&[E::Shrink]),
        S::CrAutoguard => C::cast(&[E::Guard]),
        S::PaSacrifice => C::cast(&[E::Bash3d]),
        S::LkSpiralpierce => C::cast(&[]),
        S::LkParrying => C::cast(&[E::Guard]),
        S::LkHeadcrush => C::cast(&[E::Bash3d3]),
        S::LkJointbeat => C::cast(&[E::Bash3d4]),
        S::LkAurablade => C::cast(&[E::Aurablade, E::Aurablade2]),

        S::MgEnergycoat => C::cast(&[E::Energycoat]),
        S::BsAdrenaline | S::BsAdrenaline2 => C::cast(&[E::Hasteup]),
        S::BsMaximize => C::cast(&[E::Maxpower]),
        S::BsOverthrust | S::WsOverthrustmax => C::cast(&[E::Overthrust]),
        S::McLoud => C::cast(&[E::Loud]),
        S::WsMeltdown => C::cast(&[E::Meltdown]),
        S::WsCartboost => C::cast(&[E::Cartboost]),

        S::SgSunComfort | S::SgMoonComfort | S::SgStarComfort => C::cast(&[E::Flowercast, E::Hated]),
        S::TkSevenwind => C::cast(&[E::Stormkick3, E::Beginasura1]),

        S::RgIntimidate => C::cast(&[E::Intimidate]),
        S::RgRaid => C::cast(&[E::Teihit3]),
        S::StPreserve => C::cast(&[E::Guard2]),
        S::StRejectsword => C::cast(&[E::Rejectsword]),

        S::SnSight => C::cast(&[E::Truesight]),
        S::SnWindwalk => C::cast(&[E::Portal4]),
        S::CgLongingfreedom => C::cast(&[E::Chemicalbody]),
        S::CgMoonlit => C::cast(&[E::Spherewind2]),
        S::CgMarionette => C::cast(&[E::Linelink3, E::Pinkbody]),
        S::BaFrostjoker => C::cast(&[E::TalkFrostjoke]),
        S::BaPangvoice => C::cast(&[E::Fvoice]),
        S::DcScream => C::cast(&[E::TalkScream]),
        S::DcWinkcharm => C::cast(&[E::Wink]),

        S::SaMagicrod => C::cast(&[E::Magicrod]),
        S::HwSouldrain => C::cast(&[E::Energydrain2]),
        S::PfHpconversion => C::cast(&[E::Energydrain3]),
        S::PfSoulchange => C::cast(&[E::Linelink2, E::Linklight, E::Soulchange]),
        S::PfMemorize => C::cast(&[E::Memorize]),

        S::AscMeteorassault => C::cast(&[E::Soulbreaker2]),
        S::AscEdp => C::cast(&[E::Edp]),
        S::AmPotionpitcher => C::cast(&[E::Throwitem2]),
        S::AmBerserkpitcher => C::cast(&[E::Throwitem5]),
        S::ItmTomahawk => C::cast(&[E::Shieldboomerang2]),

        S::TkStormkick => C::cast(&[E::Stormkick]),
        S::TkCounter => C::cast(&[E::Hitline5]),
        S::TkJumpkick => C::cast(&[E::Jumpkick]),
        S::TkHighjump => C::cast(&[E::Landbody]),
        S::TkRun => C::cast(&[E::Run]),

        S::GsMadnesscancel => C::cast(&[E::MadnessBlue]),
        S::GsAdjustment | S::GsGatlingfever => C::cast(&[E::MadnessRed]),
        S::GsIncreasing => C::cast(&[E::Agiup]),
        S::GsDesperado => C::cast(&[E::Desperado]),
        S::NjRaigekisai => C::cast(&[E::Thunderstorm2]),
        S::NjHyousyouraku => C::cast(&[E::Hyousyouraku]),

        S::HfliSpeed | S::HfliFleet => C::cast(&[E::Homuncasting]),
        S::HlifChange => C::cast(&[E::Memorize]),
        S::HvanExplosion => C::cast(&[E::SuiExplosion]),
        S::HamiDefence => C::cast(&[E::Hamidefence]),
        S::HamiCastle => C::cast(&[E::Hamicastle]),
        S::HamiBloodlust => C::cast(&[E::Hamiblood]),
        S::HlifAvoid => C::cast(&[E::Agiup]),

        S::NpcChangewater => C::cast(&[E::Changecold]),
        S::NpcChangeground => C::cast(&[E::Changeearth]),
        S::NpcChangefire => C::cast(&[E::Changefire]),
        S::NpcChangewind => C::cast(&[E::Changewind]),
        S::NpcChangepoison => C::cast(&[E::Changepoison]),
        S::NpcChangeholy => C::cast(&[E::Chaingeholy]),
        S::NpcChangedarkness => C::cast(&[E::Changedark]),
        S::NpcChangetelekinesis => C::cast(&[E::Changeflame]),
        S::NpcSelfdestruction => C::cast(&[E::SuiExplosion]),
        S::NpcSummonslave => C::cast(&[E::Summonslave]),
        S::NpcKeeping => C::cast(&[E::Keeping]),
        S::NpcDefender => C::cast(&[E::Deffender]),
        S::NpcPowerup => C::cast(&[E::Gumgangnpc]),
        S::NpcAgiup => C::cast(&[E::Agiup]),

        _ => C::default(),
    }
}

pub fn target_skill_effects(skill: SkillEnum) -> TargetSkillEffects {
    let skill = merc_skill_base(skill);
    use EffectId as E;
    use SkillEnum as S;
    type T = TargetSkillEffects;

    match skill {
        // Self-AoE caster fx (fired at the use packet) charge up before impact;
        // hold the hit so the damage lands when the effect hits, not on the
        // backdated server tick that would otherwise precede the visual.
        S::SmMagnum => T {
            hit: &[E::Firehit],
            hit_extra_delay_secs: 8.0 / 60.0,
            ..Default::default()
        },
        S::AscMeteorassault => T {
            hit_extra_delay_secs: 28.0 / 60.0,
            ..Default::default()
        },
        S::RgRaid => T {
            hit_extra_delay_secs: 20.0 / 60.0,
            ..Default::default()
        },
        S::SmProvoke => T::on_target(&[E::Provoke]),
        S::MgNapalmbeat => T::on_target(&[E::Hit2]),
        S::MgSoulstrike => T {
            before_hit: &[E::Soulstrike],
            ..Default::default()
        },
        S::MgFirebolt => T {
            on_target: &[E::Firearrow],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::MgFireball => T {
            on_target: &[E::Fireball],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::MgColdbolt => T {
            on_target: &[E::Icearrow],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::MgLightningbolt => T {
            on_target: &[E::Lightbolt],
            hit: &[E::Windhit],
            ..Default::default()
        },
        S::MgFrostdiver => T {
            before_hit: &[E::Frostdiver],
            on_target: &[E::Frostdiver2],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::MgStonecurse => T {
            on_target: &[E::Stonecurse],
            hit: &[E::Stonecurse],
            ..Default::default()
        },
        S::MgThunderstorm => T::on_target(&[E::Thunderstorm]),
        S::AlDemonbane => T::on_target(&[E::Tanji2]),
        S::AlHeal => T::on_target(&[E::Heal]),
        S::AlHolylight => T::hit(&[E::Holyhit]),
        S::AlCure => T::on_target(&[E::Cure]),
        S::AlIncagi | S::CashIncagi => T::on_target(&[E::Incagility]),
        S::AlBlessing | S::CashBlessing => T::on_target(&[E::Blessing]),
        S::McMammonite => T::hit(&[E::Coin]),
        S::McCartrevolution => T::hit(&[E::Cartrevolution]),
        S::TfSteal => T::on_target(&[E::Steal]),
        S::TfDetoxify => T::on_target(&[E::Detoxication]),
        S::TfSprinklesand => T::on_target(&[E::Sprinklesand]),
        S::TfThrowstone => T::on_target(&[E::Throwitem3]),

        S::KnPierce => T::hit(&[E::Pierce]),
        S::KnSpearstab => T {
            on_target: &[E::Spearstabself],
            hit: &[E::Pierce],
            ..Default::default()
        },
        S::KnSpearboomerang => T {
            on_target: &[E::Spearbmr],
            hit: &[E::Hit4],
            ..Default::default()
        },
        S::KnBowlingbash => T::hit(&[E::Bowlingbash]),
        S::KnBrandishspear => T::on_target(&[E::Brandishspear]),
        S::PrImpositio => T {
            on_target: &[E::Impositio],
            hit: &[E::Impositio],
            ..Default::default()
        },
        S::PrSuffragium => T {
            on_target: &[E::Suffragium],
            hit: &[E::Suffragium],
            ..Default::default()
        },
        S::PrAspersio => T {
            on_target: &[E::Aspersio],
            hit: &[E::Holyhit],
            ..Default::default()
        },
        S::PrTurnundead => T {
            hit: &[E::Holyhit],
            hit_extra_delay_secs: 50.0 / 60.0,
            ..Default::default()
        },
        S::PrMagnus => T::hit(&[E::Holyhit]),
        S::PrMagnificat | S::MerMagnificat => T::hit(&[E::Magnificat]),
        S::PrGloria => T::hit(&[E::Gloria]),
        S::PrLexdivina => T {
            on_target: &[E::Lexdivina],
            hit: &[E::Lexdivina],
            ..Default::default()
        },
        S::PrLexaeterna => T {
            on_target: &[E::Lexaeterna],
            hit: &[E::Lexaeterna],
            ..Default::default()
        },
        S::PrSlowpoison => T::on_target(&[E::Slowpoison]),
        S::PrStrecovery => T::on_target(&[E::Recovery]),
        S::HpAssumptio | S::CashAssumptio => T::on_target(&[E::Assumptio, E::Assumptio2]),
        S::WzFirepillar => T::hit(&[E::Firehit]),
        S::WzSightrasher => T {
            hit: &[E::Firehit],
            hit_extra_delay_secs: 10.0 / 60.0,
            ..Default::default()
        },
        S::WzJupitel => T {
            on_target: &[E::Yufitel],
            hit: &[E::Yufitelhit],
            ..Default::default()
        },
        S::WzStormgust => T::hit(&[E::Coldhit]),
        S::WzMeteor => T::hit(&[E::Firehit]),
        S::WzVermilion => T::hit(&[E::Windhit]),
        S::WzFrostnova => T {
            on_target: &[E::Frostdiver2],
            hit: &[E::Frostdiver2],
            ..Default::default()
        },
        S::WzEarthspike => T {
            on_target: &[E::Earthspike],
            hit: &[E::Earthhit],
            ..Default::default()
        },
        S::WzHeavendrive => T::hit(&[E::Earthhit]),
        S::WzQuagmire => T::hit(&[E::Earthhit]),
        S::WzWaterball => T::on_target(&[E::Waterball2]),
        S::WzEstimation => T::hit(&[E::Lockon]),
        S::BsRepairweapon => T::on_target(&[E::Repairweapon]),
        S::BsWeaponperfect => T::on_target(&[E::Perfection]),
        S::HtSkidtrap => T::hit(&[E::Bowlingbash]),
        S::HtBlitzbeat => T::on_target(&[E::Blitzbeat]),
        S::AsSonicblow => T {
            on_target: &[E::Sonicblow],
            hit: &[E::Sonicblowhit],
            ..Default::default()
        },
        S::AsGrimtooth => T {
            on_target: &[E::Grimtooth],
            hit: &[E::Grimtoothatk],
            ..Default::default()
        },
        S::AsVenomdust => T::hit(&[E::Venomdust]),
        S::AsEnchantpoison => T::on_target(&[E::EnchantpoisonFlow]),
        S::AsPoisonreact => T::on_target(&[E::Poisonreact]),
        S::AsSplasher => T::on_target(&[E::Enchantpoison, E::Splasher]),
        S::AsVenomknife => T::on_target(&[E::Throwitem6]),
        S::AscBreaker => T::on_target(&[E::Soulbreaker]),

        S::MoChaincombo => T {
            on_target: &[E::Teihit1, E::Chaincombo],
            hit: &[E::Sonicblowhit],
            ..Default::default()
        },
        S::MoBalkyoung => T::hit(&[E::Hit3]),
        S::MoExtremityfist => T::on_target(&[E::Teihit1x]),
        S::MoTripleattack => T::on_target(&[E::Tripleattack]),
        S::MoInvestigate => T::on_target(&[E::Teihit2, E::Chimto]),
        S::ChPalmstrike => T::on_target(&[E::Hitline2]),
        S::ChChaincrush => T::on_target(&[E::Chemical2]),
        S::MoFingeroffensive => T::on_target(&[E::Tanji]),

        S::CrHolycross => T::on_target(&[E::Holycross]),
        S::CrShieldboomerang => T::on_target(&[E::Shieldboomerang]),
        S::CrAciddemonstration => T::on_target(&[E::Throwitem4, E::Aciddemon]),
        S::CrShieldcharge => T::on_target(&[E::Shieldcharge]),
        S::CrProvidence => T::on_target(&[E::Providence]),
        S::CrFullprotection => T::on_target(&[E::Chemicalprotection, E::Chemicalbody]),
        S::PaShieldchain => T::on_target(&[E::Shieldboomerang3]),
        S::PaPressure => T::on_target(&[E::Pressure]),
        S::LkSpiralpierce => T {
            on_target: &[E::Magnum2],
            hit: &[E::Pierce],
            ..Default::default()
        },
        S::WsCarttermination => T::on_target(&[E::Cartter]),

        S::RgBackstap => T::on_target(&[E::Backstap]),
        S::RgStealcoin => T::on_target(&[E::RgCoin, E::Stealcoin]),
        S::RgStripweapon => T::on_target(&[E::Stripweapon]),
        S::RgStripshield => T::on_target(&[E::Stripshield]),
        S::RgStriparmor => T::on_target(&[E::Striparmor]),
        S::RgStriphelm => T::on_target(&[E::Striphelm]),
        S::RgCloseconfine => T::on_target(&[E::Quakebody4]),
        S::StFullstrip => T::on_target(&[E::RgCoin2]),

        S::SnFalconassault => T {
            on_target: &[E::Falconassault, E::Blitzbeat],
            hit: &[E::Hit1],
            ..Default::default()
        },
        S::CgTarotcard => T::on_target(&[E::Chemicalbody]),
        S::SnSharpshooting => T::on_target(&[E::Tripleattack2]),
        S::CgArrowvulcan => T::on_target(&[E::Tripleattack3]),

        S::SaSpellbreaker => T::on_target(&[E::Spellbreaker]),
        S::SaDispell => T::on_target(&[E::Dispell]),
        S::SaFlamelauncher | S::SaElementfire => T::on_target(&[E::Flamelauncher]),
        S::SaFrostweapon | S::SaElementwater => T::on_target(&[E::Frostweapon]),
        S::SaLightningloader | S::SaElementwind => T::on_target(&[E::Lightningloader]),
        S::SaSeismicweapon | S::SaElementground => T::on_target(&[E::Seismicweapon]),
        S::HwMagiccrasher => T::on_target(&[E::Magiccrasher]),
        S::HwMagicpower => T::on_target(&[E::Lightblade]),
        S::HwNapalmvulcan => T::on_target(&[E::Napalmvalcan]),
        S::HwSouldrain => T::on_target(&[E::Transbluebody]),
        S::PfSoulchange => T::on_target(&[E::Linklight, E::Soulchange]),
        S::PfDoublecasting => T::on_target(&[E::Doublecastbody]),
        S::PfSoulburn => T::on_target(&[E::Soulburn, E::Magiccrasher]),

        S::AmAcidterror => T::on_target(&[E::Throwitem]),
        S::AmBerserkpitcher => T::on_target(&[E::PotionBerserk]),
        S::AmCpWeapon | S::AmCpShield | S::AmCpArmor | S::AmCpHelm => {
            T::on_target(&[E::Chemicalprotection])
        }

        S::TkDownkick => T::on_target(&[E::Pressedbody, E::Hitline6]),
        S::TkTurnkick => T::on_target(&[E::Spinedbody2, E::Hitline4]),
        S::TkCounter => T::on_target(&[E::Kickedbody]),
        S::TkJumpkick => T::on_target(&[E::Chemical3, E::Quakebody2]),
        S::SlStin => T {
            on_target: &[E::Stin, E::Quakebody3],
            hit: &[E::BlueHit],
            ..Default::default()
        },
        S::SlStun => T {
            on_target: &[E::Stin3, E::Hitline4],
            hit: &[E::BlueHit],
            ..Default::default()
        },
        S::SlSma => T::on_target(&[E::Ef4waybody, E::Stin2, E::Hitline6, E::Hittexture]),
        S::SgHate => T::on_target(&[E::Hated]),

        S::SlKaahi => T::on_target(&[E::Hated]),
        S::SlKaupe => T::on_target(&[E::Bluebody]),
        S::SlKaizel => T::on_target(&[E::Hated, E::Kaizel]),
        S::SlKaite => T::on_target(&[E::Reflectbody, E::Bluebody]),
        S::CgMarionette => T::on_target(&[E::Pinkbody]),
        S::SlSwoo => T::on_target(&[E::Babybody, E::M07]),
        S::SlSke => T::on_target(&[E::AsurabodyMonster]),
        S::SlSka => T::on_target(&[E::Steelbody, E::Gumgang2]),
        S::SlAlchemist
        | S::SlMonk
        | S::SlStar
        | S::SlSage
        | S::SlCrusader
        | S::SlSupernovice
        | S::SlKnight
        | S::SlWizard
        | S::SlPriest
        | S::SlBarddancer
        | S::SlRogue
        | S::SlAssasin
        | S::SlBlacksmith
        | S::SlHunter
        | S::SlSoullinker
        | S::SlHigh
        | S::SlDeathknight
        | S::SlCollector
        | S::SlNinja
        | S::SlGunner => T::on_target(&[E::Soullink, E::Asurabody]),

        S::GsPiercingshot => T::on_target(&[E::Chemical4]),
        S::NjSyuriken => T::on_target(&[E::Throwitem7]),
        S::NjKunai => T::on_target(&[E::Throwitem8]),
        S::NjHuuma => T::on_target(&[E::Throwitem9]),
        S::NjZenynage => T::on_target(&[E::Throwitem10]),
        S::NjHuujin => T::on_target(&[E::Stin4]),
        S::NjKamaitachi => T::on_target(&[E::Stin5]),
        S::GsFling => T::on_target(&[E::RedHit]),
        S::GsFullbuster => T::on_target(&[E::M02]),
        S::GsSpreadattack => T::on_target(&[E::Spreadattack]),
        S::GsDust => T::on_target(&[E::Bash3d5]),
        S::GsRapidshower => T::on_target(&[E::Rapidshower]),
        S::GsMagicalbullet => T::on_target(&[E::Magicalbullet]),
        S::GsTracking => T::on_target(&[E::Tracking]),
        S::GsTripleaction => T::on_target(&[E::Tripleaction]),
        S::GsBullseye => T::on_target(&[E::Bullseye]),
        S::GsDisarm => T::on_target(&[E::RgCoin3]),
        S::NjKouenka => T {
            on_target: &[E::Kouenka],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::NjHyousensou => T {
            on_target: &[E::Hyousensou],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::NjKirikage => T::on_target(&[E::Kirikage]),
        S::NjKasumikiri => T::on_target(&[E::Kasumikiri]),
        S::NjIssen => T::on_target(&[E::Issen]),
        S::NjBakuenryu => T::on_target(&[E::Baku]),

        S::HfliMoon => T::on_target(&[E::Hflimoon1]),
        S::HfliSbr44 => T::on_target(&[E::Hflimoon3, E::Ef4waybody]),
        S::WeFemale => T::on_target(&[E::Absorbspirits]),
        S::WeMale => T::on_target(&[E::Heal]),
        S::HlifHeal => T::on_target(&[E::Heal4]),
        S::WeBaby => T::on_target(&[E::Baby]),
        S::AllResurrection => T::on_target(&[E::Resurrection, E::Revive]),
        S::AllPartyflee => T::on_target(&[E::Flowerleaf]),

        S::NpcStop => T::on_target(&[E::NpcStop]),
        S::NpcWeaponbraker => T::on_target(&[E::Stripweapon]),

        _ => T::default(),
    }
}

fn suppresses_generic_hit(skill: SkillEnum) -> bool {
    let skill = merc_skill_base(skill);
    use SkillEnum as S;
    matches!(
        skill,
        S::TkDownkick
            | S::TkTurnkick
            | S::TkCounter
            | S::TkJumpkick
            | S::TkStormkick
            | S::TkSevenwind
            | S::TkMission
            | S::SlSma
    )
}

pub fn derive_hit_effect(
    skill: Option<SkillEnum>,
    is_crit: bool,
    attacker_job: JobName,
    target_is_self: bool,
) -> &'static [EffectId] {
    if target_is_self {
        return &[];
    }
    match skill {
        None if is_crit => &[EffectId::Hit2, EffectId::Hit1],
        None if attacker_job.is_taekwon() => &[EffectId::Hitline7],
        None => &[EffectId::Hit1],
        Some(s) => {
            let hit = target_skill_effects(s).hit;
            if !hit.is_empty() {
                hit
            } else if suppresses_generic_hit(s) {
                &[]
            } else {
                &[EffectId::Hit1]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bowling_bash_splits_cast_onto_caster_and_hit_onto_target() {
        let caster = caster_skill_effects(SkillEnum::KnBowlingbash);
        assert_eq!(caster.cast, &[EffectId::Bowlingself]);
        let casting = casting_skill(SkillEnum::KnBowlingbash);
        assert!(casting.hide_cast_bar && casting.hide_cast_aura);
        assert_eq!(
            target_skill_effects(SkillEnum::KnBowlingbash).hit,
            &[EffectId::Bowlingbash]
        );
    }

    #[test]
    fn same_slot_stack_keeps_every_effect() {
        assert_eq!(
            caster_skill_effects(SkillEnum::MoExplosionspirits).cast,
            &[EffectId::Gumgang, EffectId::Gumgang2]
        );
    }

    #[test]
    fn mercenary_skills_inherit_base_skill_visuals() {
        use SkillEnum as S;
        for (merc, base) in [
            (S::MsMagnum, S::SmMagnum),
            (S::MaDouble, S::AcDouble),
            (S::MlSpiralpierce, S::LkSpiralpierce),
            (S::MerMagnificat, S::PrMagnificat),
        ] {
            assert_eq!(merc_skill_base(merc), base);
            assert_eq!(caster_skill_effects(merc), caster_skill_effects(base));
            assert_eq!(target_skill_effects(merc), target_skill_effects(base));
        }
        assert_eq!(
            caster_skill_effects(S::MsMagnum).cast,
            &[EffectId::Magnumbreak]
        );
        // Support buffs have no dedicated effect in the original game.
        assert_eq!(merc_skill_base(S::MerQuicken), S::MerQuicken);
        assert!(caster_skill_effects(S::MerQuicken).cast.is_empty());
        assert_eq!(merc_skill_base_id(S::MaDouble.id() as u16), S::AcDouble.id() as u16);
        assert_eq!(merc_skill_base_id(S::SmBash.id() as u16), S::SmBash.id() as u16);
    }

    #[test]
    fn npc_monster_skills_launch_on_correct_actor() {
        use EffectId as E;
        use SkillEnum as S;
        for (skill, effect) in [
            (S::NpcChangewater, E::Changecold),
            (S::NpcChangefire, E::Changefire),
            (S::NpcSelfdestruction, E::SuiExplosion),
            (S::NpcSummonslave, E::Summonslave),
            (S::NpcPowerup, E::Gumgangnpc),
        ] {
            assert_eq!(caster_skill_effects(skill).cast, &[effect], "{skill:?}");
            assert!(target_skill_effects(skill).on_target.is_empty(), "{skill:?}");
        }
        for (skill, effect) in [
            (S::NpcStop, E::NpcStop),
            (S::NpcWeaponbraker, E::Stripweapon),
            (S::HlifHeal, E::Heal4),
        ] {
            assert_eq!(target_skill_effects(skill).on_target, &[effect], "{skill:?}");
            assert!(caster_skill_effects(skill).cast.is_empty(), "{skill:?}");
        }
    }

    /// A one-shot cast/grant effect must play once and stop — a finite,
    /// non-repeating [`EffectSpec`] — so it flashes rather than lingering as an
    /// aura (which is the persistent status path's job).
    fn plays_once(id: EffectId) -> bool {
        use crate::spec::EffectSpec::*;
        match crate::table::effect_spec(id) {
            Some(Str { duration_ms, repeat, .. } | Spr { duration_ms, repeat, .. }) => {
                !repeat && duration_ms < u32::MAX
            }
            Some(Custom { duration_ms }) => duration_ms < u32::MAX,
            Some(SprBurst { duration_ms, .. }) => duration_ms < u32::MAX,
            Some(Noop) | None => true,
        }
    }

    #[test]
    fn auto_guard_and_parrying_launch_guard_effect() {
        use SkillEnum as S;
        for skill in [S::CrAutoguard, S::MlAutoguard, S::LkParrying, S::MsParrying] {
            assert_eq!(caster_skill_effects(skill).cast, &[EffectId::Guard], "{skill:?}");
            assert!(target_skill_effects(skill).on_target.is_empty(), "{skill:?}");
        }
    }

    #[test]
    fn one_shot_cast_buffs_flash_once_at_use() {
        use EffectId as E;
        use SkillEnum as S;
        let cases: &[(S, &[EffectId])] = &[
            (S::SmEndure, &[E::Endure]),
            (S::KnTwohandquicken, &[E::Twohandquicken]),
            (S::KnOnehand, &[E::Twohandquicken]),
            // A. previously wrongly-persistent self buffs — flashed once at cast.
            (S::CrReflectshield, &[E::Reflectshield]),
            (S::CrDefender, &[E::Defender]),
            (S::CrShrink, &[E::Shrink]),
            (S::BsAdrenaline, &[E::Hasteup]),
            (S::BsAdrenaline2, &[E::Hasteup]),
            (S::BsMaximize, &[E::Maxpower]),
            (S::McLoud, &[E::Loud]),
            (S::WsMeltdown, &[E::Meltdown]),
            (S::WsCartboost, &[E::Cartboost]),
            // D. split — the burst rides the cast, the persistent half the status.
            (S::LkAurablade, &[E::Aurablade, E::Aurablade2]),
            // E. genuinely-persistent buffs also flash their body once at cast.
            (S::MgEnergycoat, &[E::Energycoat]),
            (S::BsOverthrust, &[E::Overthrust]),
            (S::WsOverthrustmax, &[E::Overthrust]),
            (S::CrSpearquicken, &[E::Spearquicken]),
            (S::MoSteelbody, &[E::Steelbody, E::Gumgang2]),
            // F. Star Gladiator comfort + Taekwon seven-wind.
            (S::SgSunComfort, &[E::Flowercast, E::Hated]),
            (S::TkSevenwind, &[E::Stormkick3, E::Beginasura1]),
        ];
        for &(skill, expected) in cases {
            assert_eq!(caster_skill_effects(skill).cast, expected, "{skill:?}");
            for &id in expected {
                assert!(plays_once(id), "{skill:?}: {id:?} must play once, not loop");
            }
        }
    }

    #[test]
    fn target_grant_buffs_flash_once_on_the_recipient() {
        use EffectId as E;
        use SkillEnum as S;
        let cases: &[(S, &[EffectId])] = &[
            // B. per-target status grants — flashed once on the recipient.
            (S::SlKaahi, &[E::Hated]),
            (S::SlKaupe, &[E::Bluebody]),
            (S::SlKaizel, &[E::Hated, E::Kaizel]),
            (S::CgMarionette, &[E::Pinkbody]),
            // D. split — the burst on cast, the persistent half on the status.
            (S::SlKaite, &[E::Reflectbody, E::Bluebody]),
            // E. Magic Power / Assumptio also flash their body once at cast.
            (S::HwMagicpower, &[E::Lightblade]),
            (S::HpAssumptio, &[E::Assumptio, E::Assumptio2]),
            // F. Soul Linker job links, transforms, and Splasher's poison.
            (S::SlHigh, &[E::Soullink, E::Asurabody]),
            (S::SlSwoo, &[E::Babybody, E::M07]),
            (S::SlSke, &[E::AsurabodyMonster]),
            (S::SlSka, &[E::Steelbody, E::Gumgang2]),
            (S::AsSplasher, &[E::Enchantpoison, E::Splasher]),
        ];
        for &(skill, expected) in cases {
            assert_eq!(target_skill_effects(skill).on_target, expected, "{skill:?}");
            for &id in expected {
                assert!(plays_once(id), "{skill:?}: {id:?} must play once, not loop");
            }
        }
        // Marionette's caster line + pink body flash on the caster too.
        assert_eq!(
            caster_skill_effects(S::CgMarionette).cast,
            &[E::Linelink3, E::Pinkbody]
        );
    }


    #[test]
    fn physical_attack_skills_share_the_bash_begin_effect() {
        for skill in [
            SkillEnum::AcDouble,
            SkillEnum::McMammonite,
            SkillEnum::HtPower,
            SkillEnum::HtPhantasmic,
        ] {
            assert_eq!(begin_cast_effect(skill), &[EffectId::Bash], "{skill:?}");
            assert!(
                caster_skill_effects(skill).cast.is_empty(),
                "{skill:?} has no cast-slot visual"
            );
        }
        // Meltdown has a real cast time: the default Bash cast glyph shows while
        // the bar fills, and the Meltdown body flashes once at use.
        assert_eq!(
            begin_cast_effect(SkillEnum::WsMeltdown),
            &[EffectId::Bash]
        );
        assert_eq!(
            caster_skill_effects(SkillEnum::WsMeltdown).cast,
            &[EffectId::Meltdown]
        );
    }

    #[test]
    fn begin_cast_circle_is_per_skill_neutral_colored_and_special() {
        assert_eq!(
            begin_cast_effect(SkillEnum::MgFirebolt),
            &[EffectId::Beginspell]
        );
        assert_eq!(
            begin_cast_effect(SkillEnum::SaElementwater),
            &[EffectId::Beginspell2]
        );
        assert_eq!(
            begin_cast_effect(SkillEnum::HpAssumptio),
            &[EffectId::Beginspell6]
        );
        assert_eq!(
            begin_cast_effect(SkillEnum::MoExtremityfist),
            &[EffectId::Beginasura]
        );
        assert!(begin_cast_effect(SkillEnum::MoBodyrelocation).is_empty());
        assert!(casting_skill(SkillEnum::KnBowlingbash).hide_cast_aura);
    }

    #[test]
    fn spiral_pierce_begin_is_a_body_flash_that_survives_a_hidden_cast_aura() {
        assert_eq!(
            begin_cast_effect(SkillEnum::LkSpiralpierce),
            &[EffectId::Piercebody]
        );
        assert!(casting_skill(SkillEnum::LkSpiralpierce).hide_cast_aura);
        assert!(
            !is_cast_circle(EffectId::Piercebody),
            "body flash is not a circle"
        );
        assert!(is_cast_circle(EffectId::Beginspell));
        assert!(is_cast_circle(EffectId::Beginspell6));
        assert!(is_cast_circle(EffectId::Bluecasting));
    }

    #[test]
    fn high_jump_leap_is_a_begin_effect_landing_is_a_cast_effect_with_no_cast_bar() {
        let casting = casting_skill(SkillEnum::TkHighjump);
        assert_eq!(casting.begin, &[EffectId::Jumpbody, EffectId::Peong]);
        assert!(casting.hide_cast_bar);
        assert_eq!(
            caster_skill_effects(SkillEnum::TkHighjump).cast,
            &[EffectId::Landbody]
        );
    }

    #[test]
    fn damage_skills_fire_their_execution_glyph_not_a_cast_circle() {
        assert_eq!(
            fire_glyph_effect(SkillEnum::KnBrandishspear),
            &[EffectId::Brandish2]
        );
        assert_eq!(
            fire_glyph_effect(SkillEnum::AcChargearrow),
            &[EffectId::Bash]
        );
        assert!(begin_cast_effect(SkillEnum::KnBrandishspear).is_empty());
        assert!(casting_skill(SkillEnum::KnBrandishspear).hide_cast_aura);
        assert!(fire_glyph_effect(SkillEnum::McMammonite).is_empty());
    }

    #[test]
    fn sight_and_ruwach_show_no_cast_flash_only_the_option_driven_aura() {
        // The original fires nothing at cast for these detect skills; the aura
        // rides the OPTION_SIGHT / OPTION_RUWACH state instead.
        assert!(caster_skill_effects(SkillEnum::MgSight).cast.is_empty());
        assert!(caster_skill_effects(SkillEnum::AlRuwach).cast.is_empty());
    }

    #[test]
    fn storm_kick_flashes_on_the_caster() {
        // The original's no-damage handler fires EF_STORMKICK on the caster even
        // though the skill is dispatched on the damage path (it also sends the
        // no-damage packet).
        assert_eq!(
            caster_skill_effects(SkillEnum::TkStormkick).cast,
            &[EffectId::Stormkick]
        );
    }

    #[test]
    fn champion_combo_skills_route_their_body_recolors() {
        assert_eq!(
            caster_skill_effects(SkillEnum::ChTigerfist).cast,
            &[EffectId::Bash3d2, EffectId::Gumgang3]
        );
        assert_eq!(
            target_skill_effects(SkillEnum::ChPalmstrike).on_target,
            &[EffectId::Hitline2]
        );
        assert_eq!(
            caster_skill_effects(SkillEnum::ChChaincrush).cast,
            &[EffectId::Gumgang3]
        );
        assert_eq!(
            target_skill_effects(SkillEnum::ChChaincrush).on_target,
            &[EffectId::Chemical2]
        );
    }

    #[test]
    fn neutral_begin_circle_recolors_by_skill_element() {
        assert_eq!(beginspell_for_element(3), EffectId::Beginspell3);
        assert_eq!(beginspell_for_element(1), EffectId::Beginspell2);
        assert_eq!(beginspell_for_element(4), EffectId::Beginspell5);
        assert_eq!(beginspell_for_element(0), EffectId::Beginspell);
        assert_eq!(beginspell_for_element(99), EffectId::Beginspell);
    }

    #[test]
    fn cast_glyphs_cover_the_cast_time_skills_missing_them() {
        use EffectId as E;
        use SkillEnum as S;
        assert_eq!(begin_cast_effect(S::GsBullseye), &[E::Bluecasting]);
        assert_eq!(begin_cast_effect(S::SlPriest), &[E::Bluecasting]);
        assert_eq!(begin_cast_effect(S::NjKouenka), &[E::Beginspell3]);
        assert_eq!(begin_cast_effect(S::NjHyousensou), &[E::Beginspell2]);
        assert_eq!(begin_cast_effect(S::NjHuujin), &[E::Beginspell5]);
        assert_eq!(begin_cast_effect(S::AlWarp), &[E::Beginspell]);
        assert_eq!(begin_cast_effect(S::KnChargeatk), &[E::Beginspell6]);
        assert_eq!(begin_cast_effect(S::HtBlitzbeat), &[E::Bash]);
        // Both summon-mines default to the Bash cast glyph (absent from the
        // original's begin switch); Cannibalize is the odd one out (Beginspell).
        assert_eq!(begin_cast_effect(S::AmDemonstration), &[E::Bash]);
        assert_eq!(begin_cast_effect(S::AmSpheremine), &[E::Bash]);
        assert_eq!(begin_cast_effect(S::AmCannibalize), &[E::Beginspell]);
        // Musical Strike shares Throw Arrow's Bash cast glyph (both absent from
        // the begin switch, both 1.5s cast).
        assert_eq!(begin_cast_effect(S::BaMusicalstrike), &[E::Bash]);
        assert_eq!(begin_cast_effect(S::DcThrowarrow), &[E::Bash]);
        assert_eq!(begin_cast_effect(S::AmTwilight2), &[E::Twilight2]);
    }

    #[test]
    fn wedding_skills_cast_couple_and_heart_circles() {
        use EffectId as E;
        use SkillEnum as S;
        assert_eq!(begin_cast_effect(S::WeMale), &[E::Couplecasting]);
        assert_eq!(begin_cast_effect(S::WeFemale), &[E::Heartcasting]);
        assert_eq!(caster_skill_effects(S::WeCallpartner).cast, &[E::Couplecasting]);
        assert!(is_cast_circle(E::Couplecasting));
        assert!(is_cast_circle(E::Heartcasting));
        assert_eq!(target_skill_effects(S::WeMale).on_target, &[E::Heal]);
        assert_eq!(target_skill_effects(S::WeFemale).on_target, &[E::Absorbspirits]);
    }

    #[test]
    fn actor_swap_specials_render_on__position() {
        use EffectId as E;
        use SkillEnum as S;

        for (skill, effect) in [
            (S::SnSharpshooting, E::Tripleattack2),
            (S::CgArrowvulcan, E::Tripleattack3),
            (S::AsGrimtooth, E::Grimtooth),
            (S::AscBreaker, E::Soulbreaker),
            (S::CrShieldboomerang, E::Shieldboomerang),
            (S::MoFingeroffensive, E::Tanji),
            (S::RgBackstap, E::Backstap),
            (S::RgStealcoin, E::Stealcoin),
            (S::GsPiercingshot, E::Chemical4),
            (S::NjSyuriken, E::Throwitem7),
            (S::NjHuuma, E::Throwitem9),
            (S::NjHuujin, E::Stin4),
            (S::SlStin, E::Stin),
            (S::SlStun, E::Stin3),
            (S::SlSma, E::Stin2),
            (S::TkJumpkick, E::Chemical3),
        ] {
            assert!(
                target_skill_effects(skill).on_target.contains(&effect),
                "{skill:?}: {effect:?} must render on the target"
            );
            assert!(
                !caster_skill_effects(skill).cast.contains(&effect),
                "{skill:?}: {effect:?} must not stay on the caster"
            );
        }

        assert!(
            caster_skill_effects(S::ChChaincrush)
                .cast
                .contains(&E::Gumgang3)
        );
        assert!(
            !target_skill_effects(S::ChChaincrush)
                .on_target
                .contains(&E::Gumgang3)
        );
        assert!(
            target_skill_effects(S::ChChaincrush)
                .on_target
                .contains(&E::Chemical2)
        );

        assert!(
            caster_skill_effects(S::TkJumpkick)
                .cast
                .contains(&E::Jumpkick)
        );
    }
    #[test]
    fn assassin_cross_meteor_assault_and_steal() {
        use EffectId as E;
        use SkillEnum as S;

        assert_eq!(begin_cast_effect(S::AscMeteorassault), &[E::Beginspell7]);
        assert!(caster_skill_effects(S::AscMeteorassault).cast.contains(&E::Soulbreaker2));
        assert!(target_skill_effects(S::AscMeteorassault).on_target.is_empty());

        assert!(target_skill_effects(S::TfSteal).on_target.contains(&E::Steal));
        assert!(caster_skill_effects(S::TfSteal).cast.is_empty());
    }

    #[test]
    fn unmapped_skill_fires_nothing_on_either_actor() {
        assert_eq!(
            caster_skill_effects(SkillEnum::MoBodyrelocation),
            CasterSkillEffects::default()
        );
        assert_eq!(
            target_skill_effects(SkillEnum::MoBodyrelocation),
            TargetSkillEffects::default()
        );
    }

    #[test]
    fn ground_cast_classifier_matches_skill_db_target_type() {
        for s in [
            SkillEnum::MgThunderstorm,
            SkillEnum::WzStormgust,
            SkillEnum::WzMeteor,
            SkillEnum::PrSanctuary,
            SkillEnum::HtAnklesnare,
            SkillEnum::SaLandprotector,
        ] {
            assert!(is_ground_cast(s), "{s:?} is TargetType:Ground");
        }
        for s in [
            SkillEnum::MgColdbolt,
            SkillEnum::WzWaterball,
            SkillEnum::WzEarthspike,
            SkillEnum::AlHeal,
        ] {
            assert!(!is_ground_cast(s), "{s:?} targets an entity");
        }
    }

    #[test]
    fn ground_placed_effect_renders_aoe_at_cell_not_caster() {
        use EffectId as E;
        assert_eq!(
            ground_placed_effect(SkillEnum::WzStormgust, 10),
            &[E::Stormgust]
        );
        assert_eq!(
            ground_placed_effect(SkillEnum::WzMeteor, 10),
            &[E::Meteorstorm]
        );
        assert_eq!(ground_placed_effect(SkillEnum::WzVermilion, 10), &[E::Lord]);
        assert_eq!(
            ground_placed_effect(SkillEnum::MgThunderstorm, 10),
            &[E::Thunderstorm]
        );
        assert_eq!(
            ground_placed_effect(SkillEnum::CrSlimpitcher, 1),
            &[E::Slim]
        );
        assert_eq!(
            ground_placed_effect(SkillEnum::CrSlimpitcher, 10),
            &[E::Slim3]
        );
        assert!(ground_placed_effect(SkillEnum::SaVolcano, 5).is_empty());
        assert!(ground_placed_effect(SkillEnum::WzIcewall, 5).is_empty());
    }

    #[test]
    fn hit_derivation_follows_attack_type_then_skill_table() {
        use JobName::{Novice, Taekwon};
        assert_eq!(
            derive_hit_effect(None, false, Novice, false),
            &[EffectId::Hit1]
        );
        assert_eq!(
            derive_hit_effect(None, true, Novice, false),
            &[EffectId::Hit2, EffectId::Hit1]
        );
        assert_eq!(
            derive_hit_effect(None, false, Taekwon, false),
            &[EffectId::Hitline7]
        );
        // Self-target suppresses the spark.
        assert_eq!(
            derive_hit_effect(None, true, Novice, true),
            &[] as &[EffectId]
        );
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MgColdbolt), false, Novice, false),
            &[EffectId::Coldhit]
        );
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MoBodyrelocation), false, Novice, false),
            &[EffectId::Hit1]
        );
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::TkStormkick), false, Novice, false),
            &[] as &[EffectId]
        );
    }

    #[test]
    fn damage_skill_slots_drive_projectile_landing_and_cast() {
        let soulstrike = target_skill_effects(SkillEnum::MgSoulstrike);
        assert_eq!(soulstrike.before_hit, &[EffectId::Soulstrike]);
        assert!(soulstrike.on_target.is_empty() && soulstrike.hit.is_empty());

        assert_eq!(
            target_skill_effects(SkillEnum::MgNapalmbeat).on_target,
            &[EffectId::Hit2]
        );

        let coldbolt = target_skill_effects(SkillEnum::MgColdbolt);
        assert_eq!(coldbolt.on_target, &[EffectId::Icearrow]);
        assert_eq!(coldbolt.hit, &[EffectId::Coldhit]);
        assert!(coldbolt.before_hit.is_empty());

        assert_eq!(
            caster_skill_effects(SkillEnum::WzStormgust).cast,
            &[EffectId::Stormgust]
        );
    }

    #[test]
    fn target_effect_ids_match_the_original() {
        // Living target gets the green heal (the undead/demon damage path fires
        // Heal3 separately, wired in the attack-effect handler).
        assert_eq!(
            target_skill_effects(SkillEnum::AlHeal).on_target,
            &[EffectId::Heal]
        );

        let frostdiver = target_skill_effects(SkillEnum::MgFrostdiver);
        assert_eq!(frostdiver.before_hit, &[EffectId::Frostdiver]);
        assert_eq!(frostdiver.on_target, &[EffectId::Frostdiver2]);
        assert_eq!(frostdiver.hit, &[EffectId::Coldhit]);

        assert_eq!(
            target_skill_effects(SkillEnum::LkSpiralpierce).on_target,
            &[EffectId::Magnum2]
        );

        // Acid Demonstration lobs a molotov bottle (Throwitem4), distinct from
        // Acid Terror's plain bottles (Throwitem), then bursts on the target.
        assert_eq!(
            target_skill_effects(SkillEnum::CrAciddemonstration).on_target,
            &[EffectId::Throwitem4, EffectId::Aciddemon]
        );
        assert_eq!(
            target_skill_effects(SkillEnum::AmAcidterror).on_target,
            &[EffectId::Throwitem]
        );
    }

    #[test]
    fn caster_launched_at_target_effects_land_on_the_target() {
        for (skill, effect) in [
            (SkillEnum::GsFullbuster, EffectId::M02),
            (SkillEnum::GsSpreadattack, EffectId::Spreadattack),
            (SkillEnum::AsSonicblow, EffectId::Sonicblow),
            (SkillEnum::KnSpearstab, EffectId::Spearstabself),
        ] {
            assert!(
                target_skill_effects(skill).on_target.contains(&effect),
                "{skill:?}"
            );
            assert!(
                !caster_skill_effects(skill).cast.contains(&effect),
                "{skill:?}"
            );
        }
        assert!(
            caster_skill_effects(SkillEnum::AsSonicblow)
                .cast
                .contains(&EffectId::Sonicblow2)
        );
    }

    #[test]
    fn turn_undead_burst_anchors_on_the_caster() {
        let tu = SkillEnum::PrTurnundead;
        assert!(
            caster_skill_effects(tu)
                .cast
                .contains(&EffectId::Turnundead)
        );
        assert!(
            !target_skill_effects(tu)
                .on_target
                .contains(&EffectId::Turnundead)
        );
        assert_eq!(target_skill_effects(tu).hit, &[EffectId::Holyhit]);
    }

    #[test]
    fn on_target_projectiles_are_classified_as_trails() {
        use crate::effect_queue::is_trail_effect;
        for skill in [
            SkillEnum::MgFireball,
            SkillEnum::WzWaterball,
            SkillEnum::WzJupitel,
            SkillEnum::KnSpearboomerang,
            SkillEnum::AlDemonbane,
        ] {
            for e in target_skill_effects(skill).on_target {
                assert!(
                    is_trail_effect(*e),
                    "{e:?} on {skill:?} must be a trail effect"
                );
            }
        }
        assert!(!is_trail_effect(EffectId::Icearrow));
    }

    #[test]
    fn every_reachable_skill_projectile_has_a_reach_time() {
        use crate::effect_queue::{is_trail_effect, trail_arrival_secs};
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let mut missing = std::collections::BTreeSet::new();
        for id in 1u32..=745 {
            let Ok(skill) = std::panic::catch_unwind(|| SkillEnum::from_id(id)) else {
                continue;
            };
            let t = target_skill_effects(skill);
            for e in caster_skill_effects(skill)
                .cast
                .iter()
                .chain(t.on_target.iter())
                .chain(t.before_hit.iter())
            {
                if is_trail_effect(*e) && trail_arrival_secs(*e, 50.0).is_none() {
                    missing.insert(format!("{e:?}"));
                }
            }
        }
        std::panic::set_hook(prev);
        assert!(
            missing.is_empty(),
            "trail projectiles in a skill slot with no reach time: {missing:?}"
        );
    }
}
