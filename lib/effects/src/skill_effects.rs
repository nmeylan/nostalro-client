//! Per-skill effect tables — the multi-slot replacement for the single-id
//! `EffectId::from_skill` (which is scrambled and unused).
//!
//! A single skill fires **several** effects at different moments and on
//! **different actors**: a cast glyph on the caster, an optional projectile,
//! and a separate hit spark on the target. To keep "who does this play on"
//! unambiguous, the data is split into two tables keyed by actor:
//!
//! * [`caster_skill_effects`] — what plays on the **casting** entity
//!   (begin-cast glyph, cast effect, cast-bar/aura suppression).
//! * [`target_skill_effects`] — what plays on the **target** entity
//!   (the spell landing, the per-hit spark, a pre-hit projectile).
//!
//! Each slot holds a list because some skills stack effects there (e.g. Steel
//! Body launches both `EF_STEELBODY` and the `EF_GUMGANG2` shockwave on the
//! caster). A skill appears only in the table(s) where it has effects;
//! everything else returns the empty default, so nothing fires — deliberately
//! conservative while wiring proceeds. See `docs/client-plan/effects-wiring.md`
//! §2c (the slot model) and §2d (the source data + hit-effect derivation).

use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::skill_enums::SkillEnum;

/// Effects a skill plays on the **casting** entity, by packet moment. An empty
/// slot plays nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CasterSkillEffects {
    /// `ZC_USESKILL_ACK` — cast **starts**: the begin-spell / cast-bar glyph,
    /// often element-colored.
    pub begin_cast: &'static [EffectId],
    /// Skill **released** on the caster. For no-damage skills this is the
    /// `ZC_USE_SKILL` moment, for damage skills the `ZC_NOTIFY_SKILL` moment.
    pub cast: &'static [EffectId],
    /// Suppress the cast progress bar for this skill (Bowling Bash, Brandish).
    pub hide_cast_bar: bool,
    /// Suppress the elemental cast circle for this skill.
    pub hide_cast_aura: bool,
}

/// Effects a skill plays on the **target** entity, by packet moment. An empty
/// slot plays nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TargetSkillEffects {
    /// Effect that lands on the **recipient** at the spell moment
    /// (`target_on_spell` — e.g. Frost Diver's ice on the target).
    pub on_target: &'static [EffectId],
    /// Projectile fired before each hit (per hit).
    pub before_hit: &'static [EffectId],
    /// Per-damaging-hit spark on the **target** (`target_on_hit`). See
    /// [`derive_hit_effect`].
    pub hit: &'static [EffectId],
}

impl CasterSkillEffects {
    const fn cast(cast: &'static [EffectId]) -> Self {
        Self { begin_cast: &[], cast, hide_cast_bar: false, hide_cast_aura: false }
    }
}

impl TargetSkillEffects {
    const fn on_target(on_target: &'static [EffectId]) -> Self {
        Self { on_target, before_hit: &[], hit: &[] }
    }
    const fn hit(hit: &'static [EffectId]) -> Self {
        Self { on_target: &[], before_hit: &[], hit }
    }
}

/// Effects played on the **casting** entity for `skill`. Unmapped skills return
/// the empty default.
pub fn caster_skill_effects(skill: SkillEnum) -> CasterSkillEffects {
    use EffectId as E;
    use SkillEnum as S;
    type C = CasterSkillEffects;

    match skill {
        // --- Swordman / Mage / Acolyte / Merchant / Thief (first job) ----
        S::SmMagnum => C::cast(&[E::Magnumbreak]),
        S::SmEndure => C::cast(&[E::Endure]),
        S::MgEnergycoat => C::cast(&[E::Energycoat]),
        S::AlHolywater => C::cast(&[E::Aqua]),
        S::AlCrucis => C::cast(&[E::Signum]),
        S::AlAngelus => C::cast(&[E::Angelus]),
        S::McCartrevolution => C::cast(&[E::Cartrevolution]),
        S::McLoud => C::cast(&[E::Loud]),
        S::AcConcentration => C::cast(&[E::Concentration]),
        S::NvFirstaid => C::cast(&[E::Firstaid]),

        // --- Knight / Priest / Wizard / Blacksmith / Hunter / Assassin ---
        S::KnPierce => C::cast(&[E::Pierceself]),
        S::KnSpearboomerang => C::cast(&[E::Spearbmrself]),
        S::KnBowlingbash => C {
            cast: &[E::Bowlingself],
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::KnBrandishspear => C {
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::KnTwohandquicken | S::KnOnehand => C::cast(&[E::Twohandquicken]),
        S::PrTurnundead => C::cast(&[E::Turnundead]),
        S::PrMagnificat | S::MerMagnificat => C::cast(&[E::Magnificat]),
        S::PrGloria => C::cast(&[E::Gloria]),
        S::PrKyrie => C::cast(&[E::Kyrie]),
        S::WzFirepillar => C::cast(&[E::Firepillar]),
        S::WzSightrasher => C::cast(&[E::Sightrasher]),
        S::WzStormgust => C::cast(&[E::Stormgust]),
        S::BsMaximize => C::cast(&[E::Maxpower]),
        S::BsAdrenaline | S::BsAdrenaline2 => C::cast(&[E::Hasteup]),
        S::BsOverthrust | S::WsOverthrustmax => C::cast(&[E::Overthrust]),
        S::HtSpringtrap => C::cast(&[E::Springtrap]),
        S::HtRemovetrap => C::cast(&[E::Removetrap]),
        S::AsSonicblow => C::cast(&[E::Sonicblow]),
        S::AsGrimtooth => C::cast(&[E::Grimtooth]),

        // --- Monk combo (Steel Body / Explosion Spirits stack an aux ring) ---
        S::MoFingeroffensive => C::cast(&[E::Tanji]),
        S::MoAbsorbspirits => C::cast(&[E::Absorbspirits]),
        S::MoExplosionspirits => C::cast(&[E::Gumgang, E::Gumgang2]),
        S::MoSteelbody => C::cast(&[E::Steelbody, E::Gumgang2]),

        // --- Crusader / Paladin / Lord Knight / WS ------------------------
        S::CrGrandcross => C::cast(&[E::Grandcross]),
        S::CrShieldboomerang => C::cast(&[E::Shieldboomerang]),
        S::CrShrink => C::cast(&[E::Shrink]),
        S::CrDevotion => C::cast(&[E::Devotion]),
        S::CrSpearquicken => C::cast(&[E::Spearquicken]),
        S::CrReflectshield => C::cast(&[E::Reflectshield]),
        S::CrDefender | S::MlDefender => C::cast(&[E::Defender]),
        S::PaSacrifice => C::cast(&[E::Bash3d]),
        S::LkSpiralpierce => C {
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::LkHeadcrush => C::cast(&[E::Bash3d3]),
        S::LkJointbeat => C::cast(&[E::Bash3d4]),
        S::LkAurablade => C::cast(&[E::Aurablade, E::Aurablade2]),
        S::LkBerserk | S::LkFury | S::MsBerserk => C::cast(&[E::Redbody]),
        S::WsMeltdown => C::cast(&[E::Meltdown]),
        S::WsCartboost => C::cast(&[E::Cartboost]),

        // --- Rogue / Stalker ----------------------------------------------
        S::RgBackstap => C::cast(&[E::Backstap]),
        S::RgIntimidate => C::cast(&[E::Intimidate]),
        S::RgStealcoin => C::cast(&[E::Stealcoin]),
        S::RgRaid => C::cast(&[E::Teihit3]),
        S::StPreserve => C::cast(&[E::Guard2]),
        S::StRejectsword => C::cast(&[E::Rejectsword]),

        // --- Sniper / Gypsy-Clown -----------------------------------------
        S::SnSharpshooting => C::cast(&[E::Tripleattack2]),
        S::SnSight => C::cast(&[E::Truesight]),
        S::SnWindwalk => C::cast(&[E::Portal4]),
        S::CgArrowvulcan => C::cast(&[E::Tripleattack3]),
        S::CgLongingfreedom => C::cast(&[E::Chemicalbody]),
        S::CgMoonlit => C::cast(&[E::Spherewind2]),
        S::CgMarionette => C::cast(&[E::Pinkbody]),
        S::BaFrostjoker => C::cast(&[E::TalkFrostjoke]),
        S::BaPangvoice => C::cast(&[E::Fvoice]),
        S::DcScream => C::cast(&[E::TalkScream]),
        S::DcWinkcharm => C::cast(&[E::Wink]),

        // --- Sage / High Wizard / Professor -------------------------------
        S::SaMagicrod => C::cast(&[E::Magicrod]),
        S::HwSouldrain => C::cast(&[E::Energydrain2]),
        S::PfHpconversion => C::cast(&[E::Energydrain3]),
        S::PfSoulchange => C::cast(&[E::Linelink2, E::Linklight, E::Soulchange]),
        S::PfMemorize => C::cast(&[E::Memorize]),

        // --- Assassin Cross / Alchemist / Creator -------------------------
        S::AscBreaker => C::cast(&[E::Soulbreaker]),
        S::AscMeteorassault => C::cast(&[E::Soulbreaker2]),
        S::AscEdp => C::cast(&[E::Edp]),
        S::AmAcidterror => C::cast(&[E::Throwitem]),
        S::AmPotionpitcher => C::cast(&[E::Throwitem2]),
        S::AmBerserkpitcher => C::cast(&[E::Throwitem5]),
        S::ItmTomahawk => C::cast(&[E::Shieldboomerang2]),

        // --- Taekwon / Soul Linker / Star Gladiator -----------------------
        S::TkCounter => C::cast(&[E::Hitline5]),
        S::TkJumpkick => C::cast(&[E::Jumpkick, E::Chemical3]),
        S::TkRun => C::cast(&[E::Run]),
        S::TkHighjump => C::cast(&[E::Landbody]),
        S::TkStormkick => C::cast(&[E::Stormkick]),
        S::TkSevenwind => C::cast(&[E::Stormkick3, E::Beginasura1]),
        S::SlStin => C::cast(&[E::Stin]),
        S::SlStun => C::cast(&[E::Stin3]),
        S::SlSma => C::cast(&[E::Stin2]),
        S::SgSunWarm | S::SgMoonWarm | S::SgStarWarm => {
            C::cast(&[E::Doublegumgang, E::Redlightbody, E::Hated2])
        }
        S::SgSunComfort | S::SgMoonComfort | S::SgStarComfort => {
            C::cast(&[E::Flowercast, E::Hated])
        }

        // --- Gunslinger / Ninja -------------------------------------------
        S::GsPiercingshot => C::cast(&[E::Chemical4]),
        S::GsFullbuster => C::cast(&[E::M02]),
        S::GsSpreadattack => C::cast(&[E::Spreadattack]),
        S::GsMadnesscancel => C::cast(&[E::MadnessBlue]),
        S::GsAdjustment | S::GsGatlingfever => C::cast(&[E::MadnessRed]),
        S::GsIncreasing => C::cast(&[E::Agiup]),
        S::GsDesperado => C::cast(&[E::Desperado]),
        S::NjSyuriken => C::cast(&[E::Throwitem7]),
        S::NjKunai => C::cast(&[E::Throwitem8]),
        S::NjHuuma => C::cast(&[E::Throwitem9]),
        S::NjZenynage => C::cast(&[E::Throwitem10]),
        S::NjHuujin => C::cast(&[E::Stin4]),
        S::NjKamaitachi => C::cast(&[E::Stin5]),
        S::NjRaigekisai => C::cast(&[E::Thunderstorm2]),
        S::NjHyousyouraku => C::cast(&[E::Hyousyouraku]),

        // --- Homunculus / misc support ------------------------------------
        S::HfliSpeed | S::HfliFleet => C::cast(&[E::Homuncasting]),
        S::HlifChange => C::cast(&[E::Memorize]),
        S::HvanExplosion => C::cast(&[E::SuiExplosion]),
        S::HamiDefence => C::cast(&[E::Hamidefence]),
        S::HamiCastle => C::cast(&[E::Hamicastle]),
        S::HamiBloodlust => C::cast(&[E::Hamiblood]),
        S::HlifAvoid => C::cast(&[E::Agiup]),

        _ => C::default(),
    }
}

/// Effects played on the **target** entity for `skill`. Unmapped skills return
/// the empty default.
pub fn target_skill_effects(skill: SkillEnum) -> TargetSkillEffects {
    use EffectId as E;
    use SkillEnum as S;
    type T = TargetSkillEffects;

    match skill {
        // --- Swordman / Mage / Acolyte / Merchant / Thief (first job) ----
        S::SmMagnum => T::hit(&[E::Firehit]),
        S::SmProvoke => T::on_target(&[E::Provoke]),
        S::MgSoulstrike => T { before_hit: &[E::Soulstrike], ..Default::default() },
        S::MgFirebolt => T { on_target: &[E::Firearrow], hit: &[E::Firehit], ..Default::default() },
        S::MgFireball => T { on_target: &[E::Fireball], hit: &[E::Firehit], ..Default::default() },
        S::MgColdbolt => T { on_target: &[E::Icearrow], hit: &[E::Coldhit], ..Default::default() },
        S::MgLightningbolt => {
            T { on_target: &[E::Lightbolt], hit: &[E::Windhit], ..Default::default() }
        }
        S::MgFrostdiver => {
            T { on_target: &[E::Frostdiver], hit: &[E::Frostdiver2], ..Default::default() }
        }
        S::MgStonecurse => {
            T { on_target: &[E::Stonecurse], hit: &[E::Stonecurse], ..Default::default() }
        }
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

        // --- Knight / Priest / Wizard / Blacksmith / Hunter / Assassin ---
        S::KnPierce => T::hit(&[E::Pierce]),
        S::KnSpearstab => T { on_target: &[E::Spearstabself], hit: &[E::Pierce], ..Default::default() },
        S::KnSpearboomerang => {
            T { on_target: &[E::Spearbmr], hit: &[E::Hit4], ..Default::default() }
        }
        S::KnBowlingbash => T::hit(&[E::Bowlingbash]),
        S::KnBrandishspear => T::on_target(&[E::Brandishspear]),
        S::PrImpositio => T { on_target: &[E::Impositio], hit: &[E::Impositio], ..Default::default() },
        S::PrSuffragium => {
            T { on_target: &[E::Suffragium], hit: &[E::Suffragium], ..Default::default() }
        }
        S::PrAspersio => T { on_target: &[E::Aspersio], hit: &[E::Holyhit], ..Default::default() },
        S::PrTurnundead => T::hit(&[E::Holyhit]),
        S::PrMagnus => T::hit(&[E::Holyhit]),
        S::PrMagnificat | S::MerMagnificat => T::hit(&[E::Magnificat]),
        S::PrGloria => T::hit(&[E::Gloria]),
        S::PrLexdivina => T { on_target: &[E::Lexdivina], hit: &[E::Lexdivina], ..Default::default() },
        S::PrLexaeterna => {
            T { on_target: &[E::Lexaeterna], hit: &[E::Lexaeterna], ..Default::default() }
        }
        S::PrSlowpoison => T::on_target(&[E::Slowpoison]),
        S::PrStrecovery => T::on_target(&[E::Recovery]),
        S::WzFirepillar => T::hit(&[E::Firehit]),
        S::WzSightrasher => T::hit(&[E::Firehit]),
        S::WzJupitel => T { on_target: &[E::Yufitel], hit: &[E::Yufitelhit], ..Default::default() },
        S::WzStormgust => T::hit(&[E::Coldhit]),
        S::WzMeteor => T::hit(&[E::Firehit]),
        S::WzVermilion => T::hit(&[E::Windhit]),
        S::WzFrostnova => {
            T { on_target: &[E::Frostdiver2], hit: &[E::Frostdiver2], ..Default::default() }
        }
        S::WzEarthspike => T { on_target: &[E::Earthspike], hit: &[E::Earthhit], ..Default::default() },
        S::WzHeavendrive => T::hit(&[E::Earthhit]),
        S::WzQuagmire => T::hit(&[E::Earthhit]),
        S::WzWaterball => T::on_target(&[E::Waterball2]),
        S::WzEstimation => T::hit(&[E::Lockon]),
        S::BsRepairweapon => T::on_target(&[E::Repairweapon]),
        S::BsWeaponperfect => T::on_target(&[E::Perfection]),
        S::HtSkidtrap => T::hit(&[E::Bowlingbash]),
        S::HtBlitzbeat => T::hit(&[E::Blitzbeat]),
        S::AsSonicblow => T::hit(&[E::Sonicblowhit]),
        S::AsGrimtooth => T::hit(&[E::Grimtoothatk]),
        S::AsVenomdust => T::hit(&[E::Venomdust]),
        S::AsEnchantpoison => T::on_target(&[E::EnchantpoisonFlow]),
        S::AsPoisonreact => T::on_target(&[E::Poisonreact]),
        S::AsSplasher => T::on_target(&[E::Splasher]),
        S::AsVenomknife => T::on_target(&[E::Throwitem6]),

        // --- Monk combo ---------------------------------------------------
        S::MoChaincombo => T::hit(&[E::Sonicblowhit]),
        S::MoBalkyoung => T::hit(&[E::Hit3]),
        S::MoExtremityfist => T::on_target(&[E::Teihit1x]),
        S::MoTripleattack => T::on_target(&[E::Tripleattack]),
        S::MoInvestigate => T::on_target(&[E::Teihit2, E::Chimto]),

        // --- Crusader / Paladin / Lord Knight / WS ------------------------
        S::CrHolycross => T::on_target(&[E::Holycross]),
        S::CrShieldcharge => T::on_target(&[E::Shieldcharge]),
        S::CrProvidence => T::on_target(&[E::Providence]),
        S::CrFullprotection => T::on_target(&[E::Chemicalprotection, E::Chemicalbody]),
        S::PaShieldchain => T::on_target(&[E::Shieldboomerang3]),
        S::PaPressure => T::on_target(&[E::Pressure]),
        S::LkSpiralpierce => T { on_target: &[E::Pierceself], hit: &[E::Pierce], ..Default::default() },
        S::WsCarttermination => T::on_target(&[E::Cartter]),

        // --- Rogue / Stalker ----------------------------------------------
        S::RgStealcoin => T::on_target(&[E::RgCoin]),
        S::RgStripweapon => T::on_target(&[E::Stripweapon]),
        S::RgStripshield => T::on_target(&[E::Stripshield]),
        S::RgStriparmor => T::on_target(&[E::Striparmor]),
        S::RgStriphelm => T::on_target(&[E::Striphelm]),
        S::RgCloseconfine => T::on_target(&[E::Quakebody4]),
        S::StFullstrip => T::on_target(&[E::RgCoin2]),

        // --- Sniper / Gypsy-Clown -----------------------------------------
        // The original shows the generic HIT1 ring alongside the Blitz-beat
        // spark (its `atkedEfId` stays the default EF_HIT1).
        S::SnFalconassault => {
            T { on_target: &[E::Falconassault], hit: &[E::Hit1, E::Blitzbeat], ..Default::default() }
        }
        S::CgTarotcard => T::on_target(&[E::Chemicalbody]),

        // --- Sage / High Wizard / Professor -------------------------------
        S::SaSpellbreaker => T::on_target(&[E::Spellbreaker]),
        S::SaDispell => T::on_target(&[E::Dispell]),
        S::SaFlamelauncher | S::SaElementfire => T::on_target(&[E::Flamelauncher]),
        S::SaFrostweapon | S::SaElementwater => T::on_target(&[E::Frostweapon]),
        S::SaLightningloader | S::SaElementwind => T::on_target(&[E::Lightningloader]),
        S::SaSeismicweapon | S::SaElementground => T::on_target(&[E::Seismicweapon]),
        S::HwMagiccrasher => T::on_target(&[E::Magiccrasher]),
        S::HwNapalmvulcan => T::on_target(&[E::Napalmvalcan]),
        S::HwSouldrain => T::on_target(&[E::Transbluebody]),
        S::HwMagicpower => T::on_target(&[E::Lightblade]),
        S::PfSoulchange => T::on_target(&[E::Linklight, E::Soulchange]),
        S::PfDoublecasting => T::on_target(&[E::Doublecastbody]),
        S::PfSoulburn => T::on_target(&[E::Soulburn, E::Magiccrasher]),

        // --- Assassin Cross / Alchemist / Creator -------------------------
        S::AmBerserkpitcher => T::on_target(&[E::PotionBerserk]),
        S::AmCpWeapon | S::AmCpShield | S::AmCpArmor | S::AmCpHelm => {
            T::on_target(&[E::Chemicalprotection])
        }

        // --- Taekwon / Soul Linker / Star Gladiator -----------------------
        S::TkDownkick => T::on_target(&[E::Pressedbody, E::Hitline6]),
        S::TkTurnkick => T::on_target(&[E::Spinedbody2, E::Hitline4]),
        S::TkCounter => T::on_target(&[E::Kickedbody]),
        S::TkJumpkick => T::on_target(&[E::Quakebody2]),
        S::SlStin => T { on_target: &[E::Quakebody3], hit: &[E::BlueHit], ..Default::default() },
        S::SlStun => T { on_target: &[E::Hitline4], hit: &[E::BlueHit], ..Default::default() },
        S::SlSma => T::on_target(&[E::Ef4waybody, E::Hitline6, E::Hittexture]),
        S::SlSwoo => T::on_target(&[E::Babybody, E::M07]),
        S::SlSke => T::on_target(&[E::AsurabodyMonster]),
        S::SlSka => T::on_target(&[E::Steelbody, E::Gumgang2]),
        S::SlKaizel => T::on_target(&[E::Hated, E::Kaizel]),
        S::SlKaahi | S::SgHate => T::on_target(&[E::Hated]),
        S::SlKaupe => T::on_target(&[E::Bluebody]),
        S::SlKaite => T::on_target(&[E::Reflectbody, E::Bluebody]),

        // --- Gunslinger / Ninja -------------------------------------------
        S::GsFling => T::on_target(&[E::RedHit]),
        S::GsDust => T::on_target(&[E::Bash3d5]),
        S::GsRapidshower => T::on_target(&[E::Rapidshower]),
        S::GsMagicalbullet => T::on_target(&[E::Magicalbullet]),
        S::GsTracking => T::on_target(&[E::Tracking]),
        S::GsTripleaction => T::on_target(&[E::Tripleaction]),
        S::GsBullseye => T::on_target(&[E::Bullseye]),
        S::GsDisarm => T::on_target(&[E::RgCoin3]),
        S::NjKouenka => T { on_target: &[E::Kouenka], hit: &[E::Firehit], ..Default::default() },
        S::NjHyousensou => T { on_target: &[E::Hyousensou], hit: &[E::Coldhit], ..Default::default() },
        S::NjKirikage => T::on_target(&[E::Kirikage]),
        S::NjKasumikiri => T::on_target(&[E::Kasumikiri]),
        S::NjIssen => T::on_target(&[E::Issen]),
        S::NjBakuenryu => T::on_target(&[E::Baku]),

        // --- Homunculus / misc support ------------------------------------
        S::HfliMoon => T::on_target(&[E::Hflimoon1]),
        S::HfliSbr44 => T::on_target(&[E::Hflimoon3, E::Ef4waybody]),
        S::WeFemale => T::on_target(&[E::Absorbspirits]),
        S::WeBaby => T::on_target(&[E::Baby]),
        S::AllResurrection => T { on_target: &[E::Resurrection], hit: &[E::Revive], ..Default::default() },
        S::AllPartyflee => T::on_target(&[E::Flowerleaf]),

        _ => T::default(),
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
/// The original game fires the on-target spark from a two-slot record (a
/// generic spark **and** a per-skill spark), so two effects can play together:
///
/// * `skill == None` ⇒ a **normal melee attack**: the base spark `EF_HIT1`
///   (`EF_HITLINE7` for a bare-hand Taekwon-class attacker) always, **plus**
///   `EF_HIT2` on a critical — a crit shows both.
/// * `skill == Some(_)` ⇒ the skill's per-skill `hit` slot if it has one;
///   else, for a plain physical skill (Bash, Double Strafe, …), the generic
///   pair `EF_HIT1 + EF_HIT2` — the original shows the HIT1 ring/sparkle
///   alongside the HIT2 flower. Self-visual skills (they draw their own attack
///   effect) fire nothing. Elemental and signature skills keep only their own
///   `hit` because the original replaces/suppresses the generic HIT1 for them.
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
                &[EffectId::Hit1, EffectId::Hit2]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bowling_bash_splits_cast_onto_caster_and_hit_onto_target() {
        // The canonical two-actor case (§2c): a cast effect + hidden cast bar
        // on the caster, and a separate hit effect on the target.
        let caster = caster_skill_effects(SkillEnum::KnBowlingbash);
        assert_eq!(caster.cast, &[EffectId::Bowlingself]);
        assert!(caster.hide_cast_bar && caster.hide_cast_aura);
        assert_eq!(
            target_skill_effects(SkillEnum::KnBowlingbash).hit,
            &[EffectId::Bowlingbash]
        );
    }

    #[test]
    fn same_slot_stack_keeps_every_effect() {
        // Steel Body launches the body buff AND its shockwave on the caster —
        // both must survive (the old single-slot model dropped the aux).
        assert_eq!(
            caster_skill_effects(SkillEnum::MoSteelbody).cast,
            &[EffectId::Steelbody, EffectId::Gumgang2]
        );
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
    fn hit_derivation_follows_attack_type_then_skill_table() {
        use JobName::{Novice, Taekwon};
        // Normal attack: crit > taekwon-class bare hand > plain.
        assert_eq!(derive_hit_effect(None, false, Novice, false), &[EffectId::Hit1]);
        // A critical normal attack shows the generic HIT1 spark *and* HIT2.
        assert_eq!(
            derive_hit_effect(None, true, Novice, false),
            &[EffectId::Hit2, EffectId::Hit1]
        );
        assert_eq!(derive_hit_effect(None, false, Taekwon, false), &[EffectId::Hitline7]);
        // Self-target suppresses the spark.
        assert_eq!(derive_hit_effect(None, true, Novice, true), &[] as &[EffectId]);
        // A mapped skill uses its own spark; Cold Bolt is the WATER spark.
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MgColdbolt), false, Novice, false),
            &[EffectId::Coldhit]
        );
        // An unmapped (plain physical) damage skill falls back to the generic
        // pair: the HIT1 ring/sparkle plus the HIT2 flower (as Double Strafe
        // shows in the original game).
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::MoBodyrelocation), false, Novice, false),
            &[EffectId::Hit1, EffectId::Hit2]
        );
        // A self-visual skill (Taekwon kick) suppresses the generic spark.
        assert_eq!(
            derive_hit_effect(Some(SkillEnum::TkStormkick), false, Novice, false),
            &[] as &[EffectId]
        );
    }
}
