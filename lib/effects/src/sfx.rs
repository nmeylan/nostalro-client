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
        E::Loud => &[fixed_at0!("effect\\고성방가.wav")],
        E::Heartcasting => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Soulbreaker => &[at!(Fixed("effect\\기공포.wav"), &[20])],
        E::Pressure => &[at!(Fixed("effect\\프레셔.wav"), &[3])],
        E::Bash3d => &[at!(Fixed("effect\\세크리파이스.wav"), &[22])],
        E::Chemicalprotection => &[fixed_at0!("apocalips_attack.wav")],
        E::Levelup => &[fixed_at0!("levelup.wav")],
        E::Joblevelup => &[fixed_at0!("joblevelup.wav")],
        E::Criticalwound => &[at!(Fixed("effect\\wideb.wav"), &[1])],
        E::NpcSlowcast => &[at!(Fixed("effect\\EF_DecAgility.wav"), &[10])],
        E::NpcEarthquake => &[fixed_at0!("effect\\earth_quake.wav")],
        E::Issen => &[fixed_at0!("effect\\일섬.wav")],
        E::Kasumikiri => &[fixed_at0!("effect\\안개베기.wav")],
        E::Kirikage => &[fixed_at0!("effect\\그림자베기.wav")],
        E::Damage1 => &[fixed_at0!("effect\\EF_hit2.wav")],
        E::Rapidshower => &[at!(Fixed("effect\\래피드샤워.wav"), &[20])],
        E::Magicalbullet => &[fixed_at0!("effect\\매지컬블릿.wav")],
        E::Tripleaction => &[at!(Fixed("effect\\트리플액션.wav"), &[25])],
        E::Trackcasting => &[fixed_at0!("effect\\baku.wav")],
        E::Baku => &[fixed_at0!("effect\\baku.wav")],
        E::Hyousyouraku => &[fixed_at0!("effect\\빙정락.wav")],
        E::Desperado => &[at!(Fixed("effect\\데스페라도.wav"), &[10])],
        E::Bash3d5 => &[fixed_at0!("effect\\bash3d5.wav")],
        E::RgCoin3 => &[at!(Fixed("effect\\디스암.wav"), &[21])],
        E::Stin5 => &[at!(Fixed("effect\\stin5.wav"), &[1])],
        E::Stin4 => &[fixed_at0!("effect\\stin4.wav")],
        E::CookingOk => &[at!(Fixed("_heal_effect.wav"), &[45])],
        E::CookingFail => &[at!(Fixed("caramel_die.wav"), &[50])],
        E::TempOk => &[fixed_at0!("_heal_effect.wav")],
        E::TempFail => &[fixed_at0!("caramel_die.wav")],
        E::Hated2 => &[at!(Fixed("effect\\t_따듯한마법.wav"), &[10])],
        E::Cartter => &[at!(Fixed("effect\\wizard_fire_pillar_b.wav"), &[30])],
        E::Chemical2dash => &[at!(Fixed("effect\\chemical2.wav"), &[44])],
        E::Memorize => &[fixed_at0!("effect\\priest_suffragium.wav")],
        E::Shrink => &[at!(Fixed("effect\\EF_BeginSpell.wav"), &[5])],
        E::Soullink => &[
            fixed_at0!("effect\\t_캐스팅.wav"),
            at!(Fixed("effect\\t_영혼.wav"), &[145]),
        ],
        E::Castspin => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::Piercebody => &[fixed_at0!("effect\\EF_BeginSpell.wav")],
        E::RgCoin2 => &[at!(Fixed("effect\\ez2stronly.wav"), &[1, 11, 21, 31])],
        E::Stin3 => &[fixed_at0!("effect\\t_에너지방출.wav")],
        E::Quakebody4 => &[at!(Fixed("effect\\ef_hit6.wav"), &[20])],
        E::Stin => &[at!(Fixed("effect\\풍인.wav"), &[10])],
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
        E::Attackenergy2 => &[fixed_at0!("effect\\t_마법반사.wav")],
        E::Napalmvalcan => &[at!(Fixed("effect\\EF_NapalmBeat.wav"), &[20, 30, 40, 50, 60])],
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
        E::Bash3d2 => &[at!(Fixed("effect\\mon_bash3d.wav"), &[5])],
        E::Assumptio2 => &[fixed_at0!("effect\\아숨프티오.wav")],
        E::Soulbreaker2 => &[fixed_at0!("effect\\메테오 어썰트.wav")],
        E::Tripleattack2 => &[at!(Fixed("effect\\샤프슈팅.wav"), &[30])],
        E::Tripleattack3 => &[at!(Fixed("effect\\애로우 발칸.wav"), &[30])],
        E::Chaincombo => &[fixed_at0!("effect\\mon_연환.wav")],
        E::Linelink2 => &[fixed_at0!("effect\\소울 체인지.wav")],
        E::Linelink3 => &[fixed_at0!("effect\\소울 체인지.wav")],
        E::Jumpbody => &[at!(Fixed("effect\\t_회피2.wav"), &[30])],
        E::Stopeffect => &[fixed_at0!("effect\\t_달리기.wav")],
        E::Potion7 => &[fixed_at0!("_heal_effect.wav")],
        E::Potion8 => &[fixed_at0!("effect\\흡기.wav")],
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
        E::PokjukSound => &[SfxCue {
            wave: Fixed("effect\\폭죽.wav"),
            timing: EveryFrames(300),
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
    pos: [f32; 3],
    out: &mut Vec<(String, [f32; 3])>,
) {
    match cue.timing {
        SfxTiming::AtFrames(frames) => {
            for &f in frames {
                let f = f as i32;
                if f > prev && f <= cur {
                    out.push((pick_wave(&cue.wave, rng), pos));
                }
            }
        }
        SfxTiming::EveryFrames(n) => {
            let n = n as i32;
            if n > 0 {
                for f in (prev + 1)..=cur {
                    if f > 0 && f % n == 0 {
                        out.push((pick_wave(&cue.wave, rng), pos));
                    }
                }
            }
        }
        SfxTiming::AtFrameChance { frame, one_in } => {
            let f = frame as i32;
            if f > prev && f <= cur && one_in > 0 && next_rand(rng) % one_in as u32 == 0 {
                out.push((pick_wave(&cue.wave, rng), pos));
            }
        }
    }
}

pub fn emit(
    schedule: SfxSchedule,
    prev_frame: i32,
    cur_frame: i32,
    rng: &mut u32,
    pos: [f32; 3],
    out: &mut Vec<(String, [f32; 3])>,
) {
    for cue in schedule {
        emit_cue(cue, prev_frame, cur_frame, rng, pos, out);
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
}
