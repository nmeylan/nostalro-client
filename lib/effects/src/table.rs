use models::enums::effect_id::EffectId;

use super::buckets::{is_custom_bucket, is_noop_bucket};
use super::effect_trait::CameraShake;
use super::effects::{
    aciddemon, agiup, attack_energy, aura_blade, banjjakii, barrier, bash, bash3d, begin_asura,
    begin_spell, begin_spell_8, big_portal, blessing, blitzbeat, body_buff, body_tint, bottom_box,
    bottom_sanctuary_pillar, bowling_bash, bubble_drop, callzone, cartrevolution, cartter,
    cast_circle, chemical, chookgi, cloud_projectile, colorpaper, cone, couple_casting,
    curseattack, defender, detecting, dome_ring, dragonsmoke, endure, energy_drain, enhance, entry,
    exit as exit_effect, fireball, fireivy, firepillaron, firstaid, flasher, flowercast,
    frost_diver, fullscreen_overlay, glasswall, glasswall2, grandcross, gravitation, ground_sample,
    guard, gumgang, gumgang2, hasteup, heal, healsp, heartcasting, heavensdrive, hit, hit2, hit5_6,
    hitdark, kouenka, light_sphere, linelink, m_ef02, magic_bolt, magnum_break, mapzone, multibody,
    napalmbeat, napalmvalcan, orbit_burst, particle_up, peong, peong_up, pierce,
    pokjuk, portal, portal_wind, portal2, potion_berserk, potion_con, potion_pillar, pressure,
    providence, quakebody, rainbow, ready_portal, revive, rg_coin, saintwing, sakura, sandwind,
    sight, slash, sma, sonicblowhit, soul_breaker, soul_strike, soullink, spearbmr, spherewind,
    spraypond, squarebody, status_up, stin, storm_kick, stormgust, summon_slave, super_angel,
    teihit, teleportation, texture_falling, throw_item, thunderstorm2, tripleattack, turnundead,
    twilight, volcano, warp, waterball, waterball2, wind, yufitel2, yupitel,
};
use super::spec::{EffectSpec, SprBodyRecolor};
use super::spr_aliases::spr_def;
use super::spr_burst::spr_burst_params;
use super::str_aliases::str_aliases;

pub fn effect_spec(id: EffectId) -> Option<EffectSpec> {
    Some(match id {
        EffectId::Warp => EffectSpec::Custom,

        EffectId::Damage1 | EffectId::Damage12 | EffectId::Damage13 => {
            EffectSpec::Custom
        }

        EffectId::Magnumbreak => EffectSpec::Custom,
        EffectId::Magnum2 => EffectSpec::Custom,
        EffectId::GiExplosion => EffectSpec::Custom,

        EffectId::Thunderstorm2 => EffectSpec::Custom,

        EffectId::M02 => EffectSpec::Custom,

        EffectId::Kaizel => EffectSpec::Custom,

        EffectId::Stopeffect => EffectSpec::Custom,

        EffectId::Angel2 | EffectId::Angel3 => EffectSpec::Custom,

        EffectId::Guard | EffectId::Guard2 | EffectId::Guard3 => EffectSpec::Custom,

        EffectId::Stormkick
        | EffectId::Stormkick1
        | EffectId::Stormkick2
        | EffectId::Stormkick3
        | EffectId::Stormkick6
        | EffectId::Stormkick7 => EffectSpec::Custom,

        EffectId::Peong => EffectSpec::Custom,

        EffectId::Stormkick4 | EffectId::Stormkick5 => EffectSpec::Custom,

        EffectId::Chemicalprotection => EffectSpec::Custom,
        EffectId::Mgattack2 => EffectSpec::Custom,
        EffectId::Chemical2 => EffectSpec::Custom,
        EffectId::Chemical2dash => EffectSpec::Custom,
        EffectId::Chemical3 => EffectSpec::Custom,
        EffectId::Chemical4 => EffectSpec::Custom,
        EffectId::Smatk1 => EffectSpec::Custom,
        EffectId::Smatk2 => EffectSpec::Custom,
        EffectId::Smatk3 => EffectSpec::Custom,
        EffectId::Smatk4 => EffectSpec::Custom,

        EffectId::Stin => EffectSpec::Custom,
        EffectId::Soulbreaker | EffectId::Soulbreaker2 => EffectSpec::Custom,
        EffectId::Teihit2 | EffectId::Backstap => EffectSpec::Custom,

        EffectId::Tripleattack => EffectSpec::Custom,
        EffectId::Tripleattack2 => EffectSpec::Custom,
        EffectId::Tripleattack3 => EffectSpec::Custom,

        EffectId::Spherewind => EffectSpec::Custom,
        EffectId::Spherewind2 => EffectSpec::Custom,
        EffectId::Spherewind3 => EffectSpec::Custom,
        EffectId::Baby => EffectSpec::Custom,
        EffectId::Stin2 => EffectSpec::Custom,
        EffectId::Stin4 => EffectSpec::Custom,
        EffectId::Stin5 => EffectSpec::Custom,
        EffectId::Stin3 => EffectSpec::Custom,
        EffectId::Sma => EffectSpec::Custom,
        EffectId::Sma2 => EffectSpec::Custom,
        EffectId::Sma3 => EffectSpec::Custom,

        EffectId::Throwitem
        | EffectId::Throwitem2
        | EffectId::Throwitem3
        | EffectId::Throwitem4
        | EffectId::Throwitem5
        | EffectId::Throwitem6
        | EffectId::Throwitem7
        | EffectId::Throwitem8
        | EffectId::Throwitem9
        | EffectId::Throwitem10 => EffectSpec::Custom,

        EffectId::RgCoin => EffectSpec::Custom,
        EffectId::RgCoin2 => EffectSpec::Custom,
        EffectId::RgCoin3 => EffectSpec::Custom,

        EffectId::Intimidate => EffectSpec::Custom,

        EffectId::Summonslave => EffectSpec::Custom,
        EffectId::BubbleDrop => EffectSpec::Custom,
        EffectId::Cartter => EffectSpec::Custom,
        EffectId::Icearrow => EffectSpec::Custom,

        EffectId::Tanji
        | EffectId::Tanji2
        | EffectId::Alattack1
        | EffectId::Alattack2
        | EffectId::Alattack3
        | EffectId::Alattack4
        | EffectId::Shieldboomerang
        | EffectId::Shieldboomerang2
        | EffectId::Shieldboomerang3 => EffectSpec::Custom,

        EffectId::Twilight1 | EffectId::Twilight2 | EffectId::Twilight3 => EffectSpec::Custom,

        EffectId::Slim | EffectId::Slim2 | EffectId::Slim3 | EffectId::Pressure => {
            EffectSpec::Custom
        }

        EffectId::Hit1 => EffectSpec::Custom,
        EffectId::Hit2 => EffectSpec::Custom,
        EffectId::Hit3 => EffectSpec::Custom,
        EffectId::Hit4 => EffectSpec::Custom,
        EffectId::Hit5 => EffectSpec::Custom,
        EffectId::Hit6 => EffectSpec::Custom,

        EffectId::Sonicblowhit => EffectSpec::Custom,
        EffectId::Cartrevolution => EffectSpec::Custom,
        EffectId::Napalmvalcan => EffectSpec::Custom,

        EffectId::Stormgust => EffectSpec::Custom,

        EffectId::BottomSanc => EffectSpec::Custom,

        EffectId::Bash => EffectSpec::Custom,

        EffectId::Hasteup => EffectSpec::Custom,
        EffectId::Flasher => EffectSpec::Custom,

        EffectId::Blessing => EffectSpec::Custom,

        EffectId::Healsp => EffectSpec::Custom,

        EffectId::Portal => EffectSpec::Custom,

        EffectId::Portal2 | EffectId::Portal3 => EffectSpec::Custom,

        EffectId::Portal4 | EffectId::Portal5 => EffectSpec::Custom,

        EffectId::Mgdef1 | EffectId::Mgdef2 | EffectId::Mgdef3 | EffectId::Mgdef4 => {
            EffectSpec::Custom
        }

        EffectId::Halfsphere => EffectSpec::Custom,
        EffectId::Attackenergy => EffectSpec::Custom,
        EffectId::Attackenergy2 => EffectSpec::Custom,

        EffectId::BigPortal => EffectSpec::Custom,
        EffectId::BigPortal2 => EffectSpec::Custom,

        EffectId::Readyportal => EffectSpec::Custom,

        EffectId::Teleportation => EffectSpec::Custom,

        EffectId::Spraypond => EffectSpec::Custom,

        EffectId::Glasswall => EffectSpec::Custom,

        EffectId::Endure => EffectSpec::Custom,

        EffectId::Enhance => EffectSpec::Custom,

        EffectId::Entry => EffectSpec::Custom,

        EffectId::Exit => EffectSpec::Custom,

        EffectId::Firearrow => EffectSpec::Custom,
        EffectId::Fireball => EffectSpec::Custom,
        EffectId::Soulstrike => EffectSpec::Custom,
        EffectId::Soulstrike2 => EffectSpec::Custom,
        EffectId::Blooddrain => EffectSpec::Custom,
        EffectId::Energydrain => EffectSpec::Custom,
        EffectId::Energydrain2 => EffectSpec::Custom,
        EffectId::Energydrain3 => EffectSpec::Custom,
        EffectId::Yufitel => EffectSpec::Custom,

        EffectId::Blitzbeat => EffectSpec::Custom,
        EffectId::Waterball => EffectSpec::Custom,
        EffectId::Fireivy => EffectSpec::Custom,
        EffectId::Detecting => EffectSpec::Custom,
        EffectId::Toprank => EffectSpec::Custom,
        EffectId::Party => EffectSpec::Custom,
        EffectId::Curseattack => EffectSpec::Custom,

        EffectId::MapMagiczone | EffectId::MapMagiczone2 | EffectId::Glow4 => EffectSpec::Custom,

        EffectId::Waterfall
        | EffectId::Waterfall90
        | EffectId::WaterfallSmall
        | EffectId::WaterfallSmall90
        | EffectId::WaterfallT2
        | EffectId::WaterfallT290
        | EffectId::WaterfallSmallT2
        | EffectId::WaterfallSmallT290
        | EffectId::Bluefall
        | EffectId::Bluefall90
        | EffectId::Fastbluefall
        | EffectId::Fastbluefall90 => EffectSpec::Custom,

        EffectId::Cloud
        | EffectId::Cloud2
        | EffectId::Cloud3
        | EffectId::Cloud4
        | EffectId::Cloud5
        | EffectId::Cloud6
        | EffectId::Cloud7
        | EffectId::Cloud8 => EffectSpec::Custom,
        EffectId::Napalmbeat => EffectSpec::Custom,
        EffectId::Sandwind => EffectSpec::Custom,

        EffectId::Heavensdrive => EffectSpec::Custom,
        EffectId::Bottom | EffectId::Bottom2 => EffectSpec::Custom,
        EffectId::Cone => EffectSpec::Custom,
        EffectId::Flowercast => EffectSpec::Custom,

        EffectId::Yufitel2 => EffectSpec::Custom,
        EffectId::TextureFalling => EffectSpec::Custom,

        EffectId::Twohandquicken | EffectId::Spearquicken | EffectId::Lkconcentration => {
            EffectSpec::Custom
        }
        EffectId::Bunsinjyutsu => EffectSpec::Custom,

        EffectId::Quakebody => EffectSpec::Custom,
        EffectId::Quakebody2 => EffectSpec::Custom,
        EffectId::Quakebody3 => EffectSpec::Custom,
        EffectId::Quakebody4 => EffectSpec::Custom,

        EffectId::Redbody => EffectSpec::Custom,
        EffectId::Transbluebody => EffectSpec::Custom,
        EffectId::Pinkbody => EffectSpec::Custom,
        EffectId::Linklight => EffectSpec::Custom,
        EffectId::Magiccrasher => EffectSpec::Custom,
        EffectId::Magiccrasher2 => EffectSpec::Custom,
        EffectId::Hitbody => EffectSpec::Custom,
        EffectId::Falconassault => EffectSpec::Custom,

        // Tint-flicker family (colour ↔ white per frame) — Custom; aliases removed.
        EffectId::Chemicalbody => EffectSpec::Custom,
        EffectId::Piercebody => EffectSpec::Custom,
        EffectId::Memorize => EffectSpec::Custom,
        EffectId::Doublecastbody => EffectSpec::Custom,
        EffectId::Greenbody => EffectSpec::Custom,
        EffectId::Shrink => EffectSpec::Custom,
        EffectId::Rejectsword => EffectSpec::Custom,

        EffectId::Bluebody => EffectSpec::Custom,
        EffectId::Redlightbody => EffectSpec::Custom,
        EffectId::RedHit => EffectSpec::Custom,
        EffectId::BlueHit => EffectSpec::Custom,

        EffectId::MadnessBlue => EffectSpec::Custom,
        EffectId::MadnessRed => EffectSpec::Custom,

        EffectId::Pressedbody => EffectSpec::Custom,
        EffectId::Kickedbody => EffectSpec::Custom,

        EffectId::Reflectbody => EffectSpec::Custom,
        EffectId::Assumptio => EffectSpec::Custom,
        EffectId::Lightblade => EffectSpec::Custom,
        EffectId::Undeadbody => EffectSpec::Custom,

        EffectId::Aciddemon => EffectSpec::Custom,
        EffectId::Rainbow => EffectSpec::Custom,
        EffectId::Agiup => EffectSpec::Custom,
        EffectId::Lightsphere => EffectSpec::Custom,
        EffectId::Lightsphere2 => EffectSpec::Custom,

        EffectId::Frostdiver => EffectSpec::Custom,
        EffectId::Frostdiver2 => EffectSpec::Custom,

        EffectId::Sight => EffectSpec::Custom,
        EffectId::Ruwach => EffectSpec::Custom,
        EffectId::Sight2 => EffectSpec::Custom,

        EffectId::Incagility | EffectId::Decagility | EffectId::Incagidex => EffectSpec::Custom,

        EffectId::Landprotector => EffectSpec::Custom,
        EffectId::Volcano => EffectSpec::Custom,
        EffectId::Deluge => EffectSpec::Custom,
        EffectId::Violentgale => EffectSpec::Custom,
        EffectId::Ganbantein => EffectSpec::Custom,
        EffectId::Gumgang3 => EffectSpec::Custom,
        EffectId::Gumgang2 => EffectSpec::Custom,
        EffectId::Gumgang => EffectSpec::Custom,
        EffectId::Steelbody => EffectSpec::Custom,
        EffectId::Gumgangnpc => EffectSpec::Custom,
        EffectId::Doublegumgang => EffectSpec::Custom,
        EffectId::Doublegumgang2 => EffectSpec::Custom,
        EffectId::Doublegumgang3 => EffectSpec::Custom,
        EffectId::Defender => EffectSpec::Custom,
        EffectId::Reflectshield => EffectSpec::Custom,

        EffectId::Heal => EffectSpec::Custom,
        EffectId::Heal2 => EffectSpec::Custom,
        EffectId::Heal3 => EffectSpec::Custom,
        EffectId::Heal4 => EffectSpec::Custom,

        EffectId::Absorbspirits => EffectSpec::Custom,
        EffectId::Exit2 => EffectSpec::Custom,
        EffectId::Entry2 => EffectSpec::Custom,
        EffectId::Smdef => EffectSpec::Custom,
        EffectId::Teleportation2 => EffectSpec::Custom,
        EffectId::Heartcasting => EffectSpec::Custom,
        EffectId::Colorpaper => EffectSpec::Custom,
        EffectId::Readyportal2 => EffectSpec::Custom,
        EffectId::Couplecasting => EffectSpec::Custom,
        EffectId::Gravitation => EffectSpec::Custom,
        EffectId::WindBuff => EffectSpec::Custom,
        EffectId::Wind => EffectSpec::Custom,
        EffectId::Bash3d
        | EffectId::Bash3d2
        | EffectId::Bash3d3
        | EffectId::Bash3d4
        | EffectId::Bash3d5 => EffectSpec::Custom,
        EffectId::Truesight => EffectSpec::Custom,

        EffectId::Beginspell => EffectSpec::Custom,
        EffectId::Aurablade => EffectSpec::Custom,
        EffectId::Beginspell8 => EffectSpec::Custom,
        EffectId::Beginspell2
        | EffectId::Beginspell3
        | EffectId::Beginspell4
        | EffectId::Beginspell5
        | EffectId::Beginspell6
        | EffectId::Beginspell7
        | EffectId::Beginspellred
        | EffectId::Beginspellwhite
        | EffectId::BeginspellN => EffectSpec::Custom,

        EffectId::Beginasura
        | EffectId::Beginasura1
        | EffectId::Beginasura2
        | EffectId::Beginasura3
        | EffectId::Beginasura4
        | EffectId::Beginasura5
        | EffectId::Beginasura6
        | EffectId::Beginasura7
        | EffectId::Beginasura11 => EffectSpec::Custom,

        EffectId::Soullink => EffectSpec::Custom,

        EffectId::Grandcross | EffectId::Grandcross2 => EffectSpec::Custom,

        EffectId::Saintwing => EffectSpec::Custom,

        EffectId::Chookgi | EffectId::Chookgi2 | EffectId::Chookgi3 => EffectSpec::Custom,

        EffectId::Sakura | EffectId::Maple => EffectSpec::Custom,

        EffectId::Pokjuk => EffectSpec::Custom,
        EffectId::PokjukSound => EffectSpec::Custom,

        EffectId::Firstaid => EffectSpec::Custom,

        EffectId::Warpzone
        | EffectId::Warpzone2
        | EffectId::Level99
        | EffectId::Level992
        | EffectId::Level993
        | EffectId::Level994
        | EffectId::Level995
        | EffectId::Level996
        | EffectId::MapGhost
        | EffectId::Icewall
        | EffectId::Earthspike
        | EffectId::Hyousensou
        | EffectId::Grimtooth
        | EffectId::Grimtoothatk
        | EffectId::Mappillar
        | EffectId::Mappillar2
        | EffectId::Mappillar3
        | EffectId::Mappillar4 => EffectSpec::Custom,

        EffectId::Barrier => EffectSpec::Custom,
        EffectId::Banjjakii => EffectSpec::Custom,
        EffectId::Sphere => EffectSpec::Custom,
        EffectId::Removetrap => EffectSpec::Custom,
        EffectId::Turnundead => EffectSpec::Custom,
        EffectId::Firepillaron => EffectSpec::Custom,
        EffectId::Hitdark | EffectId::Darkattack => EffectSpec::Custom,
        EffectId::Spearbmr => EffectSpec::Custom,
        EffectId::Waterball2 => EffectSpec::Custom,

        EffectId::Springtrap => EffectSpec::Str {
            file: "spring",
            duration_ms: default_duration_ms(id),
            repeat: false,
        },

        EffectId::Firewall => EffectSpec::Str {
            file: str_aliases(id)[0],
            duration_ms: GROUND_UNIT_DURATION_MS,
            repeat: true,
        },

        EffectId::Bowlingbash => EffectSpec::Custom,

        EffectId::Dragonsmoke => EffectSpec::Custom,
        EffectId::Overthrust => EffectSpec::Custom,
        EffectId::Energycoat => EffectSpec::Custom,
        EffectId::Callzone => EffectSpec::Custom,
        EffectId::Groundsample => EffectSpec::Custom,

        EffectId::Potionpillar => EffectSpec::Custom,
        EffectId::Revive => EffectSpec::Custom,
        EffectId::Pierce => EffectSpec::Custom,
        EffectId::PotionBerserk => EffectSpec::Custom,
        EffectId::PotionCon => EffectSpec::Custom,
        EffectId::Potion => EffectSpec::Custom,

        EffectId::Glasswall2 => EffectSpec::Custom,
        EffectId::Providence => EffectSpec::Custom,
        EffectId::Kouenka => EffectSpec::Custom,

        EffectId::Blind
        | EffectId::Devil1
        | EffectId::Devil2
        | EffectId::Devil3
        | EffectId::Devil4
        | EffectId::Devil5
        | EffectId::Devil6
        | EffectId::Devil7
        | EffectId::Devil8
        | EffectId::Devil9
        | EffectId::Devil10
        | EffectId::DevilRed
        | EffectId::Poison
        | EffectId::CrystalBlue => EffectSpec::Custom,
        EffectId::Bleeding => EffectSpec::Custom,

        EffectId::Linelink => EffectSpec::Custom,
        EffectId::Linelink2 => EffectSpec::Custom,
        EffectId::Linelink3 => EffectSpec::Custom,

        _ => bucket_default(id),
    })
}

pub fn custom_duration_ms(id: EffectId) -> u32 {
    match id {
        EffectId::Warp => warp::TOTAL_DURATION_MS,
        EffectId::Damage1 | EffectId::Damage12 | EffectId::Damage13 => 500,
        EffectId::Magnumbreak => magnum_break::TOTAL_DURATION_MS,
        EffectId::Magnum2 => dome_ring::MAGNUM2_TOTAL_DURATION_MS,
        EffectId::GiExplosion => dome_ring::GI_EXPLOSION_TOTAL_DURATION_MS,
        EffectId::Thunderstorm2 => thunderstorm2::TOTAL_DURATION_MS,
        EffectId::M02 => m_ef02::TOTAL_DURATION_MS,
        EffectId::Kaizel => slash::TOTAL_DURATION_MS,
        EffectId::Stopeffect => slash::STOPEFFECT_DURATION_MS,
        EffectId::Angel2 | EffectId::Angel3 => super_angel::TOTAL_DURATION_MS,
        EffectId::Guard | EffectId::Guard2 | EffectId::Guard3 => guard::TOTAL_DURATION_MS,
        EffectId::Stormkick
        | EffectId::Stormkick1
        | EffectId::Stormkick2
        | EffectId::Stormkick3
        | EffectId::Stormkick6
        | EffectId::Stormkick7 => storm_kick::TOTAL_DURATION_MS,
        EffectId::Peong => peong::TOTAL_DURATION_MS,
        EffectId::Stormkick4 | EffectId::Stormkick5 => peong_up::TOTAL_DURATION_MS,
        EffectId::Chemicalprotection => chemical::CHEMICALPROTECTION.total_duration_ms(),
        EffectId::Mgattack2 => chemical::MGATTACK2.total_duration_ms(),
        EffectId::Chemical2 => chemical::CHEMICAL2.total_duration_ms(),
        EffectId::Chemical2dash => chemical::CHEMICAL2DASH.total_duration_ms(),
        EffectId::Chemical3 => chemical::CHEMICAL3.total_duration_ms(),
        EffectId::Chemical4 => chemical::CHEMICAL4.total_duration_ms(),
        EffectId::Smatk1 => chemical::SMATK1.total_duration_ms(),
        EffectId::Smatk2 => chemical::SMATK2.total_duration_ms(),
        EffectId::Smatk3 => chemical::SMATK3.total_duration_ms(),
        EffectId::Smatk4 => chemical::SMATK4.total_duration_ms(),
        EffectId::Stin => stin::STIN.total_duration_ms(),
        EffectId::Soulbreaker | EffectId::Soulbreaker2 => soul_breaker::TOTAL_DURATION_MS,
        EffectId::Teihit2 | EffectId::Backstap => teihit::TOTAL_DURATION_MS,
        EffectId::Tripleattack => tripleattack::TRIPLEATTACK.total_duration_ms(),
        EffectId::Tripleattack2 => tripleattack::TRIPLEATTACK2.total_duration_ms(),
        EffectId::Tripleattack3 => tripleattack::TRIPLEATTACK3.total_duration_ms(),
        EffectId::Spherewind => spherewind::SPHEREWIND.total_duration_ms(),
        EffectId::Spherewind2 => spherewind::SPHEREWIND2.total_duration_ms(),
        EffectId::Spherewind3 => spherewind::SPHEREWIND3.total_duration_ms(),
        EffectId::Baby => spherewind::BABY.total_duration_ms(),
        EffectId::Stin2 => stin::STIN2.total_duration_ms(),
        EffectId::Stin4 => stin::STIN4.total_duration_ms(),
        EffectId::Stin5 => stin::STIN5.total_duration_ms(),
        EffectId::Stin3 => sma::STIN3_TOTAL_DURATION_MS,
        EffectId::Sma => sma::SMA_TOTAL_DURATION_MS,
        EffectId::Sma2 => sma::SMA2_TOTAL_DURATION_MS,
        EffectId::Sma3 => particle_up::SMA3_TOTAL_DURATION_MS,
        EffectId::Throwitem
        | EffectId::Throwitem2
        | EffectId::Throwitem3
        | EffectId::Throwitem4
        | EffectId::Throwitem5
        | EffectId::Throwitem6
        | EffectId::Throwitem7
        | EffectId::Throwitem8
        | EffectId::Throwitem9
        | EffectId::Throwitem10 => throw_item::TOTAL_DURATION_MS,
        EffectId::RgCoin => rg_coin::RG_COIN.total_duration_ms(),
        EffectId::RgCoin2 => rg_coin::RG_COIN2.total_duration_ms(),
        EffectId::RgCoin3 => rg_coin::RG_COIN3.total_duration_ms(),
        EffectId::Intimidate => rg_coin::INTIMIDATE.total_duration_ms(),
        EffectId::Summonslave => summon_slave::TOTAL_DURATION_MS,
        EffectId::BubbleDrop => bubble_drop::TOTAL_DURATION_MS,
        EffectId::Cartter => cartter::TOTAL_DURATION_MS,
        EffectId::Icearrow => magic_bolt::ICE_TOTAL_DURATION_MS,
        EffectId::Tanji
        | EffectId::Tanji2
        | EffectId::Alattack1
        | EffectId::Alattack2
        | EffectId::Alattack3
        | EffectId::Alattack4
        | EffectId::Shieldboomerang
        | EffectId::Shieldboomerang2
        | EffectId::Shieldboomerang3 => cloud_projectile::TOTAL_DURATION_MS,
        EffectId::Twilight1 | EffectId::Twilight2 | EffectId::Twilight3 => twilight::TOTAL_DURATION_MS,
        EffectId::Slim | EffectId::Slim2 | EffectId::Slim3 | EffectId::Pressure => pressure::PRESSURE_TOTAL_DURATION_MS,
        EffectId::Hit1 => hit::HIT1_TOTAL_DURATION_MS,
        EffectId::Hit2 => hit2::TOTAL_DURATION_MS,
        EffectId::Hit3 => hit::HIT3_TOTAL_DURATION_MS,
        EffectId::Hit4 => hit::HIT4_TOTAL_DURATION_MS,
        EffectId::Hit5 => hit5_6::HIT5_TOTAL_DURATION_MS,
        EffectId::Hit6 => hit5_6::HIT6_TOTAL_DURATION_MS,
        EffectId::Sonicblowhit => sonicblowhit::TOTAL_DURATION_MS,
        EffectId::Cartrevolution => cartrevolution::TOTAL_DURATION_MS,
        EffectId::Napalmvalcan => napalmvalcan::TOTAL_DURATION_MS,
        EffectId::Stormgust => stormgust::TOTAL_DURATION_MS,
        EffectId::BottomSanc => bottom_sanctuary_pillar::TOTAL_DURATION_MS,
        EffectId::Bash => bash::TOTAL_DURATION_MS,
        EffectId::Hasteup => hasteup::TOTAL_DURATION_MS,
        EffectId::Flasher => flasher::TOTAL_DURATION_MS,
        EffectId::Blessing => blessing::TOTAL_DURATION_MS,
        EffectId::Healsp => healsp::TOTAL_DURATION_MS,
        EffectId::Portal => portal::TOTAL_DURATION_MS,
        EffectId::Portal2 | EffectId::Portal3 => portal2::TOTAL_DURATION_MS,
        EffectId::Portal4 | EffectId::Portal5 => portal_wind::TOTAL_DURATION_MS,
        EffectId::Mgdef1 | EffectId::Mgdef2 | EffectId::Mgdef3 | EffectId::Mgdef4 => portal_wind::TOTAL_DURATION_MS,
        EffectId::Halfsphere => attack_energy::HALFSPHERE_DURATION_MS,
        EffectId::Attackenergy => attack_energy::ATTACKENERGY_DURATION_MS,
        EffectId::Attackenergy2 => attack_energy::ATTACKENERGY2_DURATION_MS,
        EffectId::BigPortal => big_portal::TOTAL_DURATION_MS,
        EffectId::BigPortal2 => big_portal::TOTAL_DURATION_MS_PERSISTENT,
        EffectId::Readyportal => ready_portal::TOTAL_DURATION_MS,
        EffectId::Teleportation => teleportation::TOTAL_DURATION_MS,
        EffectId::Spraypond => spraypond::TOTAL_DURATION_MS,
        EffectId::Glasswall => glasswall::TOTAL_DURATION_MS,
        EffectId::Endure => endure::TOTAL_DURATION_MS,
        EffectId::Enhance => enhance::TOTAL_DURATION_MS,
        EffectId::Entry => entry::TOTAL_DURATION_MS,
        EffectId::Exit => exit_effect::TOTAL_DURATION_MS,
        EffectId::Firearrow => magic_bolt::FIRE_TOTAL_DURATION_MS,
        EffectId::Fireball => fireball::TOTAL_DURATION_MS,
        EffectId::Soulstrike => soul_strike::TOTAL_DURATION_MS,
        EffectId::Soulstrike2 => soul_strike::TOTAL_DURATION_MS,
        EffectId::Blooddrain => energy_drain::BLOOD_DRAIN.total_duration_ms(),
        EffectId::Energydrain => energy_drain::ENERGY_DRAIN.total_duration_ms(),
        EffectId::Energydrain2 => energy_drain::ENERGY_DRAIN2.total_duration_ms(),
        EffectId::Energydrain3 => energy_drain::ENERGY_DRAIN3.total_duration_ms(),
        EffectId::Yufitel => yupitel::TOTAL_DURATION_MS,
        EffectId::Blitzbeat => blitzbeat::TOTAL_DURATION_MS,
        EffectId::Waterball => waterball::TOTAL_DURATION_MS,
        EffectId::Fireivy => fireivy::TOTAL_DURATION_MS,
        EffectId::Detecting => detecting::TOTAL_DURATION_MS,
        EffectId::Toprank => 99990,
        EffectId::Party => 99990,
        EffectId::Curseattack => curseattack::TOTAL_DURATION_MS,
        EffectId::MapMagiczone | EffectId::MapMagiczone2 | EffectId::Glow4 => mapzone::TOTAL_DURATION_MS,
        EffectId::Waterfall | EffectId::Waterfall90 | EffectId::WaterfallSmall | EffectId::WaterfallSmall90 | EffectId::WaterfallT2 | EffectId::WaterfallT290 | EffectId::WaterfallSmallT2 | EffectId::WaterfallSmallT290 | EffectId::Bluefall | EffectId::Bluefall90 | EffectId::Fastbluefall | EffectId::Fastbluefall90 => 4294967295,
        EffectId::Cloud | EffectId::Cloud2 | EffectId::Cloud3 | EffectId::Cloud4 | EffectId::Cloud5 | EffectId::Cloud6 | EffectId::Cloud7 | EffectId::Cloud8 => 4294967295,
        EffectId::Napalmbeat => napalmbeat::TOTAL_DURATION_MS,
        EffectId::Sandwind => sandwind::TOTAL_DURATION_MS,
        EffectId::Heavensdrive => heavensdrive::TOTAL_DURATION_MS,
        EffectId::Bottom | EffectId::Bottom2 => bottom_box::TOTAL_DURATION_MS,
        EffectId::Cone => cone::TOTAL_DURATION_MS,
        EffectId::Flowercast => flowercast::TOTAL_DURATION_MS,
        EffectId::Yufitel2 => yufitel2::TOTAL_DURATION_MS,
        EffectId::TextureFalling => texture_falling::total_duration_ms(&texture_falling::TEXTURE_FALLING),
        EffectId::Twohandquicken | EffectId::Spearquicken | EffectId::Lkconcentration => body_buff::TOTAL_DURATION_MS,
        EffectId::Bunsinjyutsu => body_buff::TOTAL_DURATION_MS,
        EffectId::Quakebody => quakebody::total_duration_ms(&quakebody::QUAKEBODY),
        EffectId::Quakebody2 => quakebody::total_duration_ms(&quakebody::QUAKEBODY2),
        EffectId::Quakebody3 => quakebody::total_duration_ms(&quakebody::QUAKEBODY3),
        EffectId::Quakebody4 => quakebody::total_duration_ms(&quakebody::QUAKEBODY4),
        EffectId::Redbody => body_tint::REDBODY.total_duration_ms(),
        EffectId::Transbluebody => body_tint::TRANSBLUEBODY.total_duration_ms(),
        EffectId::Pinkbody => body_tint::PINKBODY.total_duration_ms(),
        EffectId::Linklight => body_tint::LINKLIGHT.total_duration_ms(),
        EffectId::Magiccrasher => body_tint::MAGICCRASHER.total_duration_ms(),
        EffectId::Magiccrasher2 => body_tint::MAGICCRASHER2.total_duration_ms(),
        EffectId::Hitbody => body_tint::HITBODY.total_duration_ms(),
        EffectId::Falconassault => body_tint::FALCONASSAULT.total_duration_ms(),
        // Tint-flicker family (colour ↔ white per frame) — Custom; aliases removed.
        EffectId::Chemicalbody => body_tint::CHEMICALBODY.total_duration_ms(),
        EffectId::Piercebody => body_tint::PIERCEBODY.total_duration_ms(),
        EffectId::Memorize => body_tint::MEMORIZE.total_duration_ms(),
        EffectId::Doublecastbody => body_tint::DOUBLECASTBODY.total_duration_ms(),
        EffectId::Greenbody => body_tint::GREENBODY.total_duration_ms(),
        EffectId::Shrink => body_tint::SHRINK.total_duration_ms(),
        EffectId::Rejectsword => body_tint::REJECTSWORD.total_duration_ms(),
        EffectId::Bluebody => body_tint::BLUEBODY.total_duration_ms(),
        EffectId::Redlightbody => body_tint::REDLIGHTBODY.total_duration_ms(),
        EffectId::RedHit => body_tint::REDHIT.total_duration_ms(),
        EffectId::BlueHit => body_tint::BLUEHIT.total_duration_ms(),
        EffectId::MadnessBlue => body_tint::MADNESSBLUE.total_duration_ms(),
        EffectId::MadnessRed => body_tint::MADNESSRED.total_duration_ms(),
        EffectId::Pressedbody => squarebody::pressed_total_duration_ms(),
        EffectId::Kickedbody => squarebody::kicked_total_duration_ms(),
        EffectId::Reflectbody => multibody::REFLECTBODY.total_duration_ms(),
        EffectId::Assumptio => multibody::ASSUMPTIO.total_duration_ms(),
        EffectId::Lightblade => multibody::LIGHTBLADE.total_duration_ms(),
        EffectId::Undeadbody => multibody::UNDEADBODY.total_duration_ms(),
        EffectId::Aciddemon => aciddemon::TOTAL_DURATION_MS,
        EffectId::Rainbow => rainbow::TOTAL_DURATION_MS,
        EffectId::Agiup => agiup::TOTAL_DURATION_MS,
        EffectId::Lightsphere => light_sphere::lightsphere_duration_ms(),
        EffectId::Lightsphere2 => light_sphere::LIGHTSPHERE2_DURATION_MS,
        EffectId::Frostdiver => frost_diver::total_duration_ms(&frost_diver::FROSTDIVER),
        EffectId::Frostdiver2 => frost_diver::total_duration_ms(&frost_diver::FROSTDIVER2),
        EffectId::Sight => sight::total_duration_ms(&sight::SIGHT),
        EffectId::Ruwach => sight::total_duration_ms(&sight::RUWACH),
        EffectId::Sight2 => 9999990,
        EffectId::Incagility | EffectId::Decagility | EffectId::Incagidex => status_up::TOTAL_DURATION_MS,
        EffectId::Landprotector => volcano::LANDPROTECTOR.total_duration_ms(),
        EffectId::Volcano => volcano::VOLCANO.total_duration_ms(),
        EffectId::Deluge => volcano::DELUGE.total_duration_ms(),
        EffectId::Violentgale => volcano::VIOLENTGALE.total_duration_ms(),
        EffectId::Ganbantein => volcano::GANBANTEIN.total_duration_ms(),
        EffectId::Gumgang3 => volcano::GUMGANG3.total_duration_ms(),
        EffectId::Gumgang2 => gumgang2::TOTAL_DURATION_MS,
        EffectId::Gumgang => gumgang::GUMGANG.total_duration_ms(),
        EffectId::Steelbody => gumgang::STEELBODY.total_duration_ms(),
        EffectId::Gumgangnpc => gumgang::GUMGANGNPC.total_duration_ms(),
        EffectId::Doublegumgang => gumgang::DOUBLE_RED.total_duration_ms(),
        EffectId::Doublegumgang2 => gumgang::DOUBLE_WHITE.total_duration_ms(),
        EffectId::Doublegumgang3 => gumgang::DOUBLE_BLUE.total_duration_ms(),
        EffectId::Defender => defender::TOTAL_DURATION_MS,
        EffectId::Reflectshield => defender::TOTAL_DURATION_MS,
        EffectId::Heal => heal::HEAL.total_duration_ms(),
        EffectId::Heal2 => heal::HEAL2.total_duration_ms(),
        EffectId::Heal3 => heal::SMDEF.total_duration_ms(),
        EffectId::Heal4 => heal::HEAL4.total_duration_ms(),
        EffectId::Absorbspirits => heal::ABSORBSPIRITS.total_duration_ms(),
        EffectId::Exit2 => heal::EXIT2.total_duration_ms(),
        EffectId::Entry2 => heal::ENTRY2.total_duration_ms(),
        EffectId::Smdef => heal::SMDEF.total_duration_ms(),
        EffectId::Teleportation2 => heal::TELEPORTATION2.total_duration_ms(),
        EffectId::Heartcasting => heartcasting::TOTAL_DURATION_MS,
        EffectId::Colorpaper => colorpaper::TOTAL_DURATION_MS,
        EffectId::Readyportal2 => portal2::READYPORTAL2_DURATION_MS,
        EffectId::Couplecasting => couple_casting::TOTAL_DURATION_MS,
        EffectId::Gravitation => gravitation::TOTAL_DURATION_MS,
        EffectId::WindBuff => 4294967295,
        EffectId::Wind => wind::TOTAL_DURATION_MS,
        EffectId::Bash3d
        | EffectId::Bash3d2
        | EffectId::Bash3d3
        | EffectId::Bash3d4
        | EffectId::Bash3d5 => bash3d::TOTAL_DURATION_MS,
        EffectId::Truesight => bash3d::TOTAL_DURATION_MS,
        EffectId::Beginspell => begin_spell::TOTAL_DURATION_MS,
        EffectId::Aurablade => aura_blade::TOTAL_DURATION_MS,
        EffectId::Beginspell8 => begin_spell_8::TOTAL_DURATION_MS,
        EffectId::Beginspell2
        | EffectId::Beginspell3
        | EffectId::Beginspell4
        | EffectId::Beginspell5
        | EffectId::Beginspell6
        | EffectId::Beginspell7
        | EffectId::Beginspellred
        | EffectId::Beginspellwhite
        | EffectId::BeginspellN => cast_circle::TOTAL_DURATION_MS,
        EffectId::Beginasura
        | EffectId::Beginasura1
        | EffectId::Beginasura2
        | EffectId::Beginasura3
        | EffectId::Beginasura4
        | EffectId::Beginasura5
        | EffectId::Beginasura6
        | EffectId::Beginasura7
        | EffectId::Beginasura11 => begin_asura::TOTAL_DURATION_MS,
        EffectId::Soullink => soullink::TOTAL_DURATION_MS,
        EffectId::Grandcross | EffectId::Grandcross2 => grandcross::TOTAL_DURATION_MS,
        EffectId::Saintwing => saintwing::TOTAL_DURATION_MS,
        EffectId::Chookgi | EffectId::Chookgi2 | EffectId::Chookgi3 => chookgi::TOTAL_DURATION_MS,
        EffectId::Sakura | EffectId::Maple => sakura::TOTAL_DURATION_MS,
        EffectId::Pokjuk => pokjuk::TOTAL_DURATION_MS,
        EffectId::PokjukSound => u32::MAX,
        EffectId::Firstaid => firstaid::TOTAL_DURATION_MS,
        EffectId::Earthspike | EffectId::Hyousensou => 2000,
        EffectId::Warpzone | EffectId::Grimtooth | EffectId::Grimtoothatk => 2500,
        EffectId::Mappillar | EffectId::Mappillar2 | EffectId::Mappillar3 | EffectId::Mappillar4 => 9990,
        EffectId::Icewall => 99990,
        EffectId::Warpzone2 | EffectId::Level99 | EffectId::Level992 | EffectId::Level993 | EffectId::Level994 | EffectId::Level995 | EffectId::Level996 | EffectId::MapGhost => 4294967295,
        EffectId::Barrier => barrier::TOTAL_DURATION_MS,
        EffectId::Banjjakii => banjjakii::TOTAL_DURATION_MS,
        EffectId::Sphere => orbit_burst::SPHERE_TOTAL_DURATION_MS,
        EffectId::Removetrap => orbit_burst::REMOVETRAP_TOTAL_DURATION_MS,
        EffectId::Turnundead => turnundead::TOTAL_DURATION_MS,
        EffectId::Firepillaron => firepillaron::TOTAL_DURATION_MS,
        EffectId::Hitdark | EffectId::Darkattack => hitdark::TOTAL_DURATION_MS,
        EffectId::Spearbmr => spearbmr::TOTAL_DURATION_MS,
        EffectId::Waterball2 => waterball2::TOTAL_DURATION_MS,
        EffectId::Bowlingbash => bowling_bash::TOTAL_DURATION_MS,
        EffectId::Dragonsmoke => dragonsmoke::TOTAL_DURATION_MS,
        EffectId::Overthrust => body_buff::TOTAL_DURATION_MS,
        EffectId::Energycoat => body_buff::TOTAL_DURATION_MS,
        EffectId::Callzone => callzone::TOTAL_DURATION_MS,
        EffectId::Groundsample => ground_sample::TOTAL_DURATION_MS,
        EffectId::Potionpillar => potion_pillar::TOTAL_DURATION_MS,
        EffectId::Revive => revive::TOTAL_DURATION_MS,
        EffectId::Pierce => pierce::TOTAL_DURATION_MS,
        EffectId::PotionBerserk => potion_berserk::TOTAL_DURATION_MS,
        EffectId::PotionCon => potion_con::CONCENTRATION_DURATION_MS,
        EffectId::Potion => potion_con::AWAKENING_DURATION_MS,
        EffectId::Glasswall2 => glasswall2::TOTAL_DURATION_MS,
        EffectId::Providence => providence::TOTAL_DURATION_MS,
        EffectId::Kouenka => kouenka::TOTAL_DURATION_MS,
        EffectId::Blind
        | EffectId::Devil1
        | EffectId::Devil2
        | EffectId::Devil3
        | EffectId::Devil4
        | EffectId::Devil5
        | EffectId::Devil6
        | EffectId::Devil7
        | EffectId::Devil8
        | EffectId::Devil9
        | EffectId::Devil10
        | EffectId::DevilRed
        | EffectId::Poison
        | EffectId::CrystalBlue => fullscreen_overlay::PERSISTENT_DURATION_MS,
        EffectId::Bleeding => fullscreen_overlay::PULSE_DURATION_MS,
        EffectId::Linelink => linelink::LINELINK_DURATION_MS,
        EffectId::Linelink2 => linelink::LINELINK2_DURATION_MS,
        EffectId::Linelink3 => linelink::LINELINK3_DURATION_MS,

        EffectId::Sonicblow => 400,
        EffectId::BabybodyBack | EffectId::BlackNumber | EffectId::BlueNumber | EffectId::Firesplashhit | EffectId::GreenNumber | EffectId::PinkNumber | EffectId::PurpleNumber | EffectId::RedNumber | EffectId::Spinedbody | EffectId::WhiteNumber | EffectId::YellowNumber => 500,
        EffectId::Coldhit => 550,
        EffectId::Bluecasting | EffectId::Darkcasting | EffectId::Landbody => 600,
        EffectId::TaeReady => 850,
        EffectId::Spinedbody2 => 900,
        EffectId::Chaingeholy | EffectId::Changecold | EffectId::Changedark | EffectId::Changeearth | EffectId::Changefire | EffectId::Changeflame | EffectId::Changepoison | EffectId::Changewind | EffectId::Ef4waybody | EffectId::Jumpkick => 1000,
        EffectId::Electric2 | EffectId::Hitline7 => 1500,
        EffectId::Fvoice | EffectId::TempFail | EffectId::TempOk | EffectId::Wink => 1667,
        EffectId::Blackdevil | EffectId::Hitline | EffectId::Hitline2 | EffectId::Hitline3 | EffectId::Hittexture | EffectId::SmaReady => 2000,
        EffectId::Hitline4 | EffectId::Hitline5 | EffectId::Sightrasher => 2500,
        EffectId::Electric | EffectId::Hated2 | EffectId::Hitline6 | EffectId::Hptime | EffectId::ItemLight | EffectId::Sprinklesand | EffectId::Sptime | EffectId::Teihit1 | EffectId::Teihit1x | EffectId::Teihit3 => 3000,
        EffectId::NpcSlowcast => 3100,
        EffectId::Lockon => 3333,
        EffectId::Foot | EffectId::Foot2 | EffectId::Foot3 | EffectId::Foot4 | EffectId::Foot5 | EffectId::Foot6 => 3400,
        EffectId::Tarotcard1 | EffectId::Tarotcard10 | EffectId::Tarotcard11 | EffectId::Tarotcard12 | EffectId::Tarotcard13 | EffectId::Tarotcard14 | EffectId::Tarotcard2 | EffectId::Tarotcard3 | EffectId::Tarotcard4 | EffectId::Tarotcard5 | EffectId::Tarotcard6 | EffectId::Tarotcard7 | EffectId::Tarotcard8 | EffectId::Tarotcard9 => 4067,
        EffectId::Hated | EffectId::Jumpbody => 5000,
        EffectId::Bat | EffectId::Bat2 | EffectId::Ghost => 40000,
        EffectId::BottomMag | EffectId::Venomdust2 => 99990,
        EffectId::BottomGospel => 199990,
        EffectId::BottomAppleidun | EffectId::BottomAssassincross | EffectId::BottomBasilica | EffectId::BottomDe | EffectId::BottomDissonance | EffectId::BottomDontforgetme | EffectId::BottomDrumbattlefield | EffectId::BottomEternalchaos | EffectId::BottomEvilland | EffectId::BottomFogwall | EffectId::BottomFortunekiss | EffectId::BottomHermode | EffectId::BottomHumming | EffectId::BottomIntoabyss | EffectId::BottomLa | EffectId::BottomLullaby | EffectId::BottomPoembragi | EffectId::BottomRichmankim | EffectId::BottomRingnibelungen | EffectId::BottomRokisweil | EffectId::BottomRunner | EffectId::BottomServiceforyou | EffectId::BottomSiegfried | EffectId::BottomSpider | EffectId::BottomSuiton | EffectId::BottomTransfer | EffectId::BottomUglydance | EffectId::BottomVi | EffectId::BottomVo | EffectId::BottomWhistle => 299990,
        EffectId::Forestlight | EffectId::Forestlight2 | EffectId::Forestlight3 | EffectId::Forestlight4 => 600000,
        EffectId::Asurabody | EffectId::Babybody | EffectId::Babybody2 | EffectId::Giantbody | EffectId::Giantbody2 => 999990,
        EffectId::Dust | EffectId::Glow1 | EffectId::Glow11 | EffectId::Glow12 | EffectId::Glow2 | EffectId::Green995 | EffectId::Green996 | EffectId::TorchGreen | EffectId::TorchPurple | EffectId::TorchRed => 4294967295,

        _ => 0,
    }
}

pub fn spawn_camera_shake(id: EffectId) -> Option<CameraShake> {
    let (amplitude, duration_ms) = match id {
        EffectId::ScreenQuake => (1.5, 1667),
        EffectId::NpcEarthquake => (2.2, 1300),
        EffectId::Dragonfear => (1.0, 650),
        EffectId::Teihit1x => (1.0, 650),
        EffectId::Gumgang2 => (1.0, 650),
        EffectId::Hitline => (1.0, 650),
        EffectId::Hitline2 => (1.0, 650),
        EffectId::Bash3d2 => (1.0, 650),
        _ => return None,
    };
    Some(CameraShake {
        amplitude,
        duration_ms,
    })
}

fn spr_body_recolor(id: EffectId) -> Option<SprBodyRecolor> {
    match id {
        EffectId::Edp => Some(SprBodyRecolor {
            window_frames: (0, 80),
            rgb: [255, 0, 255],
        }),
        _ => None,
    }
}

fn bucket_default(id: EffectId) -> EffectSpec {
    if let Some(def) = spr_def(id) {
        return EffectSpec::Spr {
            sprite: def.sprite,
            duration_ms: default_duration_ms(id),
            size_scale: def.size_scale,
            anim_speed: def.anim_speed,
            repeat: def.repeat,
            tint: def.tint,
            pos_y: def.pos_y,
            action_index: def.action,
        };
    }
    if let Some((sprite, burst)) = spr_burst_params(id) {
        return EffectSpec::SprBurst {
            sprite,
            duration_ms: default_duration_ms(id),
            burst,
            body_recolor: spr_body_recolor(id),
        };
    }
    if !str_aliases(id).is_empty() {
        return default_str_spec(id);
    }
    if is_noop_bucket(id) {
        return EffectSpec::Noop;
    }
    if is_custom_bucket(id) {
        EffectSpec::Custom
    } else {
         EffectSpec::Noop
    }
}

fn default_str_spec(id: EffectId) -> EffectSpec {
    let duration_ms = default_duration_ms(id);
    let file = str_aliases(id)[0];
    EffectSpec::Str {
        file,
        duration_ms,
        repeat: false,
    }
}

const GROUND_UNIT_DURATION_MS: u32 = 99990;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lv99_resolves_to_custom_factory_path() {
        assert!(matches!(
            effect_spec(EffectId::Level99),
            Some(EffectSpec::Custom)
        ));
    }

    #[test]
    fn weather_effects_route_to_their_procedural_impls() {
        assert!(
            matches!(effect_spec(EffectId::Maple), Some(EffectSpec::Custom)),
            "Maple must reach the sakura machinery, not a frozen spr"
        );
        assert!(
            matches!(
                effect_spec(EffectId::PokjukSound),
                Some(EffectSpec::Custom)
            ),
            "PokjukSound must hold a silent entry so its SFX schedule fires"
        );
        assert!(
            matches!(effect_spec(EffectId::Snow), Some(EffectSpec::SprBurst { .. })),
            "Snow stays on the SprBurst path"
        );
    }

    #[test]
    fn known_str_files_resolve() {
        assert!(matches!(
            effect_spec(EffectId::Bubble),
            Some(EffectSpec::Str {
                file: "bubble1",
                ..
            })
        ));
        assert!(matches!(
            effect_spec(EffectId::Lvup),
            Some(EffectSpec::Str {
                file: "LevelUP",
                ..
            })
        ));
        assert!(matches!(
            effect_spec(EffectId::Magnus),
            Some(EffectSpec::Str { file: "Magnus", .. })
        ));
    }

    #[test]
    fn torch_is_an_spr_loop() {
        let Some(EffectSpec::Spr {
            sprite,
            duration_ms,
            size_scale,
            anim_speed,
            repeat,
            tint,
            pos_y,
            action_index: _,
        }) = effect_spec(EffectId::Torch)
        else {
            panic!("Torch should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/torch_01");
        assert_eq!(duration_ms, u32::MAX);
        assert_eq!(size_scale, 1.0);
        assert_eq!(anim_speed, 1.0);
        assert!(repeat);
        assert_eq!(tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(pos_y, 0.0);
    }

    #[test]
    fn aqua_is_a_one_shot_spr() {
        let Some(EffectSpec::Spr {
            sprite,
            duration_ms,
            size_scale,
            anim_speed,
            repeat,
            tint,
            pos_y,
            action_index: _,
        }) = effect_spec(EffectId::Aqua)
        else {
            panic!("Aqua should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/성수뜨기");
        assert_eq!(duration_ms, 1667);
        assert_eq!(size_scale, 1.0);
        assert_eq!(anim_speed, 2.0);
        assert!(!repeat);
        assert_eq!(tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(pos_y, -20.0);
    }

    #[test]
    fn item_status_billboards_resolve_to_one_shot_spr() {
        for (id, sprite) in [
            (EffectId::ItemThunder, "data/sprite/이팩트/item_thunder"),
            (EffectId::ItemCloud, "data/sprite/이팩트/item_cloud"),
            (EffectId::ItemCurse, "data/sprite/이팩트/item_curse"),
            (EffectId::ItemZzz, "data/sprite/이팩트/item_zzz"),
            (EffectId::ItemRain, "data/sprite/이팩트/item_rain"),
        ] {
            let Some(EffectSpec::Spr {
                sprite: got,
                duration_ms,
                anim_speed,
                repeat,
                tint,
                ..
            }) = effect_spec(id)
            else {
                panic!("{id:?} should resolve to EffectSpec::Spr");
            };
            assert_eq!(got, sprite, "{id:?} sprite path");
            assert_eq!(duration_ms, 1667, "{id:?} duration");
            assert_eq!(anim_speed, 2.0, "{id:?} anim speed");
            assert!(!repeat, "{id:?} one-shot");
            assert_eq!(tint, [1.0, 1.0, 1.0, 1.0], "{id:?} default tint");
        }
    }

    #[test]
    fn spr_oneshot_batch_resolves() {
        for (id, sprite, anim, dur) in [
            (EffectId::M01, "data/sprite/이팩트/m_ef01", 3.0, 833),
            (EffectId::M03, "data/sprite/이팩트/m_ef03", 4.0, 1667),
            (EffectId::M05, "data/sprite/이팩트/m_ef05", 4.0, 1667),
            (EffectId::M06, "data/sprite/이팩트/m_ef06", 4.0, 1667),
            (EffectId::M07, "data/sprite/이팩트/m_ef07", 4.0, 1667),
            (
                EffectId::PokWhite,
                "data/sprite/이팩트/폭죽_화이트데이",
                4.0,
                1000,
            ),
            (
                EffectId::PokValen,
                "data/sprite/이팩트/폭죽_발렌타인",
                4.0,
                1000,
            ),
        ] {
            let Some(EffectSpec::Spr {
                sprite: got,
                anim_speed,
                repeat,
                duration_ms,
                ..
            }) = effect_spec(id)
            else {
                panic!("{id:?} should resolve to EffectSpec::Spr");
            };
            assert_eq!(got, sprite, "{id:?} sprite");
            assert_eq!(anim_speed, anim, "{id:?} anim speed");
            assert_eq!(duration_ms, dur, "{id:?} duration");
            assert!(!repeat, "{id:?} one-shot");
        }
        assert!(matches!(
            effect_spec(EffectId::M04),
            Some(EffectSpec::Spr { repeat: true, .. })
        ));
        assert!(matches!(
            effect_spec(EffectId::M02),
            Some(EffectSpec::Custom)
        ));
        assert!(matches!(
            effect_spec(EffectId::Kaizel),
            Some(EffectSpec::Custom)
        ));
        assert!(matches!(
            effect_spec(EffectId::Kaahi),
            Some(EffectSpec::Noop)
        ));
    }

    #[test]
    fn vallentine_family_resolves_to_spr_with_correct_action() {
        let Some(EffectSpec::Spr {
            sprite,
            action_index,
            anim_speed,
            repeat,
            ..
        }) = effect_spec(EffectId::Vallentine2)
        else {
            panic!("Vallentine2 should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/vallentine");
        assert_eq!(action_index, 1, "Vallentine2 plays ACT action 1");
        assert_eq!(anim_speed, 2.0);
        assert!(!repeat);

        let Some(EffectSpec::Spr {
            sprite,
            action_index,
            anim_speed,
            ..
        }) = effect_spec(EffectId::Itemfast)
        else {
            panic!("Itemfast should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/fast");
        assert_eq!(action_index, 0);
        assert_eq!(anim_speed, 4.0);

        assert!(matches!(
            effect_spec(EffectId::Vallentine),
            Some(EffectSpec::Spr {
                action_index: 0,
                ..
            })
        ));
    }

    #[test]
    fn wink_resolves_to_custom_factory_path() {
        assert!(matches!(
            effect_spec(EffectId::Wink),
            Some(EffectSpec::Custom)
        ));
        assert_eq!(custom_duration_ms(EffectId::Wink), 1667);
        assert!(matches!(
            effect_spec(EffectId::Fvoice),
            Some(EffectSpec::Custom)
        ));
        assert_eq!(custom_duration_ms(EffectId::Fvoice), 1667);
    }

    #[test]
    fn ghost_family_resolves_to_custom_factory_path() {
        for id in [EffectId::Ghost, EffectId::Bat, EffectId::Bat2] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom)),
                "{id:?} should resolve to Custom, got {:?}",
                effect_spec(id)
            );
            assert_eq!(custom_duration_ms(id), 40000, "{id:?} duration");
            let built = super::super::factory::make_effect(
                id,
                super::super::spec::EffectAnchor::Point([0.0; 3]),
                None,
                None,
                None,
            );
            assert!(
                built.is_some_and(|e| !e.is_placeholder()),
                "{id:?} must have a real impl"
            );
        }
    }

    #[test]
    fn poisonhit_uses_org_argb_size_anim_speed_and_one_shot() {
        let Some(EffectSpec::Spr {
            sprite,
            duration_ms,
            size_scale,
            anim_speed,
            repeat,
            tint,
            pos_y,
            action_index: _,
        }) = effect_spec(EffectId::Poisonhit)
        else {
            panic!("Poisonhit should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/poisonhit");
        assert_eq!(duration_ms, 500);
        assert_eq!(size_scale, 1.5);
        assert_eq!(anim_speed, 2.0);
        assert!(!repeat);
        assert_eq!(tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(pos_y, 0.0);
    }

    #[test]
    fn darkbreath_tints_red_and_overrides_table_duration() {
        let Some(EffectSpec::Spr {
            sprite,
            duration_ms,
            size_scale,
            anim_speed,
            repeat,
            tint,
            pos_y,
            action_index: _,
        }) = effect_spec(EffectId::Darkbreath)
        else {
            panic!("Darkbreath should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/darkbreath");
        assert_eq!(duration_ms, 1083);
        assert!((size_scale - 0.8).abs() < 1e-6);
        assert_eq!(anim_speed, 1.0);
        assert!(repeat);
        assert_eq!(tint, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(pos_y, -20.0);
    }

    #[test]
    fn bottom_songs_resolve_to_custom_not_missing_str() {
        for id in [
            EffectId::BottomDissonance,
            EffectId::BottomWhistle,
            EffectId::BottomServiceforyou,
            EffectId::BottomRokisweil,
            EffectId::BottomMag,
            EffectId::BottomGospel,
            EffectId::BottomSpider,
            EffectId::BottomFogwall,
            EffectId::BottomHermode,
            EffectId::BottomRunner,
            EffectId::BottomTransfer,
            EffectId::BottomEvilland,
            EffectId::Dust,
            EffectId::TorchRed,
            EffectId::TorchGreen,
            EffectId::TorchPurple,
            EffectId::Glow1,
            EffectId::Glow2,
            EffectId::Glow11,
            EffectId::Glow12,
        ] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom)),
                "{id:?} must resolve to Custom, got {:?}",
                effect_spec(id)
            );
        }
    }

    #[test]
    fn batch7_procedural_effects_resolve_to_custom_not_missing_str() {
        for id in [
            EffectId::Blackdevil,
            EffectId::Bluecasting,
            EffectId::Darkcasting,
            EffectId::Electric,
            EffectId::Electric2,
            EffectId::Hitline,
            EffectId::Hitline2,
            EffectId::Hitline3,
            EffectId::Hitline4,
            EffectId::Hitline5,
            EffectId::Hitline6,
            EffectId::Hitline7,
            EffectId::Giantbody,
            EffectId::Giantbody2,
            EffectId::Babybody,
            EffectId::Babybody2,
            EffectId::BabybodyBack,
            EffectId::Jumpkick,
            EffectId::Jumpbody,
            EffectId::Landbody,
            EffectId::Spinedbody,
            EffectId::Spinedbody2,
            EffectId::Asurabody,
            EffectId::TaeReady,
            EffectId::Ef4waybody,
        ] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom)),
                "{id:?} must resolve to Custom, got {:?}",
                effect_spec(id)
            );
        }
    }

    #[test]
    fn demonstration_loops_with_size_and_y_offset() {
        let Some(EffectSpec::Spr {
            sprite,
            size_scale,
            repeat,
            pos_y,
            ..
        }) = effect_spec(EffectId::Demonstration)
        else {
            panic!("Demonstration should resolve to EffectSpec::Spr");
        };
        assert_eq!(sprite, "data/sprite/이팩트/데몬스트레이션");
        assert!((size_scale - 1.2).abs() < 1e-6);
        assert!(repeat, "MT_LOOP means the action keeps cycling");
        assert_eq!(pos_y, -1.0);
    }

    #[test]
    fn dragonsmoke_resolves_to_custom_trail_effect() {
        let Some(EffectSpec::Custom) = effect_spec(EffectId::Dragonsmoke) else {
            panic!("Dragonsmoke should resolve to EffectSpec::Custom");
        };
        let duration_ms = custom_duration_ms(EffectId::Dragonsmoke);
        assert_eq!(
            duration_ms,
            u32::MAX,
            "ambient loop, persists for the map's lifetime"
        );
    }

    #[test]
    fn batch2_billboards_route_to_spr_burst_variants() {
        let Some(EffectSpec::Custom) = effect_spec(EffectId::Thunderstorm2) else {
            panic!("Thunderstorm2 should resolve to EffectSpec::Custom");
        };
        assert_eq!(
            custom_duration_ms(EffectId::Thunderstorm2),
            thunderstorm2::TOTAL_DURATION_MS
        );

        let Some(EffectSpec::SprBurst { sprite, burst, .. }) = effect_spec(EffectId::Slowpoison)
        else {
            panic!("Slowpoison should resolve to EffectSpec::SprBurst");
        };
        assert_eq!(sprite, "data/sprite/이팩트/particle3");
        assert_eq!(burst.period_frames, Some(5));
        assert_eq!(burst.pos_y_start, -20.0);
        assert!(
            burst.speed_range.1 < 0.0,
            "speed range stays negative for downward drift"
        );

        let Some(EffectSpec::SprBurst {
            burst,
            body_recolor,
            ..
        }) = effect_spec(EffectId::Edp)
        else {
            panic!("Edp should resolve to EffectSpec::SprBurst");
        };
        assert_eq!(burst.period_frames, Some(3));
        assert!((burst.size - 0.3).abs() < 1e-6);
        assert_eq!(
            body_recolor,
            Some(SprBodyRecolor {
                window_frames: (0, 80),
                rgb: [255, 0, 255]
            }),
            "Enchant Deadly Poison flickers the caster magenta",
        );
        let Some(EffectSpec::SprBurst {
            body_recolor: none, ..
        }) = effect_spec(EffectId::Slowpoison)
        else {
            unreachable!();
        };
        assert_eq!(none, None);
    }

    #[test]
    fn stormgust_resolves_to_factory_custom_with_str_overlay() {
        assert!(matches!(
            effect_spec(EffectId::Stormgust),
            Some(EffectSpec::Custom)
        ));
    }

    #[test]
    fn warp_routes_to_factory_via_custom() {
        assert!(matches!(
            effect_spec(EffectId::Warp),
            Some(EffectSpec::Custom)
        ));
    }

    #[test]
    fn ez2str_family_specs() {
        assert!(matches!(
            effect_spec(EffectId::Aurablade),
            Some(EffectSpec::Custom)
        ));
        for id in [EffectId::Soulburn, EffectId::Soulchange] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Str { .. })),
                "{id:?} must resolve to a Str",
            );
        }
    }

    #[test]
    fn batch_fr_routes_to_custom() {
        for id in [
            EffectId::Cone,
            EffectId::Bottom,
            EffectId::Bottom2,
            EffectId::Heavensdrive,
            EffectId::Flowercast,
        ] {
            assert!(
                matches!(effect_spec(id), Some(EffectSpec::Custom)),
                "{id:?} should be Custom",
            );
        }
    }

    #[test]
    fn dragonfear_keeps_its_str_and_gains_a_camera_shake() {
        assert!(matches!(
            effect_spec(EffectId::Dragonfear),
            Some(EffectSpec::Str { .. })
        ));
        assert!(spawn_camera_shake(EffectId::Dragonfear).is_some());
        assert!(spawn_camera_shake(EffectId::Hit1).is_none());
        // Champion impacts that quake the camera: Asura's target burst,
        // Explosion Spirits / Steel Body activation ring, Combo Finish,
        // Palm Strike, Tiger Fist.
        for id in [
            EffectId::Teihit1x,
            EffectId::Gumgang2,
            EffectId::Hitline,
            EffectId::Hitline2,
            EffectId::Bash3d2,
        ] {
            assert!(spawn_camera_shake(id).is_some(), "{id:?} must quake");
        }
    }
}

fn default_duration_ms(id: EffectId) -> u32 {
    match id {
        EffectId::Coin => 1000,
        EffectId::Steal => 500,
        EffectId::Pattack => 1000,
        EffectId::Detoxication => 1000,
        EffectId::Stonecurse => 9990,
        EffectId::Firewall => 400,
        EffectId::Lightbolt => 2000,
        EffectId::Thunderstorm => 1040,
        EffectId::Aqua => 1667,
        EffectId::Signum => 9990,
        EffectId::Angelus => 9990,
        EffectId::Smoke => 500,
        EffectId::Firefly => 2333,
        EffectId::Torch => u32::MAX,
        EffectId::Firehit => 500,
        EffectId::Windhit => 400,
        EffectId::Poisonhit => 500,
        EffectId::Arrowshot => 9990,
        EffectId::Invenom => 9990,
        EffectId::Cure => 9990,
        EffectId::Provoke => 9990,
        EffectId::Mvp => 9990,
        EffectId::Skidtrap => 99990,
        EffectId::Brandishspear => 9990,
        EffectId::Gloria => 9990,
        EffectId::Magnificat => 9990,
        EffectId::Resurrection => 9990,
        EffectId::Recovery => 9990,
        EffectId::Sanctuary => 9990,
        EffectId::Impositio => 9990,
        EffectId::Lexaeterna => 9990,
        EffectId::Aspersio => 9990,
        EffectId::Lexdivina => 9990,
        EffectId::Suffragium => 9990,
        EffectId::Lord => 99990,
        EffectId::Benedictio => 99990,
        EffectId::Meteorstorm => 99990,
        EffectId::Yufitelhit => 2500,
        EffectId::Quagmire => 1500,
        EffectId::Firepillar => 99990,
        EffectId::Firepillarbomb => 99990,
        EffectId::Repairweapon => 99990,
        EffectId::Crashearth => 99990,
        EffectId::Perfection => 9990,
        EffectId::Maxpower => 9990,
        EffectId::Blastmine => 2500,
        EffectId::Blastminebomb => 99990,
        EffectId::Claymore => 99990,
        EffectId::Freezing => 99990,
        EffectId::Bubble => 99990,
        EffectId::Gaspush => 99990,
        EffectId::Springtrap => 99990,
        EffectId::Kyrie => 9990,
        EffectId::Magnus => 99990,
        EffectId::Cloaking => 2500,
        EffectId::Venomdust => 99990,
        EffectId::Enchantpoison => 2500,
        EffectId::Poisonreact => 99990,
        EffectId::Poisonreact2 => 99990,
        EffectId::Splasher => 99990,
        EffectId::Autocounter => 1000,
        EffectId::Freeze => 99990,
        EffectId::Freezed => 99990,
        EffectId::Icecrash => 99990,
        EffectId::Slowpoison => 1333,
        EffectId::Sandman => 99990,
        EffectId::Pneuma => 99990,
        EffectId::Sonicblow2 => 9990,
        EffectId::Brandish2 => 9990,
        EffectId::Shockwave => 327970,
        EffectId::Shockwavehit => 9990,
        EffectId::Earthhit => 9990,
        EffectId::Pierceself => 9990,
        EffectId::Bowlingself => 9990,
        EffectId::Spearstabself => 9990,
        EffectId::Spearbmrself => 9990,
        EffectId::Holyhit => 1000,
        EffectId::Concentration => 9990,
        EffectId::Refineok => 9990,
        EffectId::Refinefail => 9990,
        EffectId::Jobchange => 9990,
        EffectId::Lvup => 9990,
        EffectId::Joblvup => 9990,
        EffectId::Snow => 4294967295,
        EffectId::Tamingsuccess => 9990,
        EffectId::Tamingfailed => 9990,
        EffectId::Mentalbreak => 99990,
        EffectId::Magicalatthit => 99990,
        EffectId::SuiExplosion => 99990,
        EffectId::Suicide => 99990,
        EffectId::Comboattack1 => 99990,
        EffectId::Comboattack2 => 99990,
        EffectId::Comboattack3 => 99990,
        EffectId::Comboattack4 => 99990,
        EffectId::Comboattack5 => 99990,
        EffectId::Guidedattack => 99990,
        EffectId::Poisonattack => 99990,
        EffectId::Silenceattack => 99990,
        EffectId::Stunattack => 99990,
        EffectId::Petrifyattack => 99990,
        EffectId::Sleepattack => 99990,
        EffectId::Pong => 99990,
        EffectId::Potion1 => 1000,
        EffectId::Potion2 => 1000,
        EffectId::Potion3 => 1000,
        EffectId::Potion4 => 1000,
        EffectId::Potion5 => 1000,
        EffectId::Potion6 => 1000,
        EffectId::Potion7 => 1000,
        EffectId::Potion8 => 1000,
        EffectId::Darkbreath => 1083,
        EffectId::Deffender => 99990,
        EffectId::Keeping => 99990,
        EffectId::Spellbreaker => 9990,
        EffectId::Dispell => 9990,
        EffectId::Magicrod => 4990,
        EffectId::Holycross => 4990,
        EffectId::Shieldcharge => 4990,
        EffectId::Devotion => 1500,
        EffectId::Flamelauncher => 9990,
        EffectId::Frostweapon => 9990,
        EffectId::Lightningloader => 9990,
        EffectId::Seismicweapon => 9990,
        EffectId::Chimto => 160,
        EffectId::Stealcoin => 3000,
        EffectId::Stripweapon => 3000,
        EffectId::Stripshield => 3000,
        EffectId::Striparmor => 3000,
        EffectId::Striphelm => 3000,
        EffectId::Chaincombo => 3000,
        EffectId::Demonstration => 999990,
        EffectId::PharmacyOk => 900,
        EffectId::PharmacyFail => 900,
        EffectId::Loud => 3000,
        EffectId::Joblvup50 => 2000,
        EffectId::Vallentine => 1000,
        EffectId::Vallentine2 => 1000,
        EffectId::Angel => 2000,
        EffectId::Devil => 2000,
        EffectId::Meltdown => 2500,
        EffectId::Cartboost => 1500,
        EffectId::Soulburn => 3333,
        EffectId::Soulchange => 3333,
        EffectId::Airtexture => 1000,
        EffectId::Assumptio2 => 3000,
        EffectId::NpcStop => 999990,
        EffectId::Flowercast3 => 4000,
        EffectId::Mochi => 1000,
        EffectId::Lamadan => 1000,
        EffectId::Edp => 2000,
        EffectId::Mapae => 1000,
        EffectId::Itempokjuk => 1000,
        EffectId::Ef05val => 1000,
        EffectId::Itemfast => 1000,
        EffectId::Ro2year => 2000,
        EffectId::Hflimoon1 => 1000,
        EffectId::Hflimoon2 => 1000,
        EffectId::Hflimoon3 => 1000,
        EffectId::HoUp => 1000,
        EffectId::Hamidefence => 600,
        EffectId::Hamicastle => 1000,
        EffectId::Hamiblood => 1000,
        EffectId::ItemThunder => 1667,
        EffectId::ItemCloud => 1667,
        EffectId::ItemCurse => 1667,
        EffectId::ItemZzz => 1667,
        EffectId::ItemRain => 1667,
        EffectId::M01 => 833,
        EffectId::M03 => 1667,
        EffectId::M04 => 4294967295,
        EffectId::M05 => 1667,
        EffectId::M06 => 1667,
        EffectId::M07 => 1667,
        EffectId::Food01 => 1000,
        EffectId::Food02 => 1000,
        EffectId::Food03 => 1000,
        EffectId::Food04 => 1000,
        EffectId::Food05 => 1000,
        EffectId::Food06 => 1000,
        EffectId::Firehit2 => 500,
        EffectId::NpcStop2 => 999990,
        EffectId::CookingOk => 1000,
        EffectId::CookingFail => 1000,
        EffectId::Hapgyeok => 1000,
        EffectId::Kirikage => 1000,
        EffectId::Tatami => 1000,
        EffectId::Kasumikiri => 1000,
        EffectId::Issen => 1000,
        EffectId::Kaen => 230,
        EffectId::Baku => 1000,
        EffectId::Hyousyouraku => 1000,
        EffectId::Desperado => 2000,
        EffectId::LightningS => 230,
        EffectId::BlindS => 230,
        EffectId::PoisonS => 230,
        EffectId::FreezingS => 230,
        EffectId::FlareS => 230,
        EffectId::Rapidshower => 1000,
        EffectId::Magicalbullet => 1000,
        EffectId::Spreadattack => 1000,
        EffectId::Trackcasting => 1000,
        EffectId::Tracking => 1000,
        EffectId::Tripleaction => 1000,
        EffectId::Bullseye => 1000,
        EffectId::NpcEarthquake => 1000,
        EffectId::Dragonfear => 280,
        EffectId::Wideconfuse => 280,
        EffectId::Criticalwound => 800,
        EffectId::Mapsphere => 360000,
        EffectId::PokLove => 1000,
        EffectId::PokWhite => 1000,
        EffectId::PokValen => 1000,
        EffectId::PokBirth => 1000,
        EffectId::PokChristmas => 1000,
        EffectId::MapMagiczone3 => 4294967295,
        EffectId::MapMagiczone4 => 4294967295,
        EffectId::Flowerleaf => 1000,
        EffectId::Mapsphere2 => 0,
        EffectId::Airtexture2 => 1000,
        EffectId::Airtexture3 => 1000,
        EffectId::Airtexture4 => 1000,
        EffectId::EnchantpoisonFlow => 2500,
        EffectId::GreenPop => 4294967295,
        EffectId::Levelup => 600,
        EffectId::Joblevelup => 600,
        EffectId::Npcdead => 1000,
        EffectId::ClawAtk => 380,
        EffectId::SwordLight => 600,
        EffectId::Ring4 => 4294967295,
        EffectId::Hit8 => 600,
        EffectId::Blingline => 400,
        EffectId::Blingline2 => 400,
        EffectId::Selectring => 4294967295,
        EffectId::Testeffect => 400,
        EffectId::BlowLine => 4294967295,
        EffectId::Typing => 4294967295,
        EffectId::Aldef2 => 9990,
        EffectId::Aldef3 => 9990,
        _ => 0,
    }
}
