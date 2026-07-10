use models::enums::class::JobName;
use models::enums::effect_id::EffectId;
use models::enums::element::Element;
use models::enums::weapon::WeaponType;

/// One wave the effect emits; `Randomized` picks `EF_Foo{n}.wav`, n in 1..=count.
#[derive(Clone, Copy, Debug)]
pub enum WaveChoice {
    Fixed(&'static str),
    Randomized { pattern: &'static str, count: u8 },
}

/// When, relative to effect spawn, a wave fires. Frames are at 60fps.
#[derive(Clone, Copy, Debug)]
pub enum SfxTiming {
    AtFrames(&'static [u16]),
    EveryFrames(u16),
    AtFrameChance { frame: u16, one_in: u8 },
}

#[derive(Clone, Copy, Debug)]
pub struct SfxCue {
    pub wave: WaveChoice,
    pub timing: SfxTiming,
}

pub type SfxSchedule = &'static [SfxCue];

macro_rules! at {
    ($wave:expr, $frames:expr) => {
        SfxCue {
            wave: $wave,
            timing: SfxTiming::AtFrames($frames),
        }
    };
}
macro_rules! fixed_at0 {
    ($name:literal) => {
        SfxCue {
            wave: WaveChoice::Fixed($name),
            timing: SfxTiming::AtFrames(&[0]),
        }
    };
}

pub fn effect_sound(id: EffectId) -> Option<SfxSchedule> {
    use EffectId as E;
    use SfxTiming::*;
    use WaveChoice::*;

    Some(match id {
        E::PharmacyOk => &[fixed_at0!("effect\\p_success.wav")],
        E::PharmacyFail => &[fixed_at0!("effect\\p_failed.wav")],
        E::Loud => &[fixed_at0!("effect\\makingnoise.wav")],
        E::Heartcasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Soulbreaker => &[at!(Fixed("effect\\soulBreaker.wav"), &[20])],
        E::Pressure => &[at!(Fixed("effect\\pressure.wav"), &[3])],
        E::Bash3d => &[at!(Fixed("effect\\bash3d.wav"), &[22])],
        E::Chemicalprotection => &[fixed_at0!("apocalips_attack.wav")],
        E::Mgdef1 => &[fixed_at0!("effect\\windwalk.wav")],
        E::Mgdef2 => &[fixed_at0!("effect\\windwalk.wav")],
        E::Mgdef3 => &[fixed_at0!("effect\\windwalk.wav")],
        E::Mgdef4 => &[fixed_at0!("effect\\windwalk.wav")],
        E::Levelup => &[fixed_at0!("levelup.wav")],
        E::Joblevelup => &[fixed_at0!("joblevelup.wav")],
        E::Criticalwound => &[at!(Fixed("effect\\wideb.wav"), &[1])],
        E::NpcSlowcast => &[at!(Fixed("effect\\EF_DecAgility.wav"), &[10])],
        E::NpcEarthquake => &[fixed_at0!("effect\\earth_quake.wav")],
        E::Issen => &[fixed_at0!("effect\\ilseom.wav")],
        E::Kasumikiri => &[fixed_at0!("effect\\mistslash.wav")],
        E::Kirikage => &[fixed_at0!("effect\\shadowslash.wav")],
        E::Damage1 => &[fixed_at0!("effect\\EF_hit2.wav")],
        E::Rapidshower => &[at!(Fixed("effect\\rapidshower.wav"), &[20])],
        E::Magicalbullet => &[fixed_at0!("effect\\magicalwavelet.wav")],
        E::Tripleaction => &[at!(Fixed("effect\\tripleaction.wav"), &[25])],
        E::Trackcasting => &[fixed_at0!("effect\\baku.wav")],
        E::Baku => &[fixed_at0!("effect\\baku.wav")],
        E::Hyousyouraku => &[fixed_at0!("effect\\hyousyouraku.wav")],
        E::Desperado => &[at!(Fixed("effect\\desperado.wav"), &[10])],
        E::Bash3d5 => &[fixed_at0!("effect\\bash3d5.wav")],
        E::RgCoin3 => &[at!(Fixed("effect\\disarm.wav"), &[21])],
        E::Stin5 => &[at!(Fixed("effect\\stin5.wav"), &[1])],
        E::Stin4 => &[fixed_at0!("effect\\stin4.wav")],
        E::CookingOk => &[at!(Fixed("_heal_effect.wav"), &[45])],
        E::CookingFail => &[at!(Fixed("caramel_die.wav"), &[50])],
        E::TempOk => &[fixed_at0!("_heal_effect.wav")],
        E::TempFail => &[fixed_at0!("caramel_die.wav")],
        E::Hated2 => &[at!(Fixed("effect\\warmspell.wav"), &[10])],
        E::Cartter => &[at!(Fixed("effect\\wizard_fire_pillar_b.wav"), &[30])],
        E::Chemical2dash => &[at!(Fixed("effect\\chemical2.wav"), &[44])],
        E::Memorize => &[fixed_at0!("effect\\priest_suffragium.wav")],
        E::Shrink => &[at!(Fixed("effect\\EF_BeginSpell.wav"), &[5])],
        E::Soullink => &[
            fixed_at0!("effect\\casting.wav"),
            at!(Fixed("effect\\soullinklight.wav"), &[145]),
        ],
        E::Castspin => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Piercebody => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::RgCoin2 => &[at!(Fixed("effect\\ez2stronly.wav"), &[1, 11, 21, 31])],
        E::Stin3 => &[fixed_at0!("effect\\energyrelease.wav")],
        E::Quakebody4 => &[at!(Fixed("effect\\ef_hit6.wav"), &[20])],
        E::Stin => &[at!(Fixed("effect\\magicwind.wav"), &[10])],
        E::Stin2 => &[at!(Fixed("effect\\flyingkick.wav"), &[5, 20, 35, 50, 65])],
        E::Hflimoon1 => &[fixed_at0!("effect\\h_moonlight_1.wav")],
        E::Hflimoon2 => &[fixed_at0!("effect\\h_moonlight_2.wav")],
        E::Hflimoon3 => &[fixed_at0!("effect\\h_moonlight_3.wav")],
        E::HoUp => &[fixed_at0!("levelup.wav")],
        E::Food06 => &[fixed_at0!("_heal_effect.wav")],
        E::M02 => &[at!(Fixed("effect\\pulsebeta.wav"), &[5])],
        E::Dragonfear => &[at!(Fixed("effect\\dragonfear.wav"), &[1])],
        E::Bleeding => &[at!(Fixed("effect\\wideb.wav"), &[1])],
        E::Absorbspirits => &[fixed_at0!("effect\\inspiration.wav")],
        E::Attackenergy2 => &[fixed_at0!("effect\\magicreflection.wav")],
        E::Napalmvalcan => &[at!(Fixed("effect\\EF_NapalmBeat.wav"), &[20, 30, 40, 50, 60])],
        E::Baby => &[fixed_at0!("effect\\EF_Blessing.wav")],
        E::Cartboost => &[fixed_at0!("effect\\EF_IncAgility.wav")],
        E::Rejectsword => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Flowercast => &[at!(Fixed("effect\\comfortable.wav"), &[5])],
        E::Stormkick => &[at!(Fixed("effect\\stormkick.wav"), &[5])],
        E::Stormkick4 => &[at!(Fixed("effect\\evasion1.wav"), &[15])],
        E::Stormkick5 => &[at!(Fixed("effect\\evasion1.wav"), &[15])],
        E::Electric => &[at!(Fixed("effect\\electrical.wav"), &[25])],
        E::Meltdown => &[fixed_at0!("effect\\black_overthrust.wav")],
        E::Steelbody => &[fixed_at0!("effect\\mon_GumGang.wav")],
        E::Intimidate => &[
            fixed_at0!("effect\\EF_Bash.wav"),
            at!(Fixed("effect\\rog_intimidate.wav"), &[35]),
        ],
        E::Backstap => &[at!(Fixed("effect\\rog_back stap.wav"), &[20])],
        E::Bash3d3 => &[fixed_at0!("effect\\headcrush.wav")],
        E::Bash3d4 => &[at!(Fixed("effect\\bitjoint.wav"), &[22])],
        E::Truesight => &[fixed_at0!("effect\\hunter_detecting.wav")],
        E::Bash3d2 => &[at!(Fixed("effect\\mon_bash3d.wav"), &[5])],
        E::Assumptio2 => &[fixed_at0!("effect\\assumptio2.wav")],
        E::Soulbreaker2 => &[fixed_at0!("effect\\meteorassault.wav")],
        E::Tripleattack2 => &[at!(Fixed("effect\\sharpshooting.wav"), &[30])],
        E::Tripleattack3 => &[at!(Fixed("effect\\arrowvulcan.wav"), &[30])],
        E::Chaincombo => &[fixed_at0!("effect\\mon_chaincombo.wav")],
        E::Linelink2 => &[fixed_at0!("effect\\soulchange.wav")],
        E::Linelink3 => &[fixed_at0!("effect\\soulchange.wav")],
        E::Jumpbody => &[at!(Fixed("effect\\evasion2.wav"), &[30])],
        E::Stopeffect => &[fixed_at0!("effect\\runstop.wav")],
        E::Potion7 => &[fixed_at0!("_heal_effect.wav")],
        E::Potion8 => &[fixed_at0!("effect\\inspiration.wav")],
        E::Aldef3 => &[fixed_at0!("effect\\sage_spell breake.wav")],
        E::Spellbreaker => &[fixed_at0!("effect\\sage_spell breake.wav")],
        E::Holycross => &[fixed_at0!("effect\\cru_holy cross.wav")],
        E::Striphelm => &[at!(Fixed("effect\\ez2stronly.wav"), &[6])],
        E::RgCoin => &[fixed_at0!("effect\\rog_steal coin.wav")],
        E::Flamelauncher => &[at!(Fixed("_enemy_hit_wind1.wav"), &[10])],
        E::Frostweapon => &[at!(Fixed("_enemy_hit_wind1.wav"), &[10])],
        E::Lightningloader => &[at!(Fixed("_enemy_hit_wind1.wav"), &[10])],
        E::Seismicweapon => &[at!(Fixed("_enemy_hit_wind1.wav"), &[10])],
        E::Hit2 => &[fixed_at0!("effect\\EF_hit2.wav")],
        E::Hit3 => &[fixed_at0!("effect\\EF_hit3.wav")],
        E::Hit4 => &[fixed_at0!("effect\\EF_hit4.wav")],
        E::Hit5 => &[fixed_at0!("effect\\EF_hit5.wav")],
        E::Hit6 => &[fixed_at0!("effect\\EF_hit6.wav")],
        E::Hit7 => &[fixed_at0!("effect\\EF_hit3.wav")],
        E::Exit => &[fixed_at0!("_heal_effect.wav")],
        E::Beginspell => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspellwhite => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspellred => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::BeginspellN => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell2 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell3 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell4 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell5 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell6 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell7 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginspell8 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Bluecasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Darkcasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Beginasura => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Couplecasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Glasswall => &[fixed_at0!("effect\\EF_GlassWall.wav")],
        E::Glasswall2 => &[fixed_at0!("effect\\EF_GlassWall.wav")],
        E::Blessing => &[fixed_at0!("effect\\EF_Blessing.wav")],
        E::Incagidex => &[fixed_at0!("effect\\EF_IncAgiDex.wav")],
        E::Healsp => &[fixed_at0!("_heal_effect.wav")],
        E::Soulstrike => &[at!(Fixed("effect\\EF_SoulStrike.wav"), &[6, 17, 28, 39, 50])],
        E::Bash => &[fixed_at0!("effect\\EF_Bash.wav")],
        E::Detoxication => &[fixed_at0!("effect\\EF_Detoxication.wav")],
        E::Magnumbreak => &[fixed_at0!("effect\\EF_MagnumBreak.wav")],
        E::Steal => &[fixed_at0!("effect\\EF_Steal.wav")],
        E::Poisonattack => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        E::Snow => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        E::Endure => &[fixed_at0!("effect\\EF_Endure.wav")],
        E::Stonecurse => &[fixed_at0!("effect\\EF_StoneCurse.wav")],
        E::Fireball => &[at!(Fixed("effect\\EF_FireBall.wav"), &[20, 24, 28, 32])],
        E::Icearrow => &[SfxCue {
            wave: Randomized { pattern: "effect\\EF_IceArrow{}.wav", count: 3 },
            timing: AtFrames(&[12]),
        }],
        E::Frostdiver2 => &[fixed_at0!("effect\\EF_FrostDiver2.wav")],
        E::Lightbolt => &[SfxCue {
            wave: Randomized { pattern: "effect\\EF_LightBolt{}.wav", count: 3 },
            timing: AtFrames(&[36]),
        }],
        E::Thunderstorm => &[
            fixed_at0!("effect\\magician_thunderstorm.wav"),
            at!(Fixed("effect\\EF_ThunderStorm.wav"), &[36]),
        ],
        E::Thunderstorm2 => &[fixed_at0!("effect\\EF_ThunderStorm.wav")],
        E::Firearrow => &[SfxCue {
            wave: Randomized { pattern: "effect\\EF_FireArrow{}.wav", count: 3 },
            timing: AtFrames(&[12]),
        }],
        E::Kouenka => &[SfxCue {
            wave: Randomized { pattern: "effect\\EF_FireArrow{}.wav", count: 3 },
            timing: AtFrames(&[12, 13, 14, 15]),
        }],
        E::Napalmbeat => &[fixed_at0!("effect\\EF_NapalmBeat.wav")],
        E::Teleportation => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Teleportation2 => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Exit2 => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Readyportal => &[SfxCue {
            wave: Fixed("effect\\EF_ReadyPortal.wav"),
            timing: EveryFrames(14),
        }],
        E::Readyportal2 => &[SfxCue {
            wave: Fixed("effect\\EF_ReadyPortal.wav"),
            timing: EveryFrames(14),
        }],
        E::Entry2 => &[
            fixed_at0!("effect\\EF_ReadyPortal.wav"),
            fixed_at0!("effect\\EF_Portal.wav"),
        ],
        E::Incagility => &[fixed_at0!("effect\\EF_IncAgility.wav")],
        E::Decagility => &[fixed_at0!("effect\\EF_DecAgility.wav")],
        E::Aqua => &[fixed_at0!("effect\\EF_Aqua.wav")],
        E::Sightrasher => &[fixed_at0!("effect\\wizard_sightrasher.wav")],
        E::Icewall => &[fixed_at0!("effect\\wizard_icewall.wav")],
        // NOTE: F1==2 variant plays random EF_IceArrow{1-3} instead of this wave
        E::Earthspike => &[fixed_at0!("effect\\wizard_earthspike.wav")],
        E::Turnundead => &[at!(Fixed("effect\\EF_Bash.wav"), &[37])],
        E::Spearbmr => &[at!(Fixed("effect\\EF_FireBall.wav"), &[0, 4, 8, 12])],
        E::Removetrap => &[fixed_at0!("effect\\hunter_removetrap.wav")],
        E::Yufitel => &[fixed_at0!("effect\\hunter_shockwavetrap.wav")],
        E::Hasteup => &[
            fixed_at0!("effect\\black_adrenalinerush_a.wav"),
            at!(Fixed("effect\\black_adrenalinerush_b.wav"), &[30]),
        ],
        E::Flasher => &[fixed_at0!("effect\\hunter_flasher.wav")],
        E::Blitzbeat => &[
            at!(Fixed("effect\\hunter_blitzbeat_1st.wav"), &[5]),
            at!(Fixed("effect\\hunter_blitzbeat.wav"), &[30]),
        ],
        E::Waterball => &[fixed_at0!("effect\\wizard_waterball_chulung.wav")],
        E::Waterball2 => &[fixed_at0!("effect\\wizard_waterball_chulung.wav")],
        E::Fireivy => &[fixed_at0!("effect\\wizard_fire_ivy.wav")],
        E::Detecting => &[fixed_at0!("effect\\hunter_detecting.wav")],
        E::Cloaking => &[fixed_at0!("effect\\assasin_cloaking.wav")],
        E::Sonicblow => &[fixed_at0!("effect\\EF_StoneCurse.wav")],
        E::Sonicblowhit => &[fixed_at0!("effect\\assasin_sonicblow.wav")],
        E::Grimtooth => &[fixed_at0!("effect\\EF_FrostDiver.wav")],
        E::Enchantpoison => &[
            fixed_at0!("effect\\assasin_enchantpoison.wav"),
            fixed_at0!("effect\\EF_PoisonAttack.wav"),
        ],
        E::Overthrust => &[
            fixed_at0!("effect\\black_overthrust.wav"),
            fixed_at0!("effect\\EF_StoneCurse.wav"),
        ],
        E::Slowpoison => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        E::Heavensdrive => &[fixed_at0!("effect\\wizard_earthspike.wav")],
        E::Blooddrain => &[fixed_at0!("effect\\EF_SoulStrike.wav")],
        E::Energydrain => &[fixed_at0!("effect\\EF_SoulStrike.wav")],
        E::Angelus => &[fixed_at0!("effect\\EF_Angelus.wav")],
        E::Signum => &[
            at!(Fixed("effect\\EF_Signum.wav"), &[30]),
            at!(Fixed("effect\\EF_Bash.wav"), &[30]),
        ],
        E::Gloria => &[fixed_at0!("effect\\priest_gloria.wav")],
        E::Cure => &[fixed_at0!("effect\\Acolyte_cure.wav")],
        E::Invenom => &[fixed_at0!("effect\\thief_invenom.wav")],
        E::Provoke => &[at!(Fixed("effect\\swordman_provoke.wav"), &[5, 30])],
        E::Skidtrap => &[fixed_at0!("effect\\hunter_skidtrap.wav")],
        E::Magnificat => &[fixed_at0!("effect\\priest_magnificat.wav")],
        E::Resurrection => &[fixed_at0!("effect\\priest_resurrection.wav")],
        E::Recovery => &[fixed_at0!("effect\\priest_recovery.wav")],
        E::Sanctuary => &[fixed_at0!("effect\\priest_sanctuary.wav")],
        // NOTE: original staged over 9 meteo + 2 blastmine phases; frames approximated
        E::Lord => &[
            at!(Fixed("effect\\wizard_meteo.wav"), &[0, 8, 16, 24, 32, 40, 48, 56, 64]),
            at!(Fixed("effect\\hunter_blastmine.wav"), &[70, 80]),
        ],
        E::Stormgust => &[fixed_at0!("effect\\wizard_stormgust.wav")],
        E::Impositio => &[at!(Fixed("effect\\priest_impositio.wav"), &[60])],
        E::Suffragium => &[fixed_at0!("effect\\priest_suffragium.wav")],
        E::Lexdivina => &[at!(Fixed("effect\\priest_lexdivina.wav"), &[0, 20, 35, 60, 70])],
        E::Lexaeterna => &[at!(Fixed("effect\\priest_lexaeterna.wav"), &[15])],
        E::Aspersio => &[at!(Fixed("effect\\priest_aspersio.wav"), &[70])],
        E::Benedictio => &[fixed_at0!("effect\\priest_benedictio.wav")],
        E::Quagmire => &[SfxCue {
            wave: Fixed("effect\\wizard_quagmire.wav"),
            timing: AtFrameChance { frame: 0, one_in: 12 },
        }],
        E::Meteorstorm => &[fixed_at0!("effect\\wizard_meteor.wav")],
        E::Firepillar => &[at!(Fixed("effect\\wizard_fire_pillar_a.wav"), &[75])],
        E::Firepillarbomb => &[fixed_at0!("effect\\wizard_fire_pillar_b.wav")],
        E::Repairweapon => &[at!(Fixed("effect\\black_weapon_repair_a.wav"), &[20, 55])],
        E::Crashearth => &[at!(Fixed("effect\\wizard_fire_pillar_b.wav"), &[30])],
        E::Perfection => &[fixed_at0!("effect\\black_weapon_perfection.wav")],
        E::Maxpower => &[
            fixed_at0!("effect\\black_maximize_power_circle.wav"),
            at!(Fixed("effect\\black_maximize_power_sword.wav"), &[40, 47]),
            at!(Fixed("effect\\black_maximize_power_sword_bic.wav"), &[65]),
        ],
        E::Magnus => &[fixed_at0!("effect\\priest_magnus.wav")],
        E::Blastminebomb => &[at!(Fixed("effect\\hunter_blastmine.wav"), &[20])],
        E::Claymore => &[fixed_at0!("effect\\hunter_claymoretrap.wav")],
        E::Freezing => &[fixed_at0!("effect\\hunter_freezingtrap.wav")],
        E::Springtrap => &[fixed_at0!("effect\\hunter_springtrap.wav")],
        E::Kyrie => &[
            fixed_at0!("effect\\priest_kyrie_eleison_b.wav"),
            at!(Fixed("effect\\priest_kyrie_eleison_a.wav"), &[15]),
        ],
        E::Venomdust => &[fixed_at0!("effect\\assasin_poisonreact.wav")],
        E::Autocounter => &[fixed_at0!("effect\\knight_autocounter.wav")],
        E::Poisonreact2 => &[fixed_at0!("effect\\assasin_poisonreact.wav")],
        E::Splasher => &[at!(Fixed("effect\\assasin_venomsplasher.wav"), &[10])],
        E::Concentration => &[fixed_at0!("effect\\ac_concentration.wav")],
        E::Refineok => &[at!(Fixed("effect\\bs_refinesuccess.wav"), &[15])],
        E::Refinefail => &[at!(Fixed("effect\\bs_refinefailed.wav"), &[5])],
        E::Cartrevolution => &[at!(Fixed("effect\\EF_MagnumBreak.wav"), &[7, 20])],
        E::PotionCon => &[fixed_at0!("effect\\ac_concentration.wav")],
        E::Potion => &[at!(Fixed("effect\\ac_concentration.wav"), &[30])],
        E::PokjukSound => &[at!(Fixed("effect\\firecracker.wav"), &[300])],
        E::Colorpaper => &[fixed_at0!("effect\\wedding.wav")],
        E::Shieldboomerang => &[fixed_at0!("effect\\cru_shield boomerang.wav")],
        E::Shieldboomerang3 => &[fixed_at0!("effect\\cru_shield boomerang.wav")],
        E::Firstaid => &[fixed_at0!("_heal_effect.wav")],
        E::Wind => &[fixed_at0!("_heal_effect.wav")],
        E::Grandcross => &[fixed_at0!("effect\\cru_grand cross.wav")],
        E::Heal => &[fixed_at0!("_heal_effect.wav")],
        E::Heal2 => &[fixed_at0!("_heal_effect.wav")],
        E::Guard => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Guard2 => &[fixed_at0!("effect\\black_maximize_power_sword_bic.wav")],
        E::Halfsphere => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Attackenergy => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Tanji => &[fixed_at0!("effect\\mon_tanji.wav")],
        E::Tanji2 => &[fixed_at0!("effect\\mon_tanji.wav")],
        E::Tripleattack => &[
            at!(Fixed("effect\\EF_hit2.wav"), &[10, 30]),
            at!(Fixed("effect\\EF_hit4.wav"), &[20]),
        ],
        // NOTE: only the F1==0 variant is wired here
        E::Chimto => &[fixed_at0!("effect\\mon_chimto.wav")],
        E::Magicrod => &[fixed_at0!("effect\\sage_magic rod.wav")],
        E::Magnum2 => &[
            at!(Fixed("permeter_attack.wav"), &[20]),
            at!(Fixed("effect\\EF_MagnumBreak.wav"), &[30]),
        ],
        E::Vallentine => &[fixed_at0!("effect\\vallentine.wav")],
        E::Castflower => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        _ => return None,
    })
}


use models::enums::skill_enums::SkillEnum;

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
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::BsGreed => ("effect\\ef_entry.wav", NonPositional),
        S::AllCatcry => ("effect\\2008cat.wav", NonPositional),
        S::MgStonecurse => ("_stonecurse.wav", TargetPositional),
        S::MoExplosionspirits => ("effect\\mon_aeration.wav", NonPositional),
        S::MoSteelbody => ("effect\\mon_bash3d.wav", NonPositional),
        S::LkAurablade => ("effect\\aura_blade.wav", NonPositional),
        S::LkBerserk | S::LkFury => ("effect\\berserk.wav", NonPositional),
        S::NpcPowerup => ("effect\\mon_aeration.wav", NonPositional),
        S::DcWinkcharm => ("effect\\vallentine.wav", NonPositional),
        S::BaPangvoice => ("amon_ra_die01.wav", NonPositional),
        S::BaAssassincross => ("effect\\assassin_sun.wav", NonPositional),
        S::BaPoembragi => ("effect\\browserbased_city.wav", NonPositional),
        S::BaAppleidun => ("effect\\red_apple.wav", NonPositional),
        S::DcHumming => ("effect\\stable_ham.wav", NonPositional),
        S::DcDontforgetme => ("effect\\remember.wav", NonPositional),
        S::DcServiceforyou => ("effect\\services.wav", NonPositional),
        S::BdLullaby => ("effect\\lullaby.wav", NonPositional),
        S::CgHermode => ("effect\\hermod_staff.wav", NonPositional),
        S::BdEternalchaos => ("effect\\eternal_chaos.wav", NonPositional),
        S::BdDrumbattlefield => ("effect\\battlefield.wav", NonPositional),
        S::BdRingnibelungen => ("effect\\nibelrunggen_rings.wav", NonPositional),
        S::BdIntoabyss => ("effect\\into_abyss.wav", NonPositional),
        S::BdSiegfried => ("effect\\immortality.wav", NonPositional),
        S::BdRichmankim => ("effect\\abandon.wav", NonPositional),
        S::BdRokisweil => ("effect\\rockies.wav", NonPositional),
        S::DcFortunekiss => ("effect\\fortunate.wav", NonPositional),
        S::CgTarotcard => ("effect\\priest_slowpoison.wav", NonPositional),
        S::CgLongingfreedom => ("effect\\ac_concentration.wav", NonPositional),
        S::HwMagicpower => ("effect\\amplification_depends.wav", NonPositional),
        S::PfDoublecasting => ("effect\\amplification_depends.wav", NonPositional),
        S::CgMoonlit => ("effect\\moonlight.wav", NonPositional),
        S::TkDodge => ("effect\\breakfall.wav", Depth(-150.0)),
        S::TkSevenwind => ("effect\\stin5.wav", NonPositional),
        S::TkMission => ("effect\\piring.wav", Depth(-100.0)),
        S::SgFusion => ("effect\\transform.wav", NonPositional),
        S::SgHate => ("effect\\register.wav", NonPositional),
        S::SlSwoo => ("effect\\signup.wav", Depth(-50.0)),
        S::SlSke => ("effect\\t_attack.wav", NonPositional),
        S::SlSka => ("effect\\defensive.wav", NonPositional),
        S::SlKaizel => ("effect\\priest_resurrection.wav", NonPositional),
        S::SlKaahi => ("effect\\secondary_magic.wav", NonPositional),
        S::SlKaupe => ("effect\\chiing.wav", NonPositional),
        S::SlKaite => ("effect\\magicreflection.wav", NonPositional),
        S::HamiDefence => ("effect\\h_defence.wav", NonPositional),
        S::HamiCastle => ("effect\\h_castling.wav", NonPositional),
        S::NjTatamigaeshi => ("effect\\tatami_flip.wav", NonPositional),
        S::GsCracker => ("effect\\cracker.wav", NonPositional),
        S::GsGlittering => ("effect\\flip.wav", NonPositional),
        S::PaGospel => ("effect\\gospel.wav", NonPositional),
        S::HpBasilica => ("effect\\basilica.wav", NonPositional),
        _ => return None,
    })
}

pub fn skill_cast_begin_sound(skill: SkillEnum) -> Option<(&'static str, SkillSoundPos)> {
    use SkillEnum as S;
    use SkillSoundPos::*;
    Some(match skill {
        S::MoCombofinish => ("effect\\mon_bash3d.wav", NonPositional),
        S::ChPalmstrike => ("effect\\tiger_bankruptcy.wav", NonPositional),
        S::CrSlimpitcher => ("assulter_attack.wav", Depth(-150.0)),
        S::HwGanbantein => ("effect\\EF_FireWall.wav", NonPositional),
        S::HwGravitation => ("effect\\wizard_earthspike.wav", NonPositional),
        S::NjSuiton => ("effect\\sudan.wav", NonPositional),
        S::GsGrounddrift => ("effect\\ground.wav", NonPositional),
        _ => return None,
    })
}

pub fn skill_projectile_sound(skill: SkillEnum) -> Option<&'static str> {
    use SkillEnum as S;
    Some(match skill {
        S::NjSyuriken | S::NjKunai | S::NjHuuma | S::NjZenynage => "effect\\paladin_throw.wav",
        S::MoExtremityfist => "effect\\mon_asura_peak.wav",
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
        Some(W::Revolver | W::Gatling | W::Shotgun | W::Grenade) => "_hit_gun.wav".to_string(),
        Some(W::Rifle) => "_hit_rifle.wav".to_string(),
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
        J::Archer | J::ArcherHigh | J::BabyArcher
        | J::Thief | J::ThiefHigh | J::BabyThief
        | J::Hunter | J::Sniper | J::BabyHunter
        | J::Assassin | J::AssassinCross | J::BabyAssassin
        | J::Bard | J::Clown | J::BabyBard
        | J::Dancer | J::Gypsy | J::BabyDancer
        | J::Rogue | J::Stalker | J::BabyRogue
        | J::Gunslinger | J::Ninja | J::Taekwon => "player_wooden_male.wav",

        J::Swordsman | J::SwordsmanHigh | J::BabySwordsman
        | J::Knight | J::LordKnight | J::BabyKnight
        | J::Crusader | J::Paladin | J::BabyCrusader
        | J::Monk | J::Champion | J::BabyMonk
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
        assert_eq!(weapon_hit_sound(Some(WeaponType::Bow), 0, false), "_hit_arrow.wav");
        assert_eq!(weapon_hit_sound(Some(WeaponType::Sword2H), 0, false), "_hit_sword.wav");
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

    #[test]
    fn effect_sound_lookup() {
        assert!(effect_sound(EffectId::Stormgust).is_some());
        assert!(effect_sound(EffectId::Firewall).is_none()); // handled elsewhere
    }
}
