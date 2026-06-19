//! Per-skill effect slot table — the multi-slot replacement for the single-id
//! `EffectId::from_skill` (which is scrambled and unused).
//!
//! A single skill fires **several** effects at different moments and on
//! different actors: a cast glyph on the caster, an optional projectile, and a
//! separate hit spark on the target. Some skills also stack two effects in one
//! slot (a primary plus an auxiliary shockwave/ring/body tint). The original
//! game stores this as a per-skill slot table; this is our port. See
//! `docs/client-plan/effects-wiring.md` §2c (the slot model) and §2d (the
//! source data + the hit-effect derivation rules).
//!
//! Unmapped skills return [`SkillEffects::default`] (every slot empty), so
//! nothing fires for them — deliberately conservative while wiring proceeds.

use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;

/// The effects a skill plays in each of its distinct slots. Each slot fires at a
/// different packet/moment ; an empty slot plays nothing. A slot holds a
/// list because some skills stack effects there (e.g. Steel Body launches both
/// `EF_STEELBODY` and the `EF_GUMGANG2` shockwave on the caster).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillEffects {
    /// `ZC_USESKILL_ACK` — cast **starts**; on the caster (the begin-spell /
    /// cast-bar glyph, often element-colored).
    pub begin_cast: &'static [EffectId],
    /// Skill **released**; on the caster. For no-damage skills this is the
    /// `ZC_USE_SKILL` moment, for damage skills the `ZC_NOTIFY_SKILL` moment.
    pub cast: &'static [EffectId],
    /// Effect that lands on the **recipient** at the spell moment
    /// (`target_on_spell` — e.g. Frost Diver's ice on the target).
    pub on_target: &'static [EffectId],
    /// Projectile fired before each hit (per hit).
    pub before_hit: &'static [EffectId],
    /// Per-damaging-hit spark on the **target** (`target_on_hit`). See
    /// [`derive_hit_effect`].
    pub hit: &'static [EffectId],
    /// Non-damage skills that succeed by chance (Provoke, Sight).
    pub success: &'static [EffectId],
    /// Per ground-unit cell. Mostly superseded by the `unit_id → EffectId`
    pub ground: &'static [EffectId],
    /// Suppress the cast progress bar for this skill (Bowling Bash, Brandish).
    pub hide_cast_bar: bool,
    /// Suppress the elemental cast circle for this skill.
    pub hide_cast_aura: bool,
}

impl SkillEffects {
    fn cast(cast: &'static [EffectId]) -> Self {
        Self { cast, ..Default::default() }
    }
    fn on_target(on_target: &'static [EffectId]) -> Self {
        Self { on_target, ..Default::default() }
    }
}

/// The effect slots for `skill`. Unmapped skills return the empty default.
///
pub fn skill_effects(skill: SkillEnum) -> SkillEffects {
    use EffectId as E;
    use SkillEnum as S;

    match skill {
        // --- Swordman / Mage / Acolyte / Merchant / Thief (first job) ----
        S::SmMagnum => SkillEffects {
            cast: &[E::Magnumbreak],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::SmEndure => SkillEffects::cast(&[E::Endure]),
        S::SmProvoke => SkillEffects::on_target(&[E::Provoke]),
        S::MgSoulstrike => SkillEffects { before_hit: &[E::Soulstrike], ..Default::default() },
        S::MgFirebolt => SkillEffects {
            on_target: &[E::Firearrow],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::MgFireball => SkillEffects {
            on_target: &[E::Fireball],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::MgColdbolt => SkillEffects {
            on_target: &[E::Icearrow],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::MgLightningbolt => SkillEffects {
            on_target: &[E::Lightbolt],
            hit: &[E::Windhit],
            ..Default::default()
        },
        S::MgFrostdiver => SkillEffects {
            on_target: &[E::Frostdiver],
            hit: &[E::Frostdiver2],
            ..Default::default()
        },
        S::MgStonecurse => SkillEffects {
            on_target: &[E::Stonecurse],
            hit: &[E::Stonecurse],
            ..Default::default()
        },
        S::MgThunderstorm => SkillEffects::on_target(&[E::Thunderstorm]),
        S::MgEnergycoat => SkillEffects::cast(&[E::Energycoat]),
        S::AlDemonbane => SkillEffects::on_target(&[E::Tanji2]),
        S::AlHeal => SkillEffects::on_target(&[E::Heal]),
        S::AlHolylight => SkillEffects { hit: &[E::Holyhit], ..Default::default() },
        S::AlCure => SkillEffects::on_target(&[E::Cure]),
        S::AlIncagi | S::CashIncagi => SkillEffects::on_target(&[E::Incagility]),
        S::AlHolywater => SkillEffects::cast(&[E::Aqua]),
        S::AlCrucis => SkillEffects::cast(&[E::Signum]),
        S::AlAngelus => SkillEffects::cast(&[E::Angelus]),
        S::AlBlessing | S::CashBlessing => SkillEffects::on_target(&[E::Blessing]),
        S::McMammonite => SkillEffects { hit: &[E::Coin], ..Default::default() },
        S::McCartrevolution => SkillEffects {
            cast: &[E::Cartrevolution],
            hit: &[E::Cartrevolution],
            ..Default::default()
        },
        S::McLoud => SkillEffects::cast(&[E::Loud]),
        S::AcConcentration => SkillEffects::cast(&[E::Concentration]),
        S::TfSteal => SkillEffects::on_target(&[E::Steal]),
        S::TfDetoxify => SkillEffects::on_target(&[E::Detoxication]),
        S::TfSprinklesand => SkillEffects::on_target(&[E::Sprinklesand]),
        S::TfThrowstone => SkillEffects::on_target(&[E::Throwitem3]),
        S::NvFirstaid => SkillEffects::cast(&[E::Firstaid]),

        // --- Knight / Priest / Wizard / Blacksmith / Hunter / Assassin ---
        S::KnPierce => SkillEffects {
            cast: &[E::Pierceself],
            hit: &[E::Pierce],
            ..Default::default()
        },
        S::KnSpearstab => SkillEffects {
            on_target: &[E::Spearstabself],
            hit: &[E::Pierce],
            ..Default::default()
        },
        S::KnSpearboomerang => SkillEffects {
            cast: &[E::Spearbmrself],
            on_target: &[E::Spearbmr],
            hit: &[E::Hit4],
            ..Default::default()
        },
        S::KnBowlingbash => SkillEffects {
            cast: &[E::Bowlingself],
            hit: &[E::Bowlingbash],
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::KnBrandishspear => SkillEffects {
            on_target: &[E::Brandishspear],
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::KnTwohandquicken | S::KnOnehand => SkillEffects::cast(&[E::Twohandquicken]),
        S::PrImpositio => SkillEffects {
            on_target: &[E::Impositio],
            hit: &[E::Impositio],
            ..Default::default()
        },
        S::PrSuffragium => SkillEffects {
            on_target: &[E::Suffragium],
            hit: &[E::Suffragium],
            ..Default::default()
        },
        S::PrAspersio => SkillEffects {
            on_target: &[E::Aspersio],
            hit: &[E::Holyhit],
            ..Default::default()
        },
        S::PrTurnundead => SkillEffects {
            cast: &[E::Turnundead],
            hit: &[E::Holyhit],
            ..Default::default()
        },
        S::PrMagnus => SkillEffects { hit: &[E::Holyhit], ..Default::default() },
        S::PrMagnificat | S::MerMagnificat => SkillEffects {
            cast: &[E::Magnificat],
            hit: &[E::Magnificat],
            ..Default::default()
        },
        S::PrGloria => SkillEffects {
            cast: &[E::Gloria],
            hit: &[E::Gloria],
            ..Default::default()
        },
        S::PrLexdivina => SkillEffects {
            on_target: &[E::Lexdivina],
            hit: &[E::Lexdivina],
            ..Default::default()
        },
        S::PrLexaeterna => SkillEffects {
            on_target: &[E::Lexaeterna],
            hit: &[E::Lexaeterna],
            ..Default::default()
        },
        S::PrKyrie => SkillEffects::cast(&[E::Kyrie]),
        S::PrSlowpoison => SkillEffects::on_target(&[E::Slowpoison]),
        S::PrStrecovery => SkillEffects::on_target(&[E::Recovery]),
        S::WzFirepillar => SkillEffects {
            cast: &[E::Firepillar],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::WzSightrasher => SkillEffects {
            cast: &[E::Sightrasher],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::WzJupitel => SkillEffects {
            on_target: &[E::Yufitel],
            hit: &[E::Yufitelhit],
            ..Default::default()
        },
        S::WzStormgust => SkillEffects {
            cast: &[E::Stormgust],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::WzMeteor => SkillEffects { hit: &[E::Firehit], ..Default::default() },
        S::WzVermilion => SkillEffects { hit: &[E::Windhit], ..Default::default() },
        S::WzFrostnova => SkillEffects {
            on_target: &[E::Frostdiver2],
            hit: &[E::Frostdiver2],
            ..Default::default()
        },
        S::WzEarthspike => SkillEffects {
            on_target: &[E::Earthspike],
            hit: &[E::Earthhit],
            ..Default::default()
        },
        S::WzHeavendrive => SkillEffects { hit: &[E::Earthhit], ..Default::default() },
        S::WzQuagmire => SkillEffects { hit: &[E::Earthhit], ..Default::default() },
        S::WzWaterball => SkillEffects::on_target(&[E::Waterball2]),
        S::WzEstimation => SkillEffects { hit: &[E::Lockon], ..Default::default() },
        S::BsRepairweapon => SkillEffects::on_target(&[E::Repairweapon]),
        S::BsWeaponperfect => SkillEffects::on_target(&[E::Perfection]),
        S::BsMaximize => SkillEffects::cast(&[E::Maxpower]),
        S::BsAdrenaline | S::BsAdrenaline2 => SkillEffects::cast(&[E::Hasteup]),
        S::BsOverthrust | S::WsOverthrustmax => SkillEffects::cast(&[E::Overthrust]),
        S::HtSkidtrap => SkillEffects { hit: &[E::Bowlingbash], ..Default::default() },
        S::HtBlitzbeat => SkillEffects { hit: &[E::Blitzbeat], ..Default::default() },
        S::HtSpringtrap => SkillEffects::cast(&[E::Springtrap]),
        S::HtRemovetrap => SkillEffects::cast(&[E::Removetrap]),
        S::AsSonicblow => SkillEffects {
            cast: &[E::Sonicblow],
            hit: &[E::Sonicblowhit],
            ..Default::default()
        },
        S::AsGrimtooth => SkillEffects {
            cast: &[E::Grimtooth],
            hit: &[E::Grimtoothatk],
            ..Default::default()
        },
        S::AsVenomdust => SkillEffects { hit: &[E::Venomdust], ..Default::default() },
        S::AsEnchantpoison => SkillEffects::on_target(&[E::EnchantpoisonFlow]),
        S::AsPoisonreact => SkillEffects::on_target(&[E::Poisonreact]),
        S::AsSplasher => SkillEffects::on_target(&[E::Splasher]),
        S::AsVenomknife => SkillEffects::on_target(&[E::Throwitem6]),

        // --- Monk combo (Steel Body / Explosion Spirits stack an aux ring) ---
        S::MoFingeroffensive => SkillEffects::cast(&[E::Tanji]),
        S::MoChaincombo => SkillEffects { hit: &[E::Sonicblowhit], ..Default::default() },
        S::MoBalkyoung => SkillEffects { hit: &[E::Hit3], ..Default::default() },
        S::MoExtremityfist => SkillEffects::on_target(&[E::Teihit1x]),
        S::MoTripleattack => SkillEffects::on_target(&[E::Tripleattack]),
        S::MoInvestigate => SkillEffects::on_target(&[E::Teihit2, E::Chimto]),
        S::MoAbsorbspirits => SkillEffects::cast(&[E::Absorbspirits]),
        S::MoExplosionspirits => SkillEffects::cast(&[E::Gumgang, E::Gumgang2]),
        S::MoSteelbody => SkillEffects::cast(&[E::Steelbody, E::Gumgang2]),

        // --- Crusader / Paladin / Lord Knight / WS ------------------------
        S::CrGrandcross => SkillEffects::cast(&[E::Grandcross]),
        S::CrHolycross => SkillEffects::on_target(&[E::Holycross]),
        S::CrShieldcharge => SkillEffects::on_target(&[E::Shieldcharge]),
        S::CrShieldboomerang => SkillEffects::cast(&[E::Shieldboomerang]),
        S::CrShrink => SkillEffects::cast(&[E::Shrink]),
        S::CrProvidence => SkillEffects::on_target(&[E::Providence]),
        S::CrDevotion => SkillEffects::cast(&[E::Devotion]),
        S::CrSpearquicken => SkillEffects::cast(&[E::Spearquicken]),
        S::CrReflectshield => SkillEffects::cast(&[E::Reflectshield]),
        S::CrDefender | S::MlDefender => SkillEffects::cast(&[E::Defender]),
        S::CrFullprotection => SkillEffects::on_target(&[E::Chemicalprotection, E::Chemicalbody]),
        S::PaShieldchain => SkillEffects::on_target(&[E::Shieldboomerang3]),
        S::PaPressure => SkillEffects::on_target(&[E::Pressure]),
        S::PaSacrifice => SkillEffects::cast(&[E::Bash3d]),
        S::LkSpiralpierce => SkillEffects {
            on_target: &[E::Pierceself],
            hit: &[E::Pierce],
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::LkHeadcrush => SkillEffects::cast(&[E::Bash3d3]),
        S::LkJointbeat => SkillEffects::cast(&[E::Bash3d4]),
        S::LkAurablade => SkillEffects::cast(&[E::Aurablade, E::Aurablade2]),
        S::LkBerserk | S::LkFury | S::MsBerserk => SkillEffects::cast(&[E::Redbody]),
        S::WsCarttermination => SkillEffects::on_target(&[E::Cartter]),
        S::WsMeltdown => SkillEffects::cast(&[E::Meltdown]),
        S::WsCartboost => SkillEffects::cast(&[E::Cartboost]),

        // --- Rogue / Stalker ----------------------------------------------
        S::RgBackstap => SkillEffects::cast(&[E::Backstap]),
        S::RgIntimidate => SkillEffects::cast(&[E::Intimidate]),
        S::RgStealcoin => SkillEffects {
            cast: &[E::Stealcoin],
            on_target: &[E::RgCoin],
            ..Default::default()
        },
        S::RgRaid => SkillEffects::cast(&[E::Teihit3]),
        S::RgStripweapon => SkillEffects::on_target(&[E::Stripweapon]),
        S::RgStripshield => SkillEffects::on_target(&[E::Stripshield]),
        S::RgStriparmor => SkillEffects::on_target(&[E::Striparmor]),
        S::RgStriphelm => SkillEffects::on_target(&[E::Striphelm]),
        S::RgCloseconfine => SkillEffects::on_target(&[E::Quakebody4]),
        S::StFullstrip => SkillEffects::on_target(&[E::RgCoin2]),
        S::StPreserve => SkillEffects::cast(&[E::Guard2]),
        S::StRejectsword => SkillEffects::cast(&[E::Rejectsword]),

        // --- Sniper / Gypsy-Clown -----------------------------------------
        S::SnFalconassault => SkillEffects {
            on_target: &[E::Falconassault],
            hit: &[E::Blitzbeat],
            ..Default::default()
        },
        S::SnSharpshooting => SkillEffects::cast(&[E::Tripleattack2]),
        S::SnSight => SkillEffects::cast(&[E::Truesight]),
        S::SnWindwalk => SkillEffects::cast(&[E::Portal4]),
        S::CgArrowvulcan => SkillEffects::cast(&[E::Tripleattack3]),
        S::CgTarotcard => SkillEffects::on_target(&[E::Chemicalbody]),
        S::CgLongingfreedom => SkillEffects::cast(&[E::Chemicalbody]),
        S::CgMoonlit => SkillEffects::cast(&[E::Spherewind2]),
        S::CgMarionette => SkillEffects::cast(&[E::Pinkbody]),
        S::BaFrostjoker => SkillEffects::cast(&[E::TalkFrostjoke]),
        S::BaPangvoice => SkillEffects::cast(&[E::Fvoice]),
        S::DcScream => SkillEffects::cast(&[E::TalkScream]),
        S::DcWinkcharm => SkillEffects::cast(&[E::Wink]),

        // --- Sage / High Wizard / Professor -------------------------------
        S::SaSpellbreaker => SkillEffects::on_target(&[E::Spellbreaker]),
        S::SaDispell => SkillEffects::on_target(&[E::Dispell]),
        S::SaMagicrod => SkillEffects::cast(&[E::Magicrod]),
        S::SaFlamelauncher | S::SaElementfire => SkillEffects::on_target(&[E::Flamelauncher]),
        S::SaFrostweapon | S::SaElementwater => SkillEffects::on_target(&[E::Frostweapon]),
        S::SaLightningloader | S::SaElementwind => SkillEffects::on_target(&[E::Lightningloader]),
        S::SaSeismicweapon | S::SaElementground => SkillEffects::on_target(&[E::Seismicweapon]),
        S::HwMagiccrasher => SkillEffects::on_target(&[E::Magiccrasher]),
        S::HwNapalmvulcan => SkillEffects::on_target(&[E::Napalmvalcan]),
        S::HwSouldrain => SkillEffects {
            cast: &[E::Energydrain2],
            on_target: &[E::Transbluebody],
            ..Default::default()
        },
        S::HwMagicpower => SkillEffects::on_target(&[E::Lightblade]),
        S::PfHpconversion => SkillEffects::cast(&[E::Energydrain3]),
        S::PfSoulchange => SkillEffects {
            cast: &[E::Linelink2, E::Linklight, E::Soulchange],
            on_target: &[E::Linklight, E::Soulchange],
            ..Default::default()
        },
        S::PfDoublecasting => SkillEffects::on_target(&[E::Doublecastbody]),
        S::PfSoulburn => SkillEffects::on_target(&[E::Soulburn, E::Magiccrasher]),
        S::PfMemorize => SkillEffects::cast(&[E::Memorize]),

        // --- Assassin Cross / Alchemist / Creator -------------------------
        S::AscBreaker => SkillEffects::cast(&[E::Soulbreaker]),
        S::AscMeteorassault => SkillEffects::cast(&[E::Soulbreaker2]),
        S::AscEdp => SkillEffects::cast(&[E::Edp]),
        S::AmAcidterror => SkillEffects::cast(&[E::Throwitem]),
        S::AmPotionpitcher => SkillEffects::cast(&[E::Throwitem2]),
        S::AmBerserkpitcher => SkillEffects {
            cast: &[E::Throwitem5],
            on_target: &[E::PotionBerserk],
            ..Default::default()
        },
        S::AmCpWeapon | S::AmCpShield | S::AmCpArmor | S::AmCpHelm => {
            SkillEffects::on_target(&[E::Chemicalprotection])
        }
        S::ItmTomahawk => SkillEffects::cast(&[E::Shieldboomerang2]),

        // --- Taekwon / Soul Linker / Star Gladiator -----------------------
        S::TkDownkick => SkillEffects::on_target(&[E::Pressedbody, E::Hitline6]),
        S::TkTurnkick => SkillEffects::on_target(&[E::Spinedbody2, E::Hitline4]),
        S::TkCounter => SkillEffects {
            cast: &[E::Hitline5],
            on_target: &[E::Kickedbody],
            ..Default::default()
        },
        S::TkJumpkick => SkillEffects {
            cast: &[E::Jumpkick, E::Chemical3],
            on_target: &[E::Quakebody2],
            ..Default::default()
        },
        S::TkRun => SkillEffects::cast(&[E::Run]),
        S::TkHighjump => SkillEffects::cast(&[E::Landbody]),
        S::TkStormkick => SkillEffects::cast(&[E::Stormkick]),
        S::TkSevenwind => SkillEffects::cast(&[E::Stormkick3, E::Beginasura1]),
        S::SlStin => SkillEffects {
            cast: &[E::Stin],
            on_target: &[E::Quakebody3],
            hit: &[E::BlueHit],
            ..Default::default()
        },
        S::SlStun => SkillEffects {
            cast: &[E::Stin3],
            on_target: &[E::Hitline4],
            hit: &[E::BlueHit],
            ..Default::default()
        },
        S::SlSma => SkillEffects {
            cast: &[E::Stin2],
            on_target: &[E::Ef4waybody, E::Hitline6, E::Hittexture],
            ..Default::default()
        },
        S::SlSwoo => SkillEffects::on_target(&[E::Babybody, E::M07]),
        S::SlSke => SkillEffects::on_target(&[E::AsurabodyMonster]),
        S::SlSka => SkillEffects::on_target(&[E::Steelbody, E::Gumgang2]),
        S::SlKaizel => SkillEffects::on_target(&[E::Hated, E::Kaizel]),
        S::SlKaahi | S::SgHate => SkillEffects::on_target(&[E::Hated]),
        S::SlKaupe => SkillEffects::on_target(&[E::Bluebody]),
        S::SlKaite => SkillEffects::on_target(&[E::Reflectbody, E::Bluebody]),
        S::SgSunWarm | S::SgMoonWarm | S::SgStarWarm => {
            SkillEffects::cast(&[E::Doublegumgang, E::Redlightbody, E::Hated2])
        }
        S::SgSunComfort | S::SgMoonComfort | S::SgStarComfort => {
            SkillEffects::cast(&[E::Flowercast, E::Hated])
        }

        // --- Gunslinger / Ninja -------------------------------------------
        S::GsFling => SkillEffects::on_target(&[E::RedHit]),
        S::GsPiercingshot => SkillEffects::cast(&[E::Chemical4]),
        S::GsDust => SkillEffects::on_target(&[E::Bash3d5]),
        S::GsFullbuster => SkillEffects::cast(&[E::M02]),
        S::GsRapidshower => SkillEffects::on_target(&[E::Rapidshower]),
        S::GsMagicalbullet => SkillEffects::on_target(&[E::Magicalbullet]),
        S::GsTracking => SkillEffects::on_target(&[E::Tracking]),
        S::GsTripleaction => SkillEffects::on_target(&[E::Tripleaction]),
        S::GsBullseye => SkillEffects::on_target(&[E::Bullseye]),
        S::GsSpreadattack => SkillEffects::cast(&[E::Spreadattack]),
        S::GsMadnesscancel => SkillEffects::cast(&[E::MadnessBlue]),
        S::GsAdjustment | S::GsGatlingfever => SkillEffects::cast(&[E::MadnessRed]),
        S::GsIncreasing => SkillEffects::cast(&[E::Agiup]),
        S::GsDisarm => SkillEffects::on_target(&[E::RgCoin3]),
        S::GsDesperado => SkillEffects::cast(&[E::Desperado]),
        S::NjKouenka => SkillEffects {
            on_target: &[E::Kouenka],
            hit: &[E::Firehit],
            ..Default::default()
        },
        S::NjHyousensou => SkillEffects {
            on_target: &[E::Hyousensou],
            hit: &[E::Coldhit],
            ..Default::default()
        },
        S::NjSyuriken => SkillEffects::cast(&[E::Throwitem7]),
        S::NjKunai => SkillEffects::cast(&[E::Throwitem8]),
        S::NjHuuma => SkillEffects::cast(&[E::Throwitem9]),
        S::NjZenynage => SkillEffects::cast(&[E::Throwitem10]),
        S::NjHuujin => SkillEffects::cast(&[E::Stin4]),
        S::NjKamaitachi => SkillEffects::cast(&[E::Stin5]),
        S::NjKirikage => SkillEffects::on_target(&[E::Kirikage]),
        S::NjKasumikiri => SkillEffects::on_target(&[E::Kasumikiri]),
        S::NjIssen => SkillEffects::on_target(&[E::Issen]),
        S::NjRaigekisai => SkillEffects::cast(&[E::Thunderstorm2]),
        S::NjBakuenryu => SkillEffects::on_target(&[E::Baku]),
        S::NjHyousyouraku => SkillEffects::cast(&[E::Hyousyouraku]),

        // --- Homunculus / misc support ------------------------------------
        S::HfliMoon => SkillEffects::on_target(&[E::Hflimoon1]),
        S::HfliSbr44 => SkillEffects::on_target(&[E::Hflimoon3, E::Ef4waybody]),
        S::HfliSpeed | S::HfliFleet => SkillEffects::cast(&[E::Homuncasting]),
        S::HlifChange => SkillEffects::cast(&[E::Memorize]),
        S::HvanExplosion => SkillEffects::cast(&[E::SuiExplosion]),
        S::HamiDefence => SkillEffects::cast(&[E::Hamidefence]),
        S::HamiCastle => SkillEffects::cast(&[E::Hamicastle]),
        S::HamiBloodlust => SkillEffects::cast(&[E::Hamiblood]),
        S::HlifAvoid => SkillEffects::cast(&[E::Agiup]),
        S::WeFemale => SkillEffects::on_target(&[E::Absorbspirits]),
        S::WeBaby => SkillEffects::on_target(&[E::Baby]),
        S::AllResurrection => SkillEffects {
            on_target: &[E::Resurrection],
            hit: &[E::Revive],
            ..Default::default()
        },
        S::AllPartyflee => SkillEffects::on_target(&[E::Flowerleaf]),

        _ => SkillEffects::default(),
    }
}

/// `true` if a damage skill suppresses the generic hit spark because it carries
/// its own slash/projectile visual (§2d's `-1` list). These skills draw their
/// kick/spear/strike effect, so the fallback `EF_HIT2` must not also fire. Only
/// the self-visual skills whose `hit` slot is empty need listing — skills with a
/// real `hit` already short-circuit in [`derive_hit_effect`].
fn suppresses_generic_hit(skill: SkillEnum) -> bool {
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

/// The on-hit spark(s), client-derived (§2d "How the HIT effect is chosen").
/// The damage packet carries **no** effect ids — the spark comes from the skill
/// id, attack type, and (for normal attacks) crit/Taekwon, never the wire.
///
/// * `skill == None` ⇒ a **normal melee attack**: `EF_HIT2` on a critical,
///   `EF_HITLINE7` for a bare-hand Taekwon-class attacker, else `EF_HIT1`.
/// * `skill == Some(_)` ⇒ the skill's per-skill `hit` slot if it has one; else
///   the generic `EF_HIT2` skill default, unless the skill is self-visual (it
///   draws its own attack effect) in which case nothing fires.
/// * `target_is_self` ⇒ the spark is suppressed entirely (the original `-2`).
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
        None if is_crit => &[EffectId::Hit2],
        None if attacker_job.is_taekwon() => &[EffectId::Hitline7],
        None => &[EffectId::Hit1],
        Some(s) => {
            let hit = skill_effects(s).hit;
            if !hit.is_empty() {
                hit
            } else if suppresses_generic_hit(s) {
                &[]
            } else {
                &[EffectId::Hit2]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bowling_bash_carries_cast_and_hit_in_distinct_slots() {
        // The canonical two-slot case (§2c): a cast effect on the caster and a
        // separate hit effect on the target, with the cast bar/aura suppressed.
        let fx = skill_effects(SkillEnum::KnBowlingbash);
        assert_eq!(fx.cast, &[EffectId::Bowlingself]);
        assert_eq!(fx.hit, &[EffectId::Bowlingbash]);
        assert!(fx.hide_cast_bar && fx.hide_cast_aura);
    }

    #[test]
    fn same_slot_stack_keeps_every_effect() {
        // Steel Body launches the body buff AND its shockwave on the caster —
        // both must survive (the old single-slot model dropped the aux).
        assert_eq!(
            skill_effects(SkillEnum::MoSteelbody).cast,
            &[EffectId::Steelbody, EffectId::Gumgang2]
        );
    }

    #[test]
    fn unmapped_skill_fires_nothing() {
        assert_eq!(skill_effects(SkillEnum::MoBodyrelocation), SkillEffects::default());
    }

    #[test]
    fn hit_derivation_follows_attack_type_then_skill_table() {
        use JobName::{Novice, Taekwon};
        // Normal attack: crit > taekwon-class bare hand > plain.
        assert_eq!(derive_hit_effect(None, false, Novice, false), &[EffectId::Hit1]);
        assert_eq!(derive_hit_effect(None, true, Novice, false), &[EffectId::Hit2]);
        assert_eq!(derive_hit_effect(None, false, Taekwon, false), &[EffectId::Hitline7]);
        // Self-target suppresses the spark.
        assert_eq!(derive_hit_effect(None, true, Novice, true), &[] as &[EffectId]);
        // A mapped skill uses its own spark; Cold Bolt is the WATER spark.
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MgColdbolt), false, Novice, false),
            &[EffectId::Coldhit]
        );
        // An unmapped damage skill falls back to the generic EF_HIT2.
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MoBodyrelocation), false, Novice, false),
            &[EffectId::Hit2]
        );
        // A self-visual skill (Taekwon kick) suppresses the generic spark.
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::TkStormkick), false, Novice, false),
            &[] as &[EffectId]
        );
    }
}
