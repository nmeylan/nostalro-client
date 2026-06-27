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
    /// Skill **released** on the caster. For no-damage skills this is the
    /// `ZC_USE_SKILL` moment, for damage skills the `ZC_NOTIFY_SKILL` moment.
    pub cast: &'static [EffectId],
    /// Suppress the cast progress bar for this skill (Bowling Bash, Brandish).
    pub hide_cast_bar: bool,
    /// Suppress the begin-spell cast circle for this skill.
    pub hide_cast_aura: bool,
}

/// Effects a skill plays on the **target** entity, by packet moment. An empty
/// slot plays nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TargetSkillEffects {
    /// Effect that lands on the **recipient** at the spell moment
    /// (`target_on_spell` — e.g. Frost Diver's ice on the target).
    pub on_target: &'static [EffectId],
    /// Projectile fired before each hit (per hit).
    pub before_hit: &'static [EffectId],
    /// Per-damaging-hit spark on the **target** (`target_on_hit`). See
    /// [`derive_hit_effect`].
    pub hit: &'static [EffectId],
    /// Extra seconds added to the hit-spark / damage-number delay, beyond the
    /// normal projectile-flight delay. Used when the skill's visual impact
    /// happens after the damage packet arrives (e.g. Turn Undead's ring spawns
    /// 833 ms into the effect).
    pub hit_extra_delay_secs: f32,
}

impl CasterSkillEffects {
    const fn cast(cast: &'static [EffectId]) -> Self {
        Self {
            cast,
            hide_cast_bar: false,
            hide_cast_aura: false,
        }
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

/// The begin-spell / cast-start glyph shown on the caster at `ZC_USESKILL_ACK`
/// (the cast circle). A faithful transcription of the original game's per-skill
/// begin-effect switch — **not** derived from the skill element (the engine's
/// element parameter is unused; the colored `Beginspell2-7` variants are
/// assigned per skill). Offensive/support magic gets the neutral `Beginspell`;
/// Sage element skills, holy, and poison skills get colored variants; a handful
/// of skills get signature glyphs (Asura, blue-casting, couple-casting). An
/// unmapped skill shows nothing — conservative, like the rest of the table.
/// Ground-cast (cell-targeted) skills — `TargetType: Ground` in the skill
/// database (`INF_GROUND_SKILL`). Their `cast`/`on_target` effects are **placed
/// at the targeted cell** by `ZC_NOTIFY_GROUNDSKILL` (the original game's
/// `Am_Groundskill`), so the damage path (`spawn_skill_attack_effect`) must not
/// also stamp `on_target` on each hit entity — the original's damage handler
/// (`Am_Skill`) plays only the begin/special/hit effects, never `target_on_spell`.
/// Entity-targeted skills (the bolts, Waterball, Brandish, …) are absent here
/// and keep their damage-path landing visual.
///
/// Transcribed from rathena `db/pre-re/skill_db.yml` (`TargetType: Ground`),
/// limited to the skills present in our classic `SkillEnum`.
pub fn is_ground_cast(skill: SkillEnum) -> bool {
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

/// Effect **placed at the targeted cell** when a position-cast skill lands
/// (`ZC_NOTIFY_GROUNDSKILL`). Transcribed from the original game's
/// `Am_Groundskill` per-skill `PlaceEffect(cell, EF_*, …)` switch — the
/// authoritative cell placement, which is where these AoE visuals live (NOT the
/// caster `cast` slot; Storm Gust's storm, Meteor's strike and Lord of
/// Vermilion's field all render on the ground, not on the wizard).
///
/// Skills that create persistent units (Volcano, traps, Ice Wall, Quagmire,
/// Pneuma, Magnus, songs, Land Protector, …) render from their unit packets
/// (`skill_units.rs`) and are intentionally omitted here so they aren't placed
/// twice. `level` selects the Slim Pitcher tier, matching the original.
pub fn ground_placed_effect(skill: SkillEnum, level: i16) -> &'static [EffectId] {
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
        _ => &[],
    }
}

pub fn begin_cast_effect(skill: SkillEnum) -> &'static [EffectId] {
    use EffectId as E;
    use SkillEnum as S;
    match skill {
        // Generic skill-begin spark for physical attack skills with no
        // signature cast glyph (Double Strafe, Mammonite, …).
        S::AcDouble
        | S::McMammonite
        | S::HtPower
        | S::HtPhantasmic
        | S::HtBlitzbeat
        | S::SnFalconassault
        | S::AmSpheremine
        | S::DcThrowarrow => &[E::Bash],

        // Neutral begin-spell circle — offensive/support magic.
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
        | S::AmDemonstration
        | S::SaSpellbreaker
        | S::SaDispell => &[E::Beginspell],

        // Sage element-change skills — colored by their element. The Ninja
        // element spells share the same colored begin glyphs (water/fire/wind).
        S::SaElementwater | S::NjHyousensou | S::NjHyousyouraku | S::NjSuiton => &[E::Beginspell2],
        S::SaElementfire | S::NjKouenka | S::NjBakuenryu | S::NjKaensin => &[E::Beginspell3],
        S::SaElementground => &[E::Beginspell4],
        S::SaElementwind | S::NjHuujin | S::NjKamaitachi | S::NjRaigekisai => &[E::Beginspell5],

        // Holy (saint) begin circle.
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

        // Poison / dark begin circle.
        S::AsSplasher | S::StPreserve | S::AscBreaker | S::AscMeteorassault => &[E::Beginspell7],

        S::AlDemonbane => &[E::Beginspellwhite],
        S::AcConcentration => &[E::Incagidex],
        // Asura Strife — the trans-Monk `Beginasura11` variant is job-gated in
        // the original; we show the base glyph (the common, non-trans case).
        S::MoExtremityfist => &[E::Beginasura],

        // Couple-casting (homunculus calls, wedding skills).
        S::AmCallhomun | S::AmRest | S::AmResurrecthomun => &[E::Couplecasting],

        S::StChasewalk => &[E::Castspin],

        // Creator Twilight alchemy — its own potion-array cast glyph per tier.
        S::AmTwilight1 => &[E::Twilight1],
        S::AmTwilight2 => &[E::Twilight2],
        S::AmTwilight3 => &[E::Twilight3],

        // Spiral Pierce flashes the caster's body yellow at skill start (the
        // original's begin effect); the cast circle is hidden (see its
        // `hide_cast_aura`), but the body flash still plays.
        S::LkSpiralpierce => &[E::Piercebody],

        // Blue casting glyph — Taekwon run, Star-Gladiator feel/hate, Soul
        // Linker spirit skills.
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

/// Caster glyph played at the moment a damage skill executes, on the damage
/// packet — separate from the cast-bar glyph in [`begin_cast_effect`]. The
/// original game launches this from its damage handler only, so it applies to
/// attacking skills that resolve into a damage packet; instant ones therefore
/// still show it here even though they have no cast-bar glyph. Brandish Spear
/// hides its cast bar/aura yet still throws this burst on impact.
pub fn fire_glyph_effect(skill: SkillEnum) -> &'static [EffectId] {
    use EffectId as E;
    use SkillEnum as S;
    match skill {
        S::KnBrandishspear => &[E::Brandish2],
        S::AcChargearrow => &[E::Bash],
        _ => &[],
    }
}

/// Whether a begin-cast effect is the on-the-ground cast *circle* (the
/// `Beginspell` family + the blue/asura/couple casting glyphs), as opposed to a
/// caster body-flash like `Piercebody`. `hide_cast_aura` suppresses the circle
/// but must still let the body-flash play.
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
            | E::Castspin
            | E::Incagidex
            | E::Brandish2
    )
}

/// Element-colored begin-spell circle for the generic cast aura. Only the
/// neutral `Beginspell` is recolored by the skill's element (carried in the
/// cast packet); signature glyphs keep their own effect.
pub fn beginspell_for_element(property: u32) -> EffectId {
    match property {
        1 => EffectId::Beginspell2, // water
        3 => EffectId::Beginspell3, // fire
        2 => EffectId::Beginspell4, // earth
        4 => EffectId::Beginspell5, // wind
        6 => EffectId::Beginspell6, // holy
        5 => EffectId::Beginspell7, // poison
        _ => EffectId::Beginspell,  // neutral / unknown
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
        // Energy Coat's aura is a persistent status appearance (EFST → Energycoat,
        // status_buff.rs), not a cast flash — avoid the double-spawn.
        S::MgSight => C::cast(&[E::Sight]),
        S::AlRuwach => C::cast(&[E::Ruwach]),
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
        // The yellow body buff is the persistent EFST status (server-driven
        // duration), not a transient cast effect.
        S::KnTwohandquicken | S::KnOnehand => C::cast(&[]),
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
        S::AsSonicblow => C::cast(&[E::Sonicblow2]),
        // Grim Tooth's burst is posed at the target by the original game.

        // --- Monk combo (Steel Body / Explosion Spirits stack an aux ring) ---
        // Finger Offensive's needle burst is posed at the target by the original
        // game (see `target_skill_effects`), so nothing plays on the caster.
        S::MoAbsorbspirits => C::cast(&[E::Absorbspirits]),
        S::MoExplosionspirits => C::cast(&[E::Gumgang, E::Gumgang2]),
        // Keep only the shockwave burst; the steel aura is the persistent EFST status.
        S::MoSteelbody => C::cast(&[E::Gumgang2]),
        // Champion combo finishers — Tiger Fist flashes the caster (white body
        // light) + shockwave; Palm Strike / Chain Crush recolor the target
        // (see `target_skill_effects`).
        S::ChTigerfist => C::cast(&[E::Bash3d2, E::Gumgang3]),
        // Chain Crush bursts the caster with the shockwave; the target recolor
        // (Chemical2) lives in `target_skill_effects`.
        S::ChChaincrush => C::cast(&[E::Gumgang3]),

        // --- Crusader / Paladin / Lord Knight / WS ------------------------
        S::CrGrandcross => C::cast(&[E::Grandcross]),
        // The shield is thrown at the target by the original game (`target_skill_effects`).
        S::CrShrink => C::cast(&[E::Shrink]),
        S::CrDevotion => C::cast(&[E::Devotion]),
        // Persistent EFST status, like Two Hand Quicken.
        S::CrSpearquicken => C::cast(&[]),
        S::CrReflectshield => C::cast(&[E::Reflectshield]),
        S::CrDefender | S::MlDefender => C::cast(&[E::Defender]),
        S::CrAutoguard | S::MlAutoguard => C::cast(&[E::Guard]),
        S::PaSacrifice => C::cast(&[E::Bash3d]),
        S::LkSpiralpierce => C {
            hide_cast_bar: true,
            hide_cast_aura: true,
            ..Default::default()
        },
        S::LkHeadcrush => C::cast(&[E::Bash3d3]),
        S::LkJointbeat => C::cast(&[E::Bash3d4]),
        S::LkAurablade => C::cast(&[E::Aurablade, E::Aurablade2]),
        S::LkParrying | S::MsParrying => C::cast(&[E::Guard]),
        // Berserk's red body is a persistent status appearance (EFST → Redbody,
        // see status_buff.rs), not a transient cast flash. No caster cast effect.
        S::WsMeltdown => C::cast(&[E::Meltdown]),
        S::WsCartboost => C::cast(&[E::Cartboost]),

        // --- Rogue / Stalker ----------------------------------------------
        // Back Stab's slash is posed at the target by the original game.
        S::RgIntimidate => C::cast(&[E::Intimidate]),
        S::RgStealcoin => C::cast(&[E::Stealcoin]),
        S::RgRaid => C::cast(&[E::Teihit3]),
        S::StPreserve => C::cast(&[E::Guard2]),
        S::StRejectsword => C::cast(&[E::Rejectsword]),

        // --- Sniper / Gypsy-Clown -----------------------------------------
        // Sharp Shooting / Arrow Vulcan pose their volley burst at the target
        // (see `target_skill_effects`).
        S::SnSight => C::cast(&[E::Truesight]),
        S::SnWindwalk => C::cast(&[E::Portal4]),
        S::CgLongingfreedom => C::cast(&[E::Chemicalbody]),
        S::CgMoonlit => C::cast(&[E::Spherewind2]),
        // Marionette's pink body is a persistent EFST status (status_buff.rs), not a cast flash.
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
        // Soul Breaker's slash is posed at the target by the original game.
        S::AscMeteorassault => C::cast(&[E::Soulbreaker2]),
        S::AscEdp => C::cast(&[E::Edp]),
        S::AmAcidterror => C::cast(&[E::Throwitem]),
        S::AmPotionpitcher => C::cast(&[E::Throwitem2]),
        S::AmBerserkpitcher => C::cast(&[E::Throwitem5]),
        S::ItmTomahawk => C::cast(&[E::Shieldboomerang2]),

        // --- Taekwon / Soul Linker / Star Gladiator -----------------------
        S::TkCounter => C::cast(&[E::Hitline5]),
        // Jump Kick's body stays on the caster; its shockwave (Chemical3) is
        // posed at the target by the original game (`target_skill_effects`).
        S::TkJumpkick => C::cast(&[E::Jumpkick]),
        S::TkRun => C::cast(&[E::Run]),
        S::TkHighjump => C::cast(&[E::Landbody]),
        S::TkStormkick => C::cast(&[E::Stormkick]),
        S::TkSevenwind => C::cast(&[E::Stormkick3, E::Beginasura1]),
        // Stin / Stun pose their card burst at the target (`target_skill_effects`).
        S::SlSma => C::cast(&[E::Stin2]),
        S::SgSunWarm | S::SgMoonWarm | S::SgStarWarm => {
            C::cast(&[E::Doublegumgang, E::Redlightbody, E::Hated2])
        }
        S::SgSunComfort | S::SgMoonComfort | S::SgStarComfort => {
            C::cast(&[E::Flowercast, E::Hated])
        }

        // --- Gunslinger / Ninja -------------------------------------------
        // Piercing Shot's impact is posed at the target by the original game.
        S::GsMadnesscancel => C::cast(&[E::MadnessBlue]),
        S::GsAdjustment | S::GsGatlingfever => C::cast(&[E::MadnessRed]),
        S::GsIncreasing => C::cast(&[E::Agiup]),
        S::GsDesperado => C::cast(&[E::Desperado]),
        // The throwing-weapon and wind-blade bursts (Syuriken / Kunai / Huuma /
        // Zeny Nage / Huujin / Kamaitachi) are posed at the target by the
        // original game — see `target_skill_effects`.
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
        // Frost Diver's ice spikes travel the caster→target line (effect 27 is
        // a projectile trail, not an on-target burst), so it rides the
        // `before_hit` trail slot to get real endpoints; the freeze burst
        // (28) lands on the target.
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
        S::AlHeal => T::on_target(&[E::Heal3]),
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
            on_target: &[E::Turnundead],
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
        // Keep only the cast burst; the white glow is the persistent EFST status.
        S::HpAssumptio | S::CashAssumptio => T::on_target(&[E::Assumptio2]),
        S::WzFirepillar => T::hit(&[E::Firehit]),
        S::WzSightrasher => T::hit(&[E::Firehit]),
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
        S::HtBlitzbeat => T::hit(&[E::Blitzbeat]),
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
        S::AsSplasher => T::on_target(&[E::Splasher]),
        S::AsVenomknife => T::on_target(&[E::Throwitem6]),
        // Soul Breaker's slash is posed at the target by the original game.
        S::AscBreaker => T::on_target(&[E::Soulbreaker]),

        // --- Monk combo ---------------------------------------------------
        S::MoChaincombo => T::hit(&[E::Sonicblowhit]),
        S::MoBalkyoung => T::hit(&[E::Hit3]),
        S::MoExtremityfist => T::on_target(&[E::Teihit1x]),
        S::MoTripleattack => T::on_target(&[E::Tripleattack]),
        S::MoInvestigate => T::on_target(&[E::Teihit2, E::Chimto]),
        // Champion finishers: Palm Strike flashes the target orange (Hitline2),
        // Chain Crush bursts + recolors it pale yellow (Chemical2).
        S::ChPalmstrike => T::on_target(&[E::Hitline2]),
        // Chain Crush bursts the caster (Gumgang3, see `caster_skill_effects`)
        // and recolors the target pale yellow (Chemical2).
        S::ChChaincrush => T::on_target(&[E::Chemical2]),
        // Finger Offensive's needle burst is posed at the target.
        S::MoFingeroffensive => T::on_target(&[E::Tanji]),

        // --- Crusader / Paladin / Lord Knight / WS ------------------------
        S::CrHolycross => T::on_target(&[E::Holycross]),
        // Shield Boomerang is thrown at the target by the original game.
        S::CrShieldboomerang => T::on_target(&[E::Shieldboomerang]),
        S::CrAciddemonstration => T::on_target(&[E::Aciddemon]),
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

        // --- Rogue / Stalker ----------------------------------------------
        // Back Stab's slash is posed at the target by the original game.
        S::RgBackstap => T::on_target(&[E::Backstap]),
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
        S::SnFalconassault => T {
            on_target: &[E::Falconassault],
            hit: &[E::Hit1, E::Blitzbeat],
            ..Default::default()
        },
        S::CgTarotcard => T::on_target(&[E::Chemicalbody]),
        // Sharp Shooting / Arrow Vulcan pose their volley burst at the target.
        S::SnSharpshooting => T::on_target(&[E::Tripleattack2]),
        S::CgArrowvulcan => T::on_target(&[E::Tripleattack3]),

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
        // Jump Kick's shockwave (Chemical3) is posed at the target by the
        // original game; the impact body shake (Quakebody2) is also on the target.
        S::TkJumpkick => T::on_target(&[E::Chemical3, E::Quakebody2]),
        // The card burst (Stin / Stin3) is posed at the target by the original
        // game; the body shake stays on the target too.
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
        S::SlSma => T::on_target(&[E::Ef4waybody, E::Hitline6, E::Hittexture]),
        S::SlSwoo => T::on_target(&[E::Babybody, E::M07]),
        S::SlSke => T::on_target(&[E::AsurabodyMonster]),
        S::SlSka => T::on_target(&[E::Gumgang2]),
        S::SlKaizel => T::on_target(&[E::Hated, E::Kaizel]),
        S::SlKaahi | S::SgHate => T::on_target(&[E::Hated]),
        S::SlKaupe => T::on_target(&[E::Bluebody]),
        S::SlKaite => T::on_target(&[E::Reflectbody, E::Bluebody]),

        // --- Gunslinger / Ninja -------------------------------------------
        S::GsPiercingshot => T::on_target(&[E::Chemical4]),
        // The throwing-weapon and wind-blade bursts are posed at the target.
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

        // --- Homunculus / misc support ------------------------------------
        S::HfliMoon => T::on_target(&[E::Hflimoon1]),
        S::HfliSbr44 => T::on_target(&[E::Hflimoon3, E::Ef4waybody]),
        S::WeFemale => T::on_target(&[E::Absorbspirits]),
        S::WeBaby => T::on_target(&[E::Baby]),
        S::AllResurrection => T {
            on_target: &[E::Resurrection],
            hit: &[E::Revive],
            ..Default::default()
        },
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
        // Aura Blade launches two aura layers on the caster — both must
        // survive (the old single-slot model dropped the aux).
        assert_eq!(
            caster_skill_effects(SkillEnum::LkAurablade).cast,
            &[EffectId::Aurablade, EffectId::Aurablade2]
        );
    }

    #[test]
    fn physical_attack_skills_share_the_bash_begin_effect() {
        // Skills with no signature attack visual show the generic EF_BASH spark
        // on the caster, from the begin-cast circle (not the cast slot).
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
    }

    #[test]
    fn begin_cast_circle_is_per_skill_neutral_colored_and_special() {
        // The cast circle is chosen per skill, not by element: bolts get the
        // neutral circle, Sage element skills the colored ones, and signature
        // skills their own glyph. Skills that hide their cast aura suppress it
        // at the call site, so the slot data must still carry the flag.
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
        assert!(caster_skill_effects(SkillEnum::KnBowlingbash).hide_cast_aura);
    }

    #[test]
    fn spiral_pierce_begin_is_a_body_flash_that_survives_a_hidden_cast_aura() {
        // Spiral Pierce's begin effect is the yellow body flash, not a cast
        // circle, so it must NOT be classified as a circle (the call site keeps
        // firing it even though the skill hides its cast aura).
        assert_eq!(
            begin_cast_effect(SkillEnum::LkSpiralpierce),
            &[EffectId::Piercebody]
        );
        assert!(caster_skill_effects(SkillEnum::LkSpiralpierce).hide_cast_aura);
        assert!(
            !is_cast_circle(EffectId::Piercebody),
            "body flash is not a circle"
        );
        // The cast-circle family is still recognised so the aura gate suppresses it.
        assert!(is_cast_circle(EffectId::Beginspell));
        assert!(is_cast_circle(EffectId::Beginspell6));
        assert!(is_cast_circle(EffectId::Bluecasting));
    }

    #[test]
    fn damage_skills_fire_their_execution_glyph_not_a_cast_circle() {
        // Brandish Spear and Charge Arrow throw their caster glyph at the damage
        // moment (the execution path), not from the cast bar. Brandish hides its
        // cast bar/aura, so its glyph must NOT live in begin_cast_effect (which the
        // aura gate would suppress) — it belongs in the fire-glyph slot instead.
        assert_eq!(
            fire_glyph_effect(SkillEnum::KnBrandishspear),
            &[EffectId::Brandish2]
        );
        assert_eq!(
            fire_glyph_effect(SkillEnum::AcChargearrow),
            &[EffectId::Bash]
        );
        assert!(begin_cast_effect(SkillEnum::KnBrandishspear).is_empty());
        assert!(caster_skill_effects(SkillEnum::KnBrandishspear).hide_cast_aura);
        // Skills without an execution glyph keep the empty default.
        assert!(fire_glyph_effect(SkillEnum::McMammonite).is_empty());
    }

    #[test]
    fn champion_combo_skills_route_their_body_recolors() {
        // Tiger Fist flashes the caster (Bash3d2 white glow) + shockwave; Palm
        // Strike recolors the target. Chain Crush bursts the caster with the
        // shockwave (Gumgang3) and recolors the target (Chemical2).
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
        // The generic Beginspell is remapped to the element-colored ring at
        // cast start, so Fire Bolt (Fire) shows the red circle while neutral
        // and unknown elements keep the default.
        assert_eq!(beginspell_for_element(3), EffectId::Beginspell3); // fire
        assert_eq!(beginspell_for_element(1), EffectId::Beginspell2); // water
        assert_eq!(beginspell_for_element(4), EffectId::Beginspell5); // wind
        assert_eq!(beginspell_for_element(0), EffectId::Beginspell); // neutral
        assert_eq!(beginspell_for_element(99), EffectId::Beginspell); // unknown
    }

    #[test]
    fn cast_glyphs_cover_the_cast_time_skills_missing_them() {
        // Skills with a cast time show the original game's begin glyph during the
        // cast bar; these were previously unmapped (fired nothing). One per glyph
        // family: Bluecasting (Gunslinger/Soul Linker), the colored element
        // circles (Ninja element spells), the neutral/holy circles (Sage, CP),
        // Bash (Blitz Beat), and the Twilight tiers.
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
        assert_eq!(begin_cast_effect(S::AmTwilight2), &[E::Twilight2]);
    }

    #[test]
    fn actor_swap_specials_render_on__position() {
        // Pattern 3 (AM_SETPOS): the original game launches these specials by the
        // caster but poses them at the other actor. Each id must sit in the slot
        // for the actor it renders on — caster `cast` or target `on_target`.
        use EffectId as E;
        use SkillEnum as S;

        // caster → target moves: the special leaves the caster slot entirely and
        // lands on the target.
        for (skill, effect) in [
            (S::SnSharpshooting, E::Tripleattack2),
            (S::CgArrowvulcan, E::Tripleattack3),
            (S::AsGrimtooth, E::Grimtooth),
            (S::AscBreaker, E::Soulbreaker),
            (S::CrShieldboomerang, E::Shieldboomerang),
            (S::MoFingeroffensive, E::Tanji),
            (S::RgBackstap, E::Backstap),
            (S::GsPiercingshot, E::Chemical4),
            (S::NjSyuriken, E::Throwitem7),
            (S::NjHuuma, E::Throwitem9),
            (S::NjHuujin, E::Stin4),
            (S::SlStin, E::Stin),
            (S::SlStun, E::Stin3),
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

        // Chain Crush is the reverse move: the shockwave belongs on the caster,
        // and only the recolor stays on the target.
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

        // The caster-side companions of moved specials stay put.
        assert!(
            caster_skill_effects(S::TkJumpkick)
                .cast
                .contains(&E::Jumpkick)
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
    fn ground_cast_classifier_matches_skill_db_target_type() {
        // `is_ground_cast` follows the skill DB's `TargetType: Ground`, so the
        // damage path places cast/on_target at the cell (not per hit entity).
        // Cell-targeted AoE/units are ground-cast across magic/trap/unit kinds:
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
        // Entity-targeted spells keep their damage-path landing visual.
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
        // Pure AoE ground skills place their signature visual at the cell
        // (the original's `Am_Groundskill` switch), not the caster `cast` slot.
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
        // Slim Pitcher tiers by level.
        assert_eq!(
            ground_placed_effect(SkillEnum::CrSlimpitcher, 1),
            &[E::Slim]
        );
        assert_eq!(
            ground_placed_effect(SkillEnum::CrSlimpitcher, 10),
            &[E::Slim3]
        );
        // Unit skills render from their unit packets, not here.
        assert!(ground_placed_effect(SkillEnum::SaVolcano, 5).is_empty());
        assert!(ground_placed_effect(SkillEnum::WzIcewall, 5).is_empty());
    }

    #[test]
    fn hit_derivation_follows_attack_type_then_skill_table() {
        use JobName::{Novice, Taekwon};
        // Normal attack: crit > taekwon-class bare hand > plain.
        assert_eq!(
            derive_hit_effect(None, false, Novice, false),
            &[EffectId::Hit1]
        );
        // A critical normal attack shows the generic HIT1 spark *and* HIT2.
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

    #[test]
    fn damage_skill_slots_drive_projectile_landing_and_cast() {
        // The three damage-side slots the B5 fan-out fires at `ZC_NOTIFY_SKILL`:
        // a projectile (`before_hit`), a spell landing (`on_target`), and a
        // caster-released glyph (`cast`). Soul Strike is the projectile case and
        // must keep firing only via `before_hit` (no regression). Cold Bolt
        // lands on the target and sparks per hit. Storm Gust shows its glyph on
        // the caster.
        let soulstrike = target_skill_effects(SkillEnum::MgSoulstrike);
        assert_eq!(soulstrike.before_hit, &[EffectId::Soulstrike]);
        assert!(soulstrike.on_target.is_empty() && soulstrike.hit.is_empty());

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
        // Same-actor id corrections: the original launches a different target
        // effect than our earlier wiring did. Heal uses the Heal3 special;
        // Frost Diver keeps its projectile/freeze specials but sparks the
        // explicit Coldhit per hit; Spiral Pierce's target special is Magnum2.
        assert_eq!(
            target_skill_effects(SkillEnum::AlHeal).on_target,
            &[EffectId::Heal3]
        );

        let frostdiver = target_skill_effects(SkillEnum::MgFrostdiver);
        assert_eq!(frostdiver.before_hit, &[EffectId::Frostdiver]);
        assert_eq!(frostdiver.on_target, &[EffectId::Frostdiver2]);
        assert_eq!(frostdiver.hit, &[EffectId::Coldhit]);

        assert_eq!(
            target_skill_effects(SkillEnum::LkSpiralpierce).on_target,
            &[EffectId::Magnum2]
        );
    }

    #[test]
    fn caster_launched_at_target_effects_land_on_the_target() {
        // The original positions these at the target (`SETPOS(tActor)`), so they
        // must sit in the target `on_target` slot — a point effect left in the
        // caster `cast` slot collapses onto the caster instead.
        for (skill, effect) in [
            (SkillEnum::PrTurnundead, EffectId::Turnundead),
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
        // Sonic Blow's caster-side motion lines stay on the caster.
        assert!(
            caster_skill_effects(SkillEnum::AsSonicblow)
                .cast
                .contains(&EffectId::Sonicblow2)
        );
    }

    #[test]
    fn on_target_projectiles_are_classified_as_trails() {
        // Fireball, Waterball, Jupitel, Spear Boomerang and Demon Bane park a
        // travelling projectile in their `on_target` slot. The skill spawner
        // routes any `on_target` effect that `is_trail_effect` along the
        // caster→target line (same rule the viewer uses), so these must stay
        // classified as trails or they collapse onto the target in-game.
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
        // Cold Bolt's on_target bolt rains onto the target (a count-point
        // effect), so it must NOT be trail-routed.
        assert!(!is_trail_effect(EffectId::Icearrow));
    }

    #[test]
    fn every_reachable_skill_projectile_has_a_reach_time() {
        // Any trail effect parked in a skill slot that the spawner can launch as
        // a travelling projectile — the caster-released `cast`, `on_target` or
        // `before_hit` — must report a reach time (`trail_arrival_secs`) or the
        // hit fires on cast instead of on impact. Walk every skill the damage
        // path can resolve (`from_id` covers 1..=745 and panics outside it, the
        // same range the client can reach).
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
