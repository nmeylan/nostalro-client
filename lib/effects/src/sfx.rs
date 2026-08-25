use models::enums::effect_id::EffectId;

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
    AtFrameThenEvery { first: u16, period: u16 },
    AtFrameChance { frame: u16, one_in: u8 },
}

/// Where a cue is heard from. `depth` is the original's dy volume knob, which
/// only ducks the gain.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SfxPos {
    #[default]
    World,
    WorldAtDepth(f32),
    /// Non-positional: distance never attenuates it.
    Ui(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct SfxCue {
    pub wave: WaveChoice,
    pub timing: SfxTiming,
    pub pos: SfxPos,
}

/// One wave to hand to the mixer, resolved from a cue.
#[derive(Clone, Debug, PartialEq)]
pub struct SfxEmission {
    pub name: String,
    pub world_pos: [f32; 3],
    pub pos: SfxPos,
}

pub type SfxSchedule = &'static [SfxCue];

macro_rules! at {
    ($wave:expr, $frames:expr) => {
        SfxCue {
            wave: $wave,
            timing: SfxTiming::AtFrames($frames),
            pos: SfxPos::World,
        }
    };
}
macro_rules! fixed_at0 {
    ($name:literal) => {
        SfxCue {
            wave: WaveChoice::Fixed($name),
            timing: SfxTiming::AtFrames(&[0]),
            pos: SfxPos::World,
        }
    };
}
/// Positional variant of `at!` that also carries the dy volume knob.
macro_rules! at_depth {
    ($wave:expr, $frames:expr, $depth:expr) => {
        SfxCue {
            wave: $wave,
            timing: SfxTiming::AtFrames($frames),
            pos: SfxPos::WorldAtDepth($depth),
        }
    };
}
/// Non-positional variant of `at!`; `$depth` is the dy volume knob.
macro_rules! ui_at {
    ($wave:expr, $frames:expr, $depth:expr) => {
        SfxCue {
            wave: $wave,
            timing: SfxTiming::AtFrames($frames),
            pos: SfxPos::Ui($depth),
        }
    };
}
macro_rules! fixed_ui_at0 {
    ($name:literal) => {
        ui_at!(WaveChoice::Fixed($name), &[0], 0.0)
    };
}

pub fn effect_sound(id: EffectId) -> Option<SfxSchedule> {
    use EffectId as E;
    use SfxTiming::*;
    use WaveChoice::*;

    Some(match id {
        E::PharmacyOk => &[fixed_at0!("effect\\p_success.wav")],
        E::PharmacyFail => &[fixed_at0!("effect\\p_failed.wav")],
        E::Loud => &[fixed_at0!("effect\\고성방가.wav")],
        E::Heartcasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Soulbreaker => &[at!(Fixed("effect\\기공포.wav"), &[20])],
        E::Pressure => &[at!(Fixed("effect\\프레셔.wav"), &[3])],
        E::Bash3d => &[at!(Fixed("effect\\세크리파이스.wav"), &[22])],
        E::Chemicalprotection => &[fixed_at0!("apocalips_attack.wav")],
        E::Levelup => &[fixed_at0!("levelup.wav")],
        E::Criticalwound => &[at!(Fixed("effect\\wideb.wav"), &[1])],
        E::NpcSlowcast => &[at!(Fixed("effect\\EF_DecAgility.wav"), &[10])],
        E::NpcEarthquake => &[fixed_at0!("effect\\earth_quake.wav")],
        E::Issen => &[fixed_at0!("effect\\일섬.wav")],
        E::Kasumikiri => &[fixed_at0!("effect\\안개베기.wav")],
        E::Kirikage => &[fixed_at0!("effect\\그림자베기.wav")],
        E::Damage1 => &[fixed_at0!("effect\\EF_hit2.wav")],
        E::Rapidshower => &[at!(Fixed("effect\\래피드샤워.wav"), &[20])],
        E::Magicalbullet => &[fixed_at0!("effect\\매지컬블릿.wav")],
        E::Tripleaction => &[ui_at!(Fixed("effect\\트리플액션.wav"), &[25], 0.0)],
        E::Trackcasting => &[fixed_at0!("effect\\폭염룡.wav")],
        E::Baku => &[fixed_at0!("effect\\폭염룡.wav")],
        E::Hyousyouraku => &[fixed_at0!("effect\\빙정락.wav")],
        E::Desperado => &[at!(Fixed("effect\\데스페라도.wav"), &[10])],
        E::Bash3d5 => &[fixed_at0!("effect\\더스트샷.wav")],
        E::RgCoin3 => &[at!(Fixed("effect\\디스암.wav"), &[21])],
        E::Stin5 => &[at!(Fixed("effect\\t_바람방출.wav"), &[1])],
        E::Stin4 => &[fixed_at0!("effect\\풍인.wav")],
        E::CookingOk => &[at!(Fixed("_heal_effect.wav"), &[45])],
        E::CookingFail => &[at!(Fixed("caramel_die.wav"), &[50])],
        E::TempOk => &[fixed_at0!("_heal_effect.wav")],
        E::TempFail => &[fixed_at0!("caramel_die.wav")],
        E::Hated2 => &[at!(Fixed("effect\\t_따듯한마법.wav"), &[10])],
        E::Cartter => &[at!(Fixed("effect\\wizard_fire_pillar_b.wav"), &[30])],
        E::Chemical2dash => &[at!(Fixed("effect\\뇌격쇄.wav"), &[44])],
        E::Memorize => &[fixed_at0!("effect\\priest_suffragium.wav")],
        E::Shrink => &[at!(Fixed("effect\\EF_BeginSpell.wav"), &[5])],
        E::Soullink => &[
            fixed_at0!("effect\\t_캐스팅.wav"),
            at!(Fixed("effect\\t_영혼.wav"), &[145]),
        ],
        E::Castspin => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Piercebody => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::RgCoin2 => &[at!(Fixed("effect\\strip.wav"), &[1, 11, 21, 31])],
        E::Stin3 => &[fixed_at0!("effect\\t_에너지방출.wav")],
        E::Quakebody4 => &[at!(Fixed("effect\\ef_hit6.wav"), &[20])],
        E::Stin => &[at!(Fixed("effect\\t_바람마법.wav"), &[10])],
        E::Stin2 => &[at!(Fixed("effect\\t_날라차기.wav"), &[5, 20, 35, 50, 65])],
        E::Hflimoon1 => &[fixed_at0!("effect\\h_moonlight_1.wav")],
        E::Hflimoon2 => &[fixed_at0!("effect\\h_moonlight_2.wav")],
        E::Hflimoon3 => &[fixed_at0!("effect\\h_moonlight_3.wav")],
        E::HoUp => &[fixed_at0!("levelup.wav")],
        E::Food06 => &[fixed_at0!("_heal_effect.wav")],
        E::M02 => &[at!(Fixed("effect\\풀버스터.wav"), &[5])],
        E::Dragonfear => &[at!(Fixed("effect\\dragonfear.wav"), &[1])],
        E::Bleeding => &[at!(Fixed("effect\\wideb.wav"), &[1])],
        E::Absorbspirits => &[fixed_at0!("effect\\흡기.wav")],
        E::Attackenergy2 => &[
            fixed_at0!("effect\\t_마법반사.wav"),
            at!(Fixed("effect\\kyrie_guard.wav"), &[0, 4]),
        ],
        E::Napalmvalcan => &[at!(
            Fixed("effect\\EF_NapalmBeat.wav"),
            &[20, 30, 40, 50, 60]
        )],
        E::Baby => &[fixed_at0!("effect\\EF_Blessing.wav")],
        E::Cartboost => &[fixed_at0!("effect\\EF_IncAgility.wav")],
        E::Rejectsword => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Flowercast => &[at!(Fixed("effect\\t_안락한마법.wav"), &[5])],
        E::Stormkick => &[at!(Fixed("effect\\t_회오리차기.wav"), &[5])],
        E::Stormkick4 => &[at!(Fixed("effect\\t_회피.wav"), &[15])],
        E::Stormkick5 => &[at!(Fixed("effect\\t_회피.wav"), &[15])],
        E::Electric => &[at!(Fixed("effect\\t_전기.wav"), &[25])],
        E::Meltdown => &[fixed_at0!("effect\\black_overthrust.wav")],
        E::Steelbody => &[fixed_at0!("effect\\mon_금강불괴.wav")],
        E::Intimidate => &[
            fixed_at0!("effect\\EF_Bash.wav"),
            at!(Fixed("effect\\rog_intimidate.wav"), &[35]),
        ],
        E::Backstap => &[at!(Fixed("effect\\rog_back stap.wav"), &[20])],
        E::Bash3d3 => &[fixed_at0!("effect\\헤드 크러쉬.wav")],
        E::Bash3d4 => &[at!(Fixed("effect\\비트 조인트.wav"), &[22])],
        E::Truesight => &[fixed_at0!("effect\\hunter_detecting.wav")],
        E::Bash3d2 => &[at!(Fixed("effect\\mon_맹룡과강.wav"), &[5])],
        E::Assumptio2 => &[fixed_at0!("effect\\아숨프티오.wav")],
        E::Soulbreaker2 => &[fixed_at0!("effect\\메테오 어썰트.wav")],
        E::Tripleattack2 => &[at!(Fixed("effect\\샤프슈팅.wav"), &[30])],
        E::Tripleattack3 => &[at!(Fixed("effect\\애로우 발칸.wav"), &[30])],
        E::Chaincombo => &[fixed_at0!("effect\\mon_연환.wav")],
        E::Linelink2 => &[fixed_at0!("effect\\소울 체인지.wav")],
        E::Linelink3 => &[fixed_at0!("effect\\소울 체인지.wav")],
        E::Jumpbody => &[at!(Fixed("effect\\t_회피2.wav"), &[30])],
        E::Stopeffect => &[fixed_at0!("effect\\t_달리기.wav")],
        E::Potion1 | E::Potion2 | E::Potion3 | E::Potion4 | E::Potion7 => {
            &[at_depth!(Fixed("_heal_effect.wav"), &[0], -80.0)]
        }
        E::Potion5 | E::Potion6 | E::Potion8 => {
            &[at_depth!(Fixed("effect\\흡기.wav"), &[0], -80.0)]
        }
        E::Aldef3 => &[fixed_at0!("effect\\sage_spell breake.wav")],
        E::Spellbreaker => &[fixed_at0!("effect\\sage_spell breake.wav")],
        E::Holycross => &[fixed_ui_at0!("effect\\cru_holy cross.wav")],
        E::Striphelm => &[at!(Fixed("effect\\strip.wav"), &[6])],
        E::RgCoin => &[fixed_at0!("effect\\rog_steal coin.wav")],
        E::Flamelauncher | E::Frostweapon | E::Lightningloader | E::Seismicweapon => {
            &[at_depth!(Fixed("_enemy_hit_wind1.wav"), &[10], -150.0)]
        }
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
        E::Soulstrike => &[at!(
            Fixed("effect\\EF_SoulStrike.wav"),
            &[6, 17, 28, 39, 50]
        )],
        E::Bash => &[fixed_at0!("effect\\EF_Bash.wav")],
        E::Detoxication => &[fixed_at0!("effect\\EF_Detoxication.wav")],
        E::Magnumbreak => &[fixed_at0!("effect\\EF_MagnumBreak.wav")],
        E::Steal => &[fixed_at0!("effect\\EF_Steal.wav")],
        E::Poisonattack => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        E::Snow => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        E::Endure => &[fixed_at0!("effect\\EF_Endure.wav")],
        E::Stonecurse => &[fixed_at0!("effect\\EF_StoneCurse.wav")],
        E::Fireball => &[at!(Fixed("effect\\EF_FireBall.wav"), &[20, 24, 28, 32])],
        E::Icearrow => &[at!(
            Randomized {
                pattern: "effect\\EF_IceArrow{}.wav",
                count: 3,
            },
            &[12]
        )],
        E::Frostdiver2 => &[fixed_at0!("effect\\EF_FrostDiver2.wav")],
        E::Lightbolt => &[at!(
            Randomized {
                pattern: "effect\\EF_LightBolt{}.wav",
                count: 3,
            },
            &[36]
        )],
        E::Thunderstorm => &[fixed_at0!("effect\\magician_thunderstorm.wav")],
        E::Thunderstorm2 => &[fixed_at0!("effect\\EF_ThunderStorm.wav")],
        E::Firearrow => &[at!(
            Randomized {
                pattern: "effect\\EF_FireArrow{}.wav",
                count: 3,
            },
            &[12]
        )],
        E::Kouenka => &[at!(
            Randomized {
                pattern: "effect\\EF_FireArrow{}.wav",
                count: 3,
            },
            &[12, 13, 14, 15]
        )],
        E::Napalmbeat => &[fixed_at0!("effect\\EF_NapalmBeat.wav")],
        E::Teleportation => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Teleportation2 => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Exit2 => &[fixed_at0!("effect\\EF_Teleportation.wav")],
        E::Readyportal => &[SfxCue {
            wave: Fixed("effect\\EF_ReadyPortal.wav"),
            timing: EveryFrames(14),
            pos: SfxPos::World,
        }],
        E::Readyportal2 => &[SfxCue {
            wave: Fixed("effect\\EF_ReadyPortal.wav"),
            timing: EveryFrames(14),
            pos: SfxPos::World,
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
        E::Lord => &[
            at!(
                Fixed("effect\\wizard_meteo.wav"),
                &[0, 20, 50, 80, 100, 130, 140, 180, 200]
            ),
            at!(Fixed("effect\\hunter_blastmine.wav"), &[0, 10]),
        ],
        E::Stormgust => &[fixed_at0!("effect\\wizard_stormgust.wav")],
        E::Impositio => &[at!(Fixed("effect\\priest_impositio.wav"), &[60])],
        E::Suffragium => &[fixed_at0!("effect\\priest_suffragium.wav")],
        E::Lexdivina => &[at!(
            Fixed("effect\\priest_lexdivina.wav"),
            &[0, 20, 35, 60, 70]
        )],
        E::Lexaeterna => &[at!(Fixed("effect\\priest_lexaeterna.wav"), &[15])],
        E::Aspersio => &[at!(Fixed("effect\\priest_aspersio.wav"), &[70])],
        E::Benedictio => &[fixed_at0!("effect\\priest_benedictio.wav")],
        E::Quagmire | E::GreenPop => &[SfxCue {
            wave: Fixed("effect\\wizard_quagmire.wav"),
            timing: AtFrameChance {
                frame: 0,
                one_in: 12,
            },
            pos: SfxPos::WorldAtDepth(-100.0),
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
        E::PotionCon => &[fixed_ui_at0!("effect\\ac_concentration.wav")],
        E::Potion => &[ui_at!(Fixed("effect\\ac_concentration.wav"), &[30], 0.0)],
        E::PotionBerserk => &[ui_at!(Fixed("effect\\ac_concentration.wav"), &[18], 0.0)],
        E::PokjukSound => &[SfxCue {
            wave: Fixed("effect\\폭죽.wav"),
            timing: AtFrameThenEvery {
                first: 300,
                period: 2002,
            },
            pos: SfxPos::Ui(0.0),
        }],
        E::Colorpaper => &[fixed_at0!("effect\\wedding.wav")],
        E::Shieldboomerang => &[fixed_at0!("effect\\cru_shield boomerang.wav")],
        E::Shieldboomerang3 => &[fixed_at0!("effect\\cru_shield boomerang.wav")],
        E::Firstaid => &[fixed_at0!("_heal_effect.wav")],
        E::Wind => &[fixed_at0!("_heal_effect.wav")],
        E::Grandcross => &[fixed_at0!("effect\\cru_grand cross.wav")],
        E::Heal => &[fixed_at0!("_heal_effect.wav")],
        E::Heal2 => &[fixed_at0!("_heal_effect.wav")],
        E::Heal3 => &[fixed_at0!("_heal_effect.wav")],
        E::Heal4 => &[fixed_at0!("_heal_effect.wav")],
        E::Guard => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Guard2 => &[fixed_at0!("effect\\black_maximize_power_sword_bic.wav")],
        E::Halfsphere => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Attackenergy => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Tanji => &[fixed_at0!("effect\\mon_탄지신통.wav")],
        E::Tanji2 => &[fixed_at0!("effect\\mon_탄지신통.wav")],
        E::Tripleattack => &[
            at!(Fixed("effect\\EF_hit2.wav"), &[10, 30]),
            at!(Fixed("effect\\EF_hit4.wav"), &[20]),
        ],
        // NOTE: only the F1==0 variant is wired here
        E::Chimto => &[fixed_at0!("effect\\mon_침투경.wav")],
        E::Magicrod => &[fixed_at0!("effect\\sage_magic rod.wav")],
        E::Magnum2 => &[
            at!(Fixed("permeter_attack.wav"), &[20]),
            at!(Fixed("effect\\EF_MagnumBreak.wav"), &[30]),
        ],
        E::Vallentine => &[fixed_at0!("effect\\vallentine.wav")],
        E::Aciddemon => &[at!(
            Fixed("effect\\EF_FireWall.wav"),
            &[crate::effects::aciddemon::IMPACT_FRAME]
        )],
        E::Agiup => &[fixed_at0!("effect\\EF_IncAgility.wav")],
        E::Beginasura11 => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Grandcross2 => &[fixed_at0!("effect\\cru_grand cross.wav")],
        E::Guard3 => &[fixed_at0!("effect\\kyrie_guard.wav")],
        E::Hptime => &[at_depth!(Fixed("_heal_effect.wav"), &[0], -40.0)],
        E::Sptime => &[at_depth!(Fixed("effect\\흡기.wav"), &[0], -80.0)],
        E::Hyousensou => &[at!(
            Randomized {
                pattern: "effect\\EF_IceArrow{}.wav",
                count: 3,
            },
            &[0]
        )],
        E::Shieldboomerang2 => &[fixed_at0!("effect\\cru_shield boomerang.wav")],
        E::Smdef => &[fixed_at0!("_heal_effect.wav")],
        E::Soulstrike2 => &[at!(
            Fixed("effect\\EF_SoulStrike.wav"),
            &[6, 17, 28, 39, 50]
        )],
        // The wave belongs to each coin bouncing off ground height; the particles
        // spawn at frame 20, so this stands in for the first bounce.
        E::Coin => &[at!(
            Randomized {
                pattern: "effect\\EF_Coin{}.wav",
                count: 3,
            },
            &[30]
        )],
        E::Magiccrasher2 => &[fixed_at0!("effect\\swordman_provoke.wav")],
        E::Petrifyattack => &[fixed_at0!("effect\\EF_StoneCurse.wav")],
        E::Alattack1 | E::Alattack2 | E::Alattack3 | E::Alattack4 => {
            &[fixed_at0!("effect\\mon_탄지신통.wav")]
        }
        E::Firehit => &[at!(
            Randomized {
                pattern: "_enemy_hit_fire{}.wav",
                count: 2,
            },
            &[0]
        )],
        E::Edp => &[at!(
            Fixed("effect\\assasin_enchantpoison.wav"),
            &[0, 5, 11, 18, 26, 40]
        )],
        E::Pattack | E::EnchantpoisonFlow => &[
            fixed_at0!("effect\\assasin_enchantpoison.wav"),
            fixed_at0!("effect\\EF_PoisonAttack.wav"),
        ],
        E::Stripweapon | E::Stripshield | E::Striparmor => &[at!(Fixed("effect\\strip.wav"), &[6])],
        E::Food01 | E::Food02 | E::Food03 | E::Food04 | E::Food05 => {
            &[fixed_at0!("_heal_effect.wav")]
        }
        E::Hamiblood => &[fixed_at0!("effect\\h_blood_lust.wav")],
        E::M01 => &[fixed_at0!("effect\\ef_firehit.wav")],
        E::Homuncasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::EndureZhan | E::EndureSou | E::EndureShan | E::EndureJing => {
            &[fixed_at0!("effect\\EF_Endure.wav")]
        }
        E::Aldef2 => &[fixed_at0!("effect\\sage_spell breake.wav")],
        E::Vallentine2 | E::Itemfast | E::Ro2year => &[fixed_at0!("effect\\vallentine.wav")],
        E::PokLove | E::PokWhite | E::PokValen | E::PokBirth | E::PokChristmas => {
            &[fixed_ui_at0!("effect\\itempokjuk.wav")]
        }
        E::Castflower => &[fixed_at0!("effect\\EF_PoisonAttack.wav")],
        _ => return None,
    })
}

fn next_rand(state: &mut u32) -> u32 {
    // xorshift; seed is never zero at call time.
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick_wave(w: &WaveChoice, rng: &mut u32) -> String {
    match w {
        WaveChoice::Fixed(s) => (*s).to_string(),
        WaveChoice::Randomized { pattern, count } => {
            let n = 1 + (next_rand(rng) % (*count as u32).max(1));
            pattern.replace("{}", &n.to_string())
        }
    }
}

fn emit_cue(
    cue: &SfxCue,
    prev: i32,
    cur: i32,
    rng: &mut u32,
    world_pos: [f32; 3],
    out: &mut Vec<SfxEmission>,
) {
    let mut push = |rng: &mut u32| {
        out.push(SfxEmission {
            name: pick_wave(&cue.wave, rng),
            world_pos,
            pos: cue.pos,
        })
    };
    match cue.timing {
        SfxTiming::AtFrames(frames) => {
            for &f in frames {
                let f = f as i32;
                if f > prev && f <= cur {
                    push(rng);
                }
            }
        }
        SfxTiming::EveryFrames(n) => {
            let n = n as i32;
            if n > 0 {
                for f in (prev + 1)..=cur {
                    if f > 0 && f % n == 0 {
                        push(rng);
                    }
                }
            }
        }
        SfxTiming::AtFrameThenEvery { first, period } => {
            let (first, period) = (first as i32, period as i32);
            if period > 0 {
                for f in (prev + 1).max(first)..=cur {
                    if (f - first) % period == 0 {
                        push(rng);
                    }
                }
            }
        }
        SfxTiming::AtFrameChance { frame, one_in } => {
            let f = frame as i32;
            if f > prev && f <= cur && one_in > 0 && next_rand(rng) % one_in as u32 == 0 {
                push(rng);
            }
        }
    }
}

pub fn emit(
    schedule: SfxSchedule,
    prev_frame: i32,
    cur_frame: i32,
    rng: &mut u32,
    world_pos: [f32; 3],
    out: &mut Vec<SfxEmission>,
) {
    for cue in schedule {
        emit_cue(cue, prev_frame, cur_frame, rng, world_pos, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_sound_lookup() {
        assert!(effect_sound(EffectId::Stormgust).is_some());
        for heal in [
            EffectId::Heal,
            EffectId::Heal2,
            EffectId::Heal3,
            EffectId::Heal4,
        ] {
            assert!(effect_sound(heal).is_some());
        }
        assert!(effect_sound(EffectId::Firewall).is_none()); // handled elsewhere
        assert!(effect_sound(EffectId::Mgdef1).is_none()); // PortalWindEffect self-emits windwalk
    }

    fn run(id: EffectId, upto: i32) -> Vec<SfxEmission> {
        let mut out = Vec::new();
        let mut rng = 0x1234_5678;
        emit(
            effect_sound(id).unwrap(),
            -1,
            upto,
            &mut rng,
            [1.0, 2.0, 3.0],
            &mut out,
        );
        out
    }

    #[test]
    fn cue_positions_and_repeat_schedule_reach_the_emission() {
        let heal = run(EffectId::Hptime, 0);
        assert_eq!(heal.len(), 1);
        assert_eq!(heal[0].name, "_heal_effect.wav");
        assert_eq!(heal[0].pos, SfxPos::WorldAtDepth(-40.0));
        assert_eq!(heal[0].world_pos, [1.0, 2.0, 3.0]);

        assert_eq!(run(EffectId::Holycross, 0)[0].pos, SfxPos::Ui(0.0));
        assert_eq!(run(EffectId::Bash, 0)[0].pos, SfxPos::World);

        // Firecrackers fire once at 300, then the counter rewinds past 2300.
        assert!(run(EffectId::PokjukSound, 299).is_empty());
        assert_eq!(run(EffectId::PokjukSound, 300).len(), 1);
        assert_eq!(run(EffectId::PokjukSound, 2301).len(), 1);
        assert_eq!(run(EffectId::PokjukSound, 2302).len(), 2);
    }

    #[test]
    fn acid_demonstration_bursts_with_the_landing_bottle() {
        let impact = crate::effects::aciddemon::IMPACT_FRAME as i32;
        assert!(run(EffectId::Aciddemon, impact - 1).is_empty());
        assert_eq!(run(EffectId::Aciddemon, impact).len(), 1);
    }
}
