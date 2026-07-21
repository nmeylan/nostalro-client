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
        EffectId::Warp => EffectSpec::Custom {
            duration_ms: warp::TOTAL_DURATION_MS,
        },

        EffectId::Damage1 | EffectId::Damage12 | EffectId::Damage13 => {
            EffectSpec::Custom { duration_ms: 500 }
        }

        EffectId::Magnumbreak => EffectSpec::Custom {
            duration_ms: magnum_break::TOTAL_DURATION_MS,
        },
        EffectId::Magnum2 => EffectSpec::Custom {
            duration_ms: dome_ring::MAGNUM2_TOTAL_DURATION_MS,
        },
        EffectId::GiExplosion => EffectSpec::Custom {
            duration_ms: dome_ring::GI_EXPLOSION_TOTAL_DURATION_MS,
        },

        EffectId::Thunderstorm2 => EffectSpec::Custom {
            duration_ms: thunderstorm2::TOTAL_DURATION_MS,
        },

        EffectId::M02 => EffectSpec::Custom {
            duration_ms: m_ef02::TOTAL_DURATION_MS,
        },

        EffectId::Kaizel => EffectSpec::Custom {
            duration_ms: slash::TOTAL_DURATION_MS,
        },

        EffectId::Stopeffect => EffectSpec::Custom {
            duration_ms: slash::STOPEFFECT_DURATION_MS,
        },

        EffectId::Angel2 | EffectId::Angel3 => EffectSpec::Custom {
            duration_ms: super_angel::TOTAL_DURATION_MS,
        },

        EffectId::Guard | EffectId::Guard2 | EffectId::Guard3 => EffectSpec::Custom {
            duration_ms: guard::TOTAL_DURATION_MS,
        },

        EffectId::Stormkick
        | EffectId::Stormkick1
        | EffectId::Stormkick2
        | EffectId::Stormkick3
        | EffectId::Stormkick6
        | EffectId::Stormkick7 => EffectSpec::Custom {
            duration_ms: storm_kick::TOTAL_DURATION_MS,
        },

        EffectId::Peong => EffectSpec::Custom {
            duration_ms: peong::TOTAL_DURATION_MS,
        },

        EffectId::Stormkick4 | EffectId::Stormkick5 => EffectSpec::Custom {
            duration_ms: peong_up::TOTAL_DURATION_MS,
        },

        EffectId::Chemicalprotection => EffectSpec::Custom {
            duration_ms: chemical::CHEMICALPROTECTION.total_duration_ms(),
        },
        EffectId::Mgattack2 => EffectSpec::Custom {
            duration_ms: chemical::MGATTACK2.total_duration_ms(),
        },
        EffectId::Chemical2 => EffectSpec::Custom {
            duration_ms: chemical::CHEMICAL2.total_duration_ms(),
        },
        EffectId::Chemical2dash => EffectSpec::Custom {
            duration_ms: chemical::CHEMICAL2DASH.total_duration_ms(),
        },
        EffectId::Chemical3 => EffectSpec::Custom {
            duration_ms: chemical::CHEMICAL3.total_duration_ms(),
        },
        EffectId::Chemical4 => EffectSpec::Custom {
            duration_ms: chemical::CHEMICAL4.total_duration_ms(),
        },
        EffectId::Smatk1 => EffectSpec::Custom {
            duration_ms: chemical::SMATK1.total_duration_ms(),
        },
        EffectId::Smatk2 => EffectSpec::Custom {
            duration_ms: chemical::SMATK2.total_duration_ms(),
        },
        EffectId::Smatk3 => EffectSpec::Custom {
            duration_ms: chemical::SMATK3.total_duration_ms(),
        },
        EffectId::Smatk4 => EffectSpec::Custom {
            duration_ms: chemical::SMATK4.total_duration_ms(),
        },

        EffectId::Stin => EffectSpec::Custom {
            duration_ms: stin::STIN.total_duration_ms(),
        },
        EffectId::Soulbreaker | EffectId::Soulbreaker2 => EffectSpec::Custom {
            duration_ms: soul_breaker::TOTAL_DURATION_MS,
        },
        EffectId::Teihit2 | EffectId::Backstap => EffectSpec::Custom {
            duration_ms: teihit::TOTAL_DURATION_MS,
        },

        EffectId::Tripleattack => EffectSpec::Custom {
            duration_ms: tripleattack::TRIPLEATTACK.total_duration_ms(),
        },
        EffectId::Tripleattack2 => EffectSpec::Custom {
            duration_ms: tripleattack::TRIPLEATTACK2.total_duration_ms(),
        },
        EffectId::Tripleattack3 => EffectSpec::Custom {
            duration_ms: tripleattack::TRIPLEATTACK3.total_duration_ms(),
        },

        EffectId::Spherewind => EffectSpec::Custom {
            duration_ms: spherewind::SPHEREWIND.total_duration_ms(),
        },
        EffectId::Spherewind2 => EffectSpec::Custom {
            duration_ms: spherewind::SPHEREWIND2.total_duration_ms(),
        },
        EffectId::Spherewind3 => EffectSpec::Custom {
            duration_ms: spherewind::SPHEREWIND3.total_duration_ms(),
        },
        EffectId::Baby => EffectSpec::Custom {
            duration_ms: spherewind::BABY.total_duration_ms(),
        },
        EffectId::Stin2 => EffectSpec::Custom {
            duration_ms: stin::STIN2.total_duration_ms(),
        },
        EffectId::Stin4 => EffectSpec::Custom {
            duration_ms: stin::STIN4.total_duration_ms(),
        },
        EffectId::Stin5 => EffectSpec::Custom {
            duration_ms: stin::STIN5.total_duration_ms(),
        },
        EffectId::Stin3 => EffectSpec::Custom {
            duration_ms: sma::STIN3_TOTAL_DURATION_MS,
        },
        EffectId::Sma => EffectSpec::Custom {
            duration_ms: sma::SMA_TOTAL_DURATION_MS,
        },
        EffectId::Sma2 => EffectSpec::Custom {
            duration_ms: sma::SMA2_TOTAL_DURATION_MS,
        },
        EffectId::Sma3 => EffectSpec::Custom {
            duration_ms: particle_up::SMA3_TOTAL_DURATION_MS,
        },

        EffectId::Throwitem
        | EffectId::Throwitem2
        | EffectId::Throwitem3
        | EffectId::Throwitem4
        | EffectId::Throwitem5
        | EffectId::Throwitem6
        | EffectId::Throwitem7
        | EffectId::Throwitem8
        | EffectId::Throwitem9
        | EffectId::Throwitem10 => EffectSpec::Custom {
            duration_ms: throw_item::TOTAL_DURATION_MS,
        },

        EffectId::RgCoin => EffectSpec::Custom {
            duration_ms: rg_coin::RG_COIN.total_duration_ms(),
        },
        EffectId::RgCoin2 => EffectSpec::Custom {
            duration_ms: rg_coin::RG_COIN2.total_duration_ms(),
        },
        EffectId::RgCoin3 => EffectSpec::Custom {
            duration_ms: rg_coin::RG_COIN3.total_duration_ms(),
        },

        EffectId::Intimidate => EffectSpec::Custom {
            duration_ms: rg_coin::INTIMIDATE.total_duration_ms(),
        },

        EffectId::Summonslave => EffectSpec::Custom {
            duration_ms: summon_slave::TOTAL_DURATION_MS,
        },
        EffectId::BubbleDrop => EffectSpec::Custom {
            duration_ms: bubble_drop::TOTAL_DURATION_MS,
        },
        EffectId::Cartter => EffectSpec::Custom {
            duration_ms: cartter::TOTAL_DURATION_MS,
        },
        EffectId::Icearrow => EffectSpec::Custom {
            duration_ms: magic_bolt::ICE_TOTAL_DURATION_MS,
        },

        EffectId::Tanji
        | EffectId::Tanji2
        | EffectId::Alattack1
        | EffectId::Alattack2
        | EffectId::Alattack3
        | EffectId::Alattack4
        | EffectId::Shieldboomerang
        | EffectId::Shieldboomerang2
        | EffectId::Shieldboomerang3 => EffectSpec::Custom {
            duration_ms: cloud_projectile::TOTAL_DURATION_MS,
        },

        EffectId::Twilight1 | EffectId::Twilight2 | EffectId::Twilight3 => EffectSpec::Custom {
            duration_ms: twilight::TOTAL_DURATION_MS,
        },

        EffectId::Slim | EffectId::Slim2 | EffectId::Slim3 | EffectId::Pressure => {
            EffectSpec::Custom {
                duration_ms: pressure::PRESSURE_TOTAL_DURATION_MS,
            }
        }

        EffectId::Hit1 => EffectSpec::Custom {
            duration_ms: hit::HIT1_TOTAL_DURATION_MS,
        },
        EffectId::Hit2 => EffectSpec::Custom {
            duration_ms: hit2::TOTAL_DURATION_MS,
        },
        EffectId::Hit3 => EffectSpec::Custom {
            duration_ms: hit::HIT3_TOTAL_DURATION_MS,
        },
        EffectId::Hit4 => EffectSpec::Custom {
            duration_ms: hit::HIT4_TOTAL_DURATION_MS,
        },
        EffectId::Hit5 => EffectSpec::Custom {
            duration_ms: hit5_6::HIT5_TOTAL_DURATION_MS,
        },
        EffectId::Hit6 => EffectSpec::Custom {
            duration_ms: hit5_6::HIT6_TOTAL_DURATION_MS,
        },

        EffectId::Sonicblowhit => EffectSpec::Custom {
            duration_ms: sonicblowhit::TOTAL_DURATION_MS,
        },
        EffectId::Cartrevolution => EffectSpec::Custom {
            duration_ms: cartrevolution::TOTAL_DURATION_MS,
        },
        EffectId::Napalmvalcan => EffectSpec::Custom {
            duration_ms: napalmvalcan::TOTAL_DURATION_MS,
        },

        EffectId::Stormgust => EffectSpec::Custom {
            duration_ms: stormgust::TOTAL_DURATION_MS,
        },

        EffectId::BottomSanc => EffectSpec::Custom {
            duration_ms: bottom_sanctuary_pillar::TOTAL_DURATION_MS,
        },

        EffectId::Bash => EffectSpec::Custom {
            duration_ms: bash::TOTAL_DURATION_MS,
        },

        EffectId::Hasteup => EffectSpec::Custom {
            duration_ms: hasteup::TOTAL_DURATION_MS,
        },
        EffectId::Flasher => EffectSpec::Custom {
            duration_ms: flasher::TOTAL_DURATION_MS,
        },

        EffectId::Blessing => EffectSpec::Custom {
            duration_ms: blessing::TOTAL_DURATION_MS,
        },

        EffectId::Healsp => EffectSpec::Custom {
            duration_ms: healsp::TOTAL_DURATION_MS,
        },

        EffectId::Portal => EffectSpec::Custom {
            duration_ms: portal::TOTAL_DURATION_MS,
        },

        EffectId::Portal2 | EffectId::Portal3 => EffectSpec::Custom {
            duration_ms: portal2::TOTAL_DURATION_MS,
        },

        EffectId::Portal4 | EffectId::Portal5 => EffectSpec::Custom {
            duration_ms: portal_wind::TOTAL_DURATION_MS,
        },

        EffectId::Mgdef1 | EffectId::Mgdef2 | EffectId::Mgdef3 | EffectId::Mgdef4 => {
            EffectSpec::Custom {
                duration_ms: portal_wind::TOTAL_DURATION_MS,
            }
        }

        EffectId::Halfsphere => EffectSpec::Custom {
            duration_ms: attack_energy::HALFSPHERE_DURATION_MS,
        },
        EffectId::Attackenergy => EffectSpec::Custom {
            duration_ms: attack_energy::ATTACKENERGY_DURATION_MS,
        },
        EffectId::Attackenergy2 => EffectSpec::Custom {
            duration_ms: attack_energy::ATTACKENERGY2_DURATION_MS,
        },

        EffectId::BigPortal => EffectSpec::Custom {
            duration_ms: big_portal::TOTAL_DURATION_MS,
        },
        EffectId::BigPortal2 => EffectSpec::Custom {
            duration_ms: big_portal::TOTAL_DURATION_MS_PERSISTENT,
        },

        EffectId::Readyportal => EffectSpec::Custom {
            duration_ms: ready_portal::TOTAL_DURATION_MS,
        },

        EffectId::Teleportation => EffectSpec::Custom {
            duration_ms: teleportation::TOTAL_DURATION_MS,
        },

        EffectId::Spraypond => EffectSpec::Custom {
            duration_ms: spraypond::TOTAL_DURATION_MS,
        },

        EffectId::Glasswall => EffectSpec::Custom {
            duration_ms: glasswall::TOTAL_DURATION_MS,
        },

        EffectId::Endure => EffectSpec::Custom {
            duration_ms: endure::TOTAL_DURATION_MS,
        },

        EffectId::Enhance => EffectSpec::Custom {
            duration_ms: enhance::TOTAL_DURATION_MS,
        },

        EffectId::Entry => EffectSpec::Custom {
            duration_ms: entry::TOTAL_DURATION_MS,
        },

        EffectId::Exit => EffectSpec::Custom {
            duration_ms: exit_effect::TOTAL_DURATION_MS,
        },

        EffectId::Firearrow => EffectSpec::Custom {
            duration_ms: magic_bolt::FIRE_TOTAL_DURATION_MS,
        },
        EffectId::Fireball => EffectSpec::Custom {
            duration_ms: fireball::TOTAL_DURATION_MS,
        },
        EffectId::Soulstrike => EffectSpec::Custom {
            duration_ms: soul_strike::TOTAL_DURATION_MS,
        },
        EffectId::Soulstrike2 => EffectSpec::Custom {
            duration_ms: soul_strike::TOTAL_DURATION_MS,
        },
        EffectId::Blooddrain => EffectSpec::Custom {
            duration_ms: energy_drain::BLOOD_DRAIN.total_duration_ms(),
        },
        EffectId::Energydrain => EffectSpec::Custom {
            duration_ms: energy_drain::ENERGY_DRAIN.total_duration_ms(),
        },
        EffectId::Energydrain2 => EffectSpec::Custom {
            duration_ms: energy_drain::ENERGY_DRAIN2.total_duration_ms(),
        },
        EffectId::Energydrain3 => EffectSpec::Custom {
            duration_ms: energy_drain::ENERGY_DRAIN3.total_duration_ms(),
        },
        EffectId::Yufitel => EffectSpec::Custom {
            duration_ms: yupitel::TOTAL_DURATION_MS,
        },

        EffectId::Blitzbeat => EffectSpec::Custom {
            duration_ms: blitzbeat::TOTAL_DURATION_MS,
        },
        EffectId::Waterball => EffectSpec::Custom {
            duration_ms: waterball::TOTAL_DURATION_MS,
        },
        EffectId::Fireivy => EffectSpec::Custom {
            duration_ms: fireivy::TOTAL_DURATION_MS,
        },
        EffectId::Detecting => EffectSpec::Custom {
            duration_ms: detecting::TOTAL_DURATION_MS,
        },
        EffectId::Toprank => EffectSpec::Custom {
            duration_ms: default_duration_ms(EffectId::Toprank),
        },
        EffectId::Party => EffectSpec::Custom {
            duration_ms: default_duration_ms(EffectId::Party),
        },
        EffectId::Curseattack => EffectSpec::Custom {
            duration_ms: curseattack::TOTAL_DURATION_MS,
        },

        EffectId::MapMagiczone | EffectId::MapMagiczone2 | EffectId::Glow4 => EffectSpec::Custom {
            duration_ms: mapzone::TOTAL_DURATION_MS,
        },

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
        | EffectId::Fastbluefall90 => EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        },

        EffectId::Cloud
        | EffectId::Cloud2
        | EffectId::Cloud3
        | EffectId::Cloud4
        | EffectId::Cloud5
        | EffectId::Cloud6
        | EffectId::Cloud7
        | EffectId::Cloud8 => EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        },
        EffectId::Napalmbeat => EffectSpec::Custom {
            duration_ms: napalmbeat::TOTAL_DURATION_MS,
        },
        EffectId::Sandwind => EffectSpec::Custom {
            duration_ms: sandwind::TOTAL_DURATION_MS,
        },

        EffectId::Heavensdrive => EffectSpec::Custom {
            duration_ms: heavensdrive::TOTAL_DURATION_MS,
        },
        EffectId::Bottom | EffectId::Bottom2 => EffectSpec::Custom {
            duration_ms: bottom_box::TOTAL_DURATION_MS,
        },
        EffectId::Cone => EffectSpec::Custom {
            duration_ms: cone::TOTAL_DURATION_MS,
        },
        EffectId::Flowercast => EffectSpec::Custom {
            duration_ms: flowercast::TOTAL_DURATION_MS,
        },

        EffectId::Yufitel2 => EffectSpec::Custom {
            duration_ms: yufitel2::TOTAL_DURATION_MS,
        },
        EffectId::TextureFalling => EffectSpec::Custom {
            duration_ms: texture_falling::total_duration_ms(&texture_falling::TEXTURE_FALLING),
        },

        EffectId::Twohandquicken | EffectId::Spearquicken | EffectId::Lkconcentration => {
            EffectSpec::Custom {
                duration_ms: body_buff::TOTAL_DURATION_MS,
            }
        }
        EffectId::Bunsinjyutsu => EffectSpec::Custom {
            duration_ms: body_buff::TOTAL_DURATION_MS,
        },

        EffectId::Quakebody => EffectSpec::Custom {
            duration_ms: quakebody::total_duration_ms(&quakebody::QUAKEBODY),
        },
        EffectId::Quakebody2 => EffectSpec::Custom {
            duration_ms: quakebody::total_duration_ms(&quakebody::QUAKEBODY2),
        },
        EffectId::Quakebody3 => EffectSpec::Custom {
            duration_ms: quakebody::total_duration_ms(&quakebody::QUAKEBODY3),
        },
        EffectId::Quakebody4 => EffectSpec::Custom {
            duration_ms: quakebody::total_duration_ms(&quakebody::QUAKEBODY4),
        },

        EffectId::Redbody => EffectSpec::Custom {
            duration_ms: body_tint::REDBODY.total_duration_ms(),
        },
        EffectId::Transbluebody => EffectSpec::Custom {
            duration_ms: body_tint::TRANSBLUEBODY.total_duration_ms(),
        },
        EffectId::Pinkbody => EffectSpec::Custom {
            duration_ms: body_tint::PINKBODY.total_duration_ms(),
        },
        EffectId::Linklight => EffectSpec::Custom {
            duration_ms: body_tint::LINKLIGHT.total_duration_ms(),
        },
        EffectId::Magiccrasher => EffectSpec::Custom {
            duration_ms: body_tint::MAGICCRASHER.total_duration_ms(),
        },
        EffectId::Magiccrasher2 => EffectSpec::Custom {
            duration_ms: body_tint::MAGICCRASHER2.total_duration_ms(),
        },
        EffectId::Hitbody => EffectSpec::Custom {
            duration_ms: body_tint::HITBODY.total_duration_ms(),
        },
        EffectId::Falconassault => EffectSpec::Custom {
            duration_ms: body_tint::FALCONASSAULT.total_duration_ms(),
        },

        // Tint-flicker family (colour ↔ white per frame) — Custom; aliases removed.
        EffectId::Chemicalbody => EffectSpec::Custom {
            duration_ms: body_tint::CHEMICALBODY.total_duration_ms(),
        },
        EffectId::Piercebody => EffectSpec::Custom {
            duration_ms: body_tint::PIERCEBODY.total_duration_ms(),
        },
        EffectId::Memorize => EffectSpec::Custom {
            duration_ms: body_tint::MEMORIZE.total_duration_ms(),
        },
        EffectId::Doublecastbody => EffectSpec::Custom {
            duration_ms: body_tint::DOUBLECASTBODY.total_duration_ms(),
        },
        EffectId::Greenbody => EffectSpec::Custom {
            duration_ms: body_tint::GREENBODY.total_duration_ms(),
        },
        EffectId::Shrink => EffectSpec::Custom {
            duration_ms: body_tint::SHRINK.total_duration_ms(),
        },
        EffectId::Rejectsword => EffectSpec::Custom {
            duration_ms: body_tint::REJECTSWORD.total_duration_ms(),
        },

        EffectId::Bluebody => EffectSpec::Custom {
            duration_ms: body_tint::BLUEBODY.total_duration_ms(),
        },
        EffectId::Redlightbody => EffectSpec::Custom {
            duration_ms: body_tint::REDLIGHTBODY.total_duration_ms(),
        },
        EffectId::RedHit => EffectSpec::Custom {
            duration_ms: body_tint::REDHIT.total_duration_ms(),
        },
        EffectId::BlueHit => EffectSpec::Custom {
            duration_ms: body_tint::BLUEHIT.total_duration_ms(),
        },

        EffectId::MadnessBlue => EffectSpec::Custom {
            duration_ms: body_tint::MADNESSBLUE.total_duration_ms(),
        },
        EffectId::MadnessRed => EffectSpec::Custom {
            duration_ms: body_tint::MADNESSRED.total_duration_ms(),
        },

        EffectId::Pressedbody => EffectSpec::Custom {
            duration_ms: squarebody::pressed_total_duration_ms(),
        },
        EffectId::Kickedbody => EffectSpec::Custom {
            duration_ms: squarebody::kicked_total_duration_ms(),
        },

        EffectId::Reflectbody => EffectSpec::Custom {
            duration_ms: multibody::REFLECTBODY.total_duration_ms(),
        },
        EffectId::Assumptio => EffectSpec::Custom {
            duration_ms: multibody::ASSUMPTIO.total_duration_ms(),
        },
        EffectId::Lightblade => EffectSpec::Custom {
            duration_ms: multibody::LIGHTBLADE.total_duration_ms(),
        },
        EffectId::Undeadbody => EffectSpec::Custom {
            duration_ms: multibody::UNDEADBODY.total_duration_ms(),
        },

        EffectId::Aciddemon => EffectSpec::Custom {
            duration_ms: aciddemon::TOTAL_DURATION_MS,
        },
        EffectId::Rainbow => EffectSpec::Custom {
            duration_ms: rainbow::TOTAL_DURATION_MS,
        },
        EffectId::Agiup => EffectSpec::Custom {
            duration_ms: agiup::TOTAL_DURATION_MS,
        },
        EffectId::Lightsphere => EffectSpec::Custom {
            duration_ms: light_sphere::lightsphere_duration_ms(),
        },
        EffectId::Lightsphere2 => EffectSpec::Custom {
            duration_ms: light_sphere::LIGHTSPHERE2_DURATION_MS,
        },

        EffectId::Frostdiver => EffectSpec::Custom {
            duration_ms: frost_diver::total_duration_ms(&frost_diver::FROSTDIVER),
        },
        EffectId::Frostdiver2 => EffectSpec::Custom {
            duration_ms: frost_diver::total_duration_ms(&frost_diver::FROSTDIVER2),
        },

        EffectId::Sight => EffectSpec::Custom {
            duration_ms: sight::total_duration_ms(&sight::SIGHT),
        },
        EffectId::Ruwach => EffectSpec::Custom {
            duration_ms: sight::total_duration_ms(&sight::RUWACH),
        },
        EffectId::Sight2 => EffectSpec::Custom {
            duration_ms: default_duration_ms(EffectId::Sight2),
        },

        EffectId::Incagility | EffectId::Decagility | EffectId::Incagidex => EffectSpec::Custom {
            duration_ms: status_up::TOTAL_DURATION_MS,
        },

        EffectId::Landprotector => EffectSpec::Custom {
            duration_ms: volcano::LANDPROTECTOR.total_duration_ms(),
        },
        EffectId::Volcano => EffectSpec::Custom {
            duration_ms: volcano::VOLCANO.total_duration_ms(),
        },
        EffectId::Deluge => EffectSpec::Custom {
            duration_ms: volcano::DELUGE.total_duration_ms(),
        },
        EffectId::Violentgale => EffectSpec::Custom {
            duration_ms: volcano::VIOLENTGALE.total_duration_ms(),
        },
        EffectId::Ganbantein => EffectSpec::Custom {
            duration_ms: volcano::GANBANTEIN.total_duration_ms(),
        },
        EffectId::Gumgang3 => EffectSpec::Custom {
            duration_ms: volcano::GUMGANG3.total_duration_ms(),
        },
        EffectId::Gumgang2 => EffectSpec::Custom {
            duration_ms: gumgang2::TOTAL_DURATION_MS,
        },
        EffectId::Gumgang => EffectSpec::Custom {
            duration_ms: gumgang::GUMGANG.total_duration_ms(),
        },
        EffectId::Steelbody => EffectSpec::Custom {
            duration_ms: gumgang::STEELBODY.total_duration_ms(),
        },
        EffectId::Gumgangnpc => EffectSpec::Custom {
            duration_ms: gumgang::GUMGANGNPC.total_duration_ms(),
        },
        EffectId::Doublegumgang => EffectSpec::Custom {
            duration_ms: gumgang::DOUBLE_RED.total_duration_ms(),
        },
        EffectId::Doublegumgang2 => EffectSpec::Custom {
            duration_ms: gumgang::DOUBLE_WHITE.total_duration_ms(),
        },
        EffectId::Doublegumgang3 => EffectSpec::Custom {
            duration_ms: gumgang::DOUBLE_BLUE.total_duration_ms(),
        },
        EffectId::Defender => EffectSpec::Custom {
            duration_ms: defender::TOTAL_DURATION_MS,
        },
        EffectId::Reflectshield => EffectSpec::Custom {
            duration_ms: defender::TOTAL_DURATION_MS,
        },

        EffectId::Heal => EffectSpec::Custom {
            duration_ms: heal::HEAL.total_duration_ms(),
        },
        EffectId::Heal2 => EffectSpec::Custom {
            duration_ms: heal::HEAL2.total_duration_ms(),
        },
        EffectId::Heal3 => EffectSpec::Custom {
            duration_ms: heal::SMDEF.total_duration_ms(),
        },
        EffectId::Heal4 => EffectSpec::Custom {
            duration_ms: heal::HEAL4.total_duration_ms(),
        },

        EffectId::Absorbspirits => EffectSpec::Custom {
            duration_ms: heal::ABSORBSPIRITS.total_duration_ms(),
        },
        EffectId::Exit2 => EffectSpec::Custom {
            duration_ms: heal::EXIT2.total_duration_ms(),
        },
        EffectId::Entry2 => EffectSpec::Custom {
            duration_ms: heal::ENTRY2.total_duration_ms(),
        },
        EffectId::Smdef => EffectSpec::Custom {
            duration_ms: heal::SMDEF.total_duration_ms(),
        },
        EffectId::Teleportation2 => EffectSpec::Custom {
            duration_ms: heal::TELEPORTATION2.total_duration_ms(),
        },
        EffectId::Heartcasting => EffectSpec::Custom {
            duration_ms: heartcasting::TOTAL_DURATION_MS,
        },
        EffectId::Colorpaper => EffectSpec::Custom {
            duration_ms: colorpaper::TOTAL_DURATION_MS,
        },
        EffectId::Readyportal2 => EffectSpec::Custom {
            duration_ms: portal2::READYPORTAL2_DURATION_MS,
        },
        EffectId::Couplecasting => EffectSpec::Custom {
            duration_ms: couple_casting::TOTAL_DURATION_MS,
        },
        EffectId::Gravitation => EffectSpec::Custom {
            duration_ms: gravitation::TOTAL_DURATION_MS,
        },
        EffectId::WindBuff => EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        },
        EffectId::Wind => EffectSpec::Custom {
            duration_ms: wind::TOTAL_DURATION_MS,
        },
        EffectId::Bash3d
        | EffectId::Bash3d2
        | EffectId::Bash3d3
        | EffectId::Bash3d4
        | EffectId::Bash3d5 => EffectSpec::Custom {
            duration_ms: bash3d::TOTAL_DURATION_MS,
        },
        EffectId::Truesight => EffectSpec::Custom {
            duration_ms: bash3d::TOTAL_DURATION_MS,
        },

        EffectId::Beginspell => EffectSpec::Custom {
            duration_ms: begin_spell::TOTAL_DURATION_MS,
        },
        EffectId::Aurablade => EffectSpec::Custom {
            duration_ms: aura_blade::TOTAL_DURATION_MS,
        },
        EffectId::Beginspell8 => EffectSpec::Custom {
            duration_ms: begin_spell_8::TOTAL_DURATION_MS,
        },
        EffectId::Beginspell2
        | EffectId::Beginspell3
        | EffectId::Beginspell4
        | EffectId::Beginspell5
        | EffectId::Beginspell6
        | EffectId::Beginspell7
        | EffectId::Beginspellred
        | EffectId::Beginspellwhite
        | EffectId::BeginspellN => EffectSpec::Custom {
            duration_ms: cast_circle::TOTAL_DURATION_MS,
        },

        EffectId::Beginasura
        | EffectId::Beginasura1
        | EffectId::Beginasura2
        | EffectId::Beginasura3
        | EffectId::Beginasura4
        | EffectId::Beginasura5
        | EffectId::Beginasura6
        | EffectId::Beginasura7
        | EffectId::Beginasura11 => EffectSpec::Custom {
            duration_ms: begin_asura::TOTAL_DURATION_MS,
        },

        EffectId::Soullink => EffectSpec::Custom {
            duration_ms: soullink::TOTAL_DURATION_MS,
        },

        EffectId::Grandcross | EffectId::Grandcross2 => EffectSpec::Custom {
            duration_ms: grandcross::TOTAL_DURATION_MS,
        },

        EffectId::Saintwing => EffectSpec::Custom {
            duration_ms: saintwing::TOTAL_DURATION_MS,
        },

        EffectId::Chookgi | EffectId::Chookgi2 | EffectId::Chookgi3 => EffectSpec::Custom {
            duration_ms: chookgi::TOTAL_DURATION_MS,
        },

        EffectId::Sakura | EffectId::Maple => EffectSpec::Custom {
            duration_ms: sakura::TOTAL_DURATION_MS,
        },

        EffectId::Pokjuk => EffectSpec::Custom {
            duration_ms: pokjuk::TOTAL_DURATION_MS,
        },
        EffectId::PokjukSound => EffectSpec::Custom {
            duration_ms: u32::MAX,
        },

        EffectId::Firstaid => EffectSpec::Custom {
            duration_ms: firstaid::TOTAL_DURATION_MS,
        },

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
        | EffectId::Mappillar4 => EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        },

        EffectId::Barrier => EffectSpec::Custom {
            duration_ms: barrier::TOTAL_DURATION_MS,
        },
        EffectId::Banjjakii => EffectSpec::Custom {
            duration_ms: banjjakii::TOTAL_DURATION_MS,
        },
        EffectId::Sphere => EffectSpec::Custom {
            duration_ms: orbit_burst::SPHERE_TOTAL_DURATION_MS,
        },
        EffectId::Removetrap => EffectSpec::Custom {
            duration_ms: orbit_burst::REMOVETRAP_TOTAL_DURATION_MS,
        },
        EffectId::Turnundead => EffectSpec::Custom {
            duration_ms: turnundead::TOTAL_DURATION_MS,
        },
        EffectId::Firepillaron => EffectSpec::Custom {
            duration_ms: firepillaron::TOTAL_DURATION_MS,
        },
        EffectId::Hitdark | EffectId::Darkattack => EffectSpec::Custom {
            duration_ms: hitdark::TOTAL_DURATION_MS,
        },
        EffectId::Spearbmr => EffectSpec::Custom {
            duration_ms: spearbmr::TOTAL_DURATION_MS,
        },
        EffectId::Waterball2 => EffectSpec::Custom {
            duration_ms: waterball2::TOTAL_DURATION_MS,
        },

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

        EffectId::Bowlingbash => EffectSpec::Custom {
            duration_ms: bowling_bash::TOTAL_DURATION_MS,
        },

        EffectId::Dragonsmoke => EffectSpec::Custom {
            duration_ms: dragonsmoke::TOTAL_DURATION_MS,
        },
        EffectId::Overthrust => EffectSpec::Custom {
            duration_ms: body_buff::TOTAL_DURATION_MS,
        },
        EffectId::Energycoat => EffectSpec::Custom {
            duration_ms: body_buff::TOTAL_DURATION_MS,
        },
        EffectId::Callzone => EffectSpec::Custom {
            duration_ms: callzone::TOTAL_DURATION_MS,
        },
        EffectId::Groundsample => EffectSpec::Custom {
            duration_ms: ground_sample::TOTAL_DURATION_MS,
        },

        EffectId::Potionpillar => EffectSpec::Custom {
            duration_ms: potion_pillar::TOTAL_DURATION_MS,
        },
        EffectId::Revive => EffectSpec::Custom {
            duration_ms: revive::TOTAL_DURATION_MS,
        },
        EffectId::Pierce => EffectSpec::Custom {
            duration_ms: pierce::TOTAL_DURATION_MS,
        },
        EffectId::PotionBerserk => EffectSpec::Custom {
            duration_ms: potion_berserk::TOTAL_DURATION_MS,
        },
        EffectId::PotionCon => EffectSpec::Custom {
            duration_ms: potion_con::CONCENTRATION_DURATION_MS,
        },
        EffectId::Potion => EffectSpec::Custom {
            duration_ms: potion_con::AWAKENING_DURATION_MS,
        },

        EffectId::Glasswall2 => EffectSpec::Custom {
            duration_ms: glasswall2::TOTAL_DURATION_MS,
        },
        EffectId::Providence => EffectSpec::Custom {
            duration_ms: providence::TOTAL_DURATION_MS,
        },
        EffectId::Kouenka => EffectSpec::Custom {
            duration_ms: kouenka::TOTAL_DURATION_MS,
        },

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
        | EffectId::CrystalBlue => EffectSpec::Custom {
            duration_ms: fullscreen_overlay::PERSISTENT_DURATION_MS,
        },
        EffectId::Bleeding => EffectSpec::Custom {
            duration_ms: fullscreen_overlay::PULSE_DURATION_MS,
        },

        EffectId::Linelink => EffectSpec::Custom {
            duration_ms: linelink::LINELINK_DURATION_MS,
        },
        EffectId::Linelink2 => EffectSpec::Custom {
            duration_ms: linelink::LINELINK2_DURATION_MS,
        },
        EffectId::Linelink3 => EffectSpec::Custom {
            duration_ms: linelink::LINELINK3_DURATION_MS,
        },

        _ => bucket_default(id),
    })
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
        EffectSpec::Custom {
            duration_ms: default_duration_ms(id),
        }
    } else {
        default_str_spec(id)
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
            Some(EffectSpec::Custom { .. })
        ));
    }

    #[test]
    fn weather_effects_route_to_their_procedural_impls() {
        assert!(
            matches!(effect_spec(EffectId::Maple), Some(EffectSpec::Custom { .. })),
            "Maple must reach the sakura machinery, not a frozen spr"
        );
        assert!(
            matches!(
                effect_spec(EffectId::PokjukSound),
                Some(EffectSpec::Custom { .. })
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
            Some(EffectSpec::Custom { .. })
        ));
        assert!(matches!(
            effect_spec(EffectId::Kaizel),
            Some(EffectSpec::Custom { .. })
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
            Some(EffectSpec::Custom { duration_ms: 1667 })
        ));
        assert!(matches!(
            effect_spec(EffectId::Fvoice),
            Some(EffectSpec::Custom { duration_ms: 1667 })
        ));
    }

    #[test]
    fn ghost_family_resolves_to_custom_factory_path() {
        for id in [EffectId::Ghost, EffectId::Bat, EffectId::Bat2] {
            assert!(
                matches!(
                    effect_spec(id),
                    Some(EffectSpec::Custom { duration_ms: 40000 })
                ),
                "{id:?} should resolve to Custom 40000, got {:?}",
                effect_spec(id)
            );
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
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
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
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
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
        let Some(EffectSpec::Custom { duration_ms }) = effect_spec(EffectId::Dragonsmoke) else {
            panic!("Dragonsmoke should resolve to EffectSpec::Custom");
        };
        assert_eq!(
            duration_ms,
            u32::MAX,
            "ambient loop, persists for the map's lifetime"
        );
    }

    #[test]
    fn batch2_billboards_route_to_spr_burst_variants() {
        let Some(EffectSpec::Custom { duration_ms }) = effect_spec(EffectId::Thunderstorm2) else {
            panic!("Thunderstorm2 should resolve to EffectSpec::Custom");
        };
        assert_eq!(duration_ms, thunderstorm2::TOTAL_DURATION_MS);

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
            Some(EffectSpec::Custom { .. })
        ));
    }

    #[test]
    fn warp_routes_to_factory_via_custom() {
        assert!(matches!(
            effect_spec(EffectId::Warp),
            Some(EffectSpec::Custom { .. })
        ));
    }

    #[test]
    fn ez2str_family_specs() {
        assert!(matches!(
            effect_spec(EffectId::Aurablade),
            Some(EffectSpec::Custom { .. })
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
                matches!(effect_spec(id), Some(EffectSpec::Custom { .. })),
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
        EffectId::Hit1 => 500,
        EffectId::Hit2 => 500,
        EffectId::Hit3 => 500,
        EffectId::Hit4 => 500,
        EffectId::Hit5 => 500,
        EffectId::Hit6 => 500,
        EffectId::Entry => 1000,
        EffectId::Exit => 500,
        EffectId::Warp => 800,
        EffectId::Enhance => 800,
        EffectId::Coin => 1000,
        EffectId::Endure => 800,
        EffectId::Beginspell => 400,
        EffectId::Glasswall => 99990,
        EffectId::Healsp => 500,
        EffectId::Soulstrike => 1000,
        EffectId::Bash => 400,
        EffectId::Magnumbreak => 300,
        EffectId::Steal => 500,
        EffectId::Hiding => 500,
        EffectId::Pattack => 1000,
        EffectId::Detoxication => 1000,
        EffectId::Sight => 2700,
        EffectId::Stonecurse => 9990,
        EffectId::Fireball => 1000,
        EffectId::Firewall => 400,
        EffectId::Icearrow => 1600,
        EffectId::Frostdiver => 1500,
        EffectId::Frostdiver2 => 1000,
        EffectId::Lightbolt => 2000,
        EffectId::Thunderstorm => 1040,
        EffectId::Firearrow => 1600,
        EffectId::Napalmbeat => 1000,
        EffectId::Ruwach => 9999990,
        EffectId::Teleportation => 1000,
        EffectId::Readyportal => 1000,
        EffectId::Portal => 1000,
        EffectId::Incagility => 1000,
        EffectId::Decagility => 1000,
        EffectId::Aqua => 1667,
        EffectId::Signum => 9990,
        EffectId::Angelus => 9990,
        EffectId::Blessing => 1500,
        EffectId::Incagidex => 1000,
        EffectId::Smoke => 500,
        EffectId::Firefly => 2333,
        EffectId::Sandwind => 1800,
        EffectId::Torch => u32::MAX,
        EffectId::Spraypond => 1300,
        EffectId::Firehit => 500,
        EffectId::Firesplashhit => 500,
        EffectId::Coldhit => 550,
        EffectId::Windhit => 400,
        EffectId::Poisonhit => 500,
        EffectId::Beginspell2 => 400,
        EffectId::Beginspell3 => 400,
        EffectId::Beginspell4 => 400,
        EffectId::Beginspell5 => 400,
        EffectId::Beginspell6 => 1100,
        EffectId::Beginspell7 => 400,
        EffectId::Lockon => 3333,
        EffectId::Warpzone => 2500,
        EffectId::Sightrasher => 2500,
        EffectId::Barrier => 2500,
        EffectId::Arrowshot => 9990,
        EffectId::Invenom => 9990,
        EffectId::Cure => 9990,
        EffectId::Provoke => 9990,
        EffectId::Mvp => 9990,
        EffectId::Skidtrap => 99990,
        EffectId::Brandishspear => 9990,
        EffectId::Cone => 2500,
        EffectId::Sphere => 5000,
        EffectId::Bowlingbash => 2500,
        EffectId::Icewall => 99990,
        EffectId::Gloria => 9990,
        EffectId::Magnificat => 9990,
        EffectId::Resurrection => 9990,
        EffectId::Recovery => 9990,
        EffectId::Earthspike => 2000,
        EffectId::Spearbmr => 2500,
        EffectId::Pierce => 2500,
        EffectId::Turnundead => 2500,
        EffectId::Sanctuary => 9990,
        EffectId::Impositio => 9990,
        EffectId::Lexaeterna => 9990,
        EffectId::Aspersio => 9990,
        EffectId::Lexdivina => 9990,
        EffectId::Suffragium => 9990,
        EffectId::Stormgust => 99990,
        EffectId::Lord => 99990,
        EffectId::Benedictio => 99990,
        EffectId::Meteorstorm => 99990,
        EffectId::Yufitel => 2500,
        EffectId::Yufitelhit => 2500,
        EffectId::Quagmire => 1500,
        EffectId::Firepillar => 99990,
        EffectId::Firepillarbomb => 99990,
        EffectId::Hasteup => 2500,
        EffectId::Flasher => 2500,
        EffectId::Removetrap => 700,
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
        EffectId::Bottom => 3500,
        EffectId::Blitzbeat => 2500,
        EffectId::Waterball => 2500,
        EffectId::Waterball2 => 1000,
        EffectId::Fireivy => 2500,
        EffectId::Detecting => 2500,
        EffectId::Cloaking => 2500,
        EffectId::Sonicblow => 400,
        EffectId::Sonicblowhit => 2500,
        EffectId::Grimtooth => 2500,
        EffectId::Venomdust => 99990,
        EffectId::Enchantpoison => 2500,
        EffectId::Poisonreact => 99990,
        EffectId::Poisonreact2 => 99990,
        EffectId::Overthrust => 2500,
        EffectId::Splasher => 99990,
        EffectId::Twohandquicken => 99990,
        EffectId::Autocounter => 1000,
        EffectId::Grimtoothatk => 2500,
        EffectId::Freeze => 99990,
        EffectId::Freezed => 99990,
        EffectId::Icecrash => 99990,
        EffectId::Slowpoison => 1333,
        EffectId::Bottom2 => 3500,
        EffectId::Firepillaron => 80000,
        EffectId::Sandman => 99990,
        EffectId::Revive => 2500,
        EffectId::Pneuma => 99990,
        EffectId::Heavensdrive => 2500,
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
        EffectId::Toprank => 99990,
        EffectId::Party => 99990,
        EffectId::Rain => 0,
        EffectId::Snow => 4294967295,
        EffectId::Sakura => 4294967295,
        EffectId::StatusState => 99990,
        EffectId::Banjjakii => 10000,
        EffectId::Makeblur => 99990,
        EffectId::Tamingsuccess => 9990,
        EffectId::Tamingfailed => 9990,
        EffectId::Cartrevolution => 9990,
        EffectId::Changedark => 1000,
        EffectId::Changefire => 1000,
        EffectId::Changecold => 1000,
        EffectId::Changewind => 1000,
        EffectId::Changeflame => 1000,
        EffectId::Changeearth => 1000,
        EffectId::Chaingeholy => 1000,
        EffectId::Changepoison => 1000,
        EffectId::Hitdark => 500,
        EffectId::Mentalbreak => 99990,
        EffectId::Magicalatthit => 99990,
        EffectId::SuiExplosion => 99990,
        EffectId::Darkattack => 0,
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
        EffectId::Curseattack => 1500,
        EffectId::Sleepattack => 99990,
        EffectId::Telekhit => 0,
        EffectId::Pong => 99990,
        EffectId::Level99 => 4294967295,
        EffectId::Level992 => 4294967295,
        EffectId::Level993 => 4294967295,
        EffectId::Gumgang => 999990,
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
        EffectId::Summonslave => 5000,
        EffectId::Blooddrain => 2000,
        EffectId::Energydrain => 2000,
        EffectId::PotionCon => 1990,
        EffectId::Potion => 1990,
        EffectId::PotionBerserk => 1990,
        EffectId::Potionpillar => 1500,
        EffectId::Defender => 2000,
        EffectId::Ganbantein => 3000,
        EffectId::Wind => 3000,
        EffectId::Volcano => 3000,
        EffectId::Grandcross => 9990,
        EffectId::Intimidate => 9990,
        EffectId::Chookgi => 9999990,
        EffectId::Cloud => 4294967295,
        EffectId::Cloud2 => 4294967295,
        EffectId::Mappillar => 9990,
        EffectId::Linelink => 999990,
        EffectId::Cloud3 => 4294967295,
        EffectId::Spellbreaker => 9990,
        EffectId::Dispell => 9990,
        EffectId::Deluge => 9990,
        EffectId::Violentgale => 9990,
        EffectId::Landprotector => 9990,
        EffectId::BottomVo => 299990,
        EffectId::BottomDe => 299990,
        EffectId::BottomVi => 299990,
        EffectId::BottomLa => 299990,
        EffectId::Fastmove => 99990,
        EffectId::Magicrod => 4990,
        EffectId::Holycross => 4990,
        EffectId::Shieldcharge => 4990,
        EffectId::Mappillar2 => 9990,
        EffectId::Providence => 2000,
        EffectId::Shieldboomerang => 4990,
        EffectId::Spearquicken => 99990,
        EffectId::Devotion => 1500,
        EffectId::Reflectshield => 2000,
        EffectId::Absorbspirits => 1000,
        EffectId::Steelbody => 4294967295,
        EffectId::Flamelauncher => 9990,
        EffectId::Frostweapon => 9990,
        EffectId::Lightningloader => 9990,
        EffectId::Seismicweapon => 9990,
        EffectId::Mappillar3 => 9990,
        EffectId::Mappillar4 => 9990,
        EffectId::Gumgang2 => 3000,
        EffectId::Teihit1 => 3000,
        EffectId::Gumgang3 => 3000,
        EffectId::Teihit2 => 3000,
        EffectId::Tanji => 3000,
        EffectId::Teihit1x => 3000,
        EffectId::Chimto => 160,
        EffectId::Stealcoin => 3000,
        EffectId::Stripweapon => 3000,
        EffectId::Stripshield => 3000,
        EffectId::Striparmor => 3000,
        EffectId::Striphelm => 3000,
        EffectId::Chaincombo => 3000,
        EffectId::RgCoin => 3000,
        EffectId::Backstap => 3000,
        EffectId::Teihit3 => 3000,
        EffectId::BottomDissonance => 299990,
        EffectId::BottomLullaby => 299990,
        EffectId::BottomRichmankim => 299990,
        EffectId::BottomEternalchaos => 299990,
        EffectId::BottomDrumbattlefield => 299990,
        EffectId::BottomRingnibelungen => 299990,
        EffectId::BottomRokisweil => 299990,
        EffectId::BottomIntoabyss => 299990,
        EffectId::BottomSiegfried => 299990,
        EffectId::BottomWhistle => 299990,
        EffectId::BottomAssassincross => 299990,
        EffectId::BottomPoembragi => 299990,
        EffectId::BottomAppleidun => 299990,
        EffectId::BottomUglydance => 299990,
        EffectId::BottomHumming => 299990,
        EffectId::BottomDontforgetme => 299990,
        EffectId::BottomFortunekiss => 299990,
        EffectId::BottomServiceforyou => 299990,
        EffectId::TalkFrostjoke => 9990,
        EffectId::TalkScream => 9990,
        EffectId::Pokjuk => 4294967295,
        EffectId::Throwitem => 3000,
        EffectId::Throwitem2 => 3000,
        EffectId::Chemicalprotection => 3000,
        EffectId::PokjukSound => 4294967295,
        EffectId::Demonstration => 999990,
        EffectId::Chemical2 => 3000,
        EffectId::Teleportation2 => 2000,
        EffectId::PharmacyOk => 900,
        EffectId::PharmacyFail => 900,
        EffectId::Forestlight => 600000,
        EffectId::Throwitem3 => 2000,
        EffectId::Firstaid => 2990,
        EffectId::Sprinklesand => 3000,
        EffectId::Loud => 3000,
        EffectId::Heal => 1000,
        EffectId::Heal2 => 1000,
        EffectId::Exit2 => 2000,
        EffectId::Glasswall2 => 99990,
        EffectId::Readyportal2 => 2000,
        EffectId::Portal2 => 99990,
        EffectId::BottomMag => 99990,
        EffectId::BottomSanc => 99990,
        EffectId::Heal3 => 1000,
        EffectId::Warpzone2 => 4294967295,
        EffectId::Forestlight2 => 600000,
        EffectId::Forestlight3 => 600000,
        EffectId::Forestlight4 => 600000,
        EffectId::Heal4 => 1000,
        EffectId::Foot => 3400,
        EffectId::Foot2 => 3400,
        EffectId::Beginasura => 10000,
        EffectId::Tripleattack => 1000,
        EffectId::Hitline => 2000,
        EffectId::Hptime => 3000,
        EffectId::Sptime => 3000,
        EffectId::Maple => 4294967295,
        EffectId::Blind => 4294967295,
        EffectId::Poison => 4294967295,
        EffectId::Guard => 2000,
        EffectId::Joblvup50 => 2000,
        EffectId::Angel2 => 2000,
        EffectId::Magnum2 => 1000,
        EffectId::Callzone => 30000,
        EffectId::Portal3 => 99990,
        EffectId::Couplecasting => 10000,
        EffectId::Heartcasting => 10000,
        EffectId::Entry2 => 1000,
        EffectId::Saintwing => 4294967295,
        EffectId::Spherewind => 4294967295,
        EffectId::Colorpaper => 99990,
        EffectId::Lightsphere => 10000,
        EffectId::Waterfall => 4294967295,
        EffectId::Waterfall90 => 4294967295,
        EffectId::WaterfallSmall => 4294967295,
        EffectId::WaterfallSmall90 => 4294967295,
        EffectId::WaterfallT2 => 4294967295,
        EffectId::WaterfallT290 => 4294967295,
        EffectId::WaterfallSmallT2 => 4294967295,
        EffectId::WaterfallSmallT290 => 4294967295,
        EffectId::MiniTetris => 4294967295,
        EffectId::Ghost => 40000,
        EffectId::Bat => 40000,
        EffectId::Bat2 => 40000,
        EffectId::Soulbreaker => 3000,
        EffectId::Level994 => 4294967295,
        EffectId::Vallentine => 1000,
        EffectId::Vallentine2 => 1000,
        EffectId::Pressure => 3000,
        EffectId::Bash3d => 2000,
        EffectId::Aurablade => 1000,
        EffectId::Redbody => 999990,
        EffectId::Lkconcentration => 999990,
        EffectId::BottomGospel => 199990,
        EffectId::Angel => 2000,
        EffectId::Devil => 2000,
        EffectId::Dragonsmoke => 500,
        EffectId::BottomBasilica => 299990,
        EffectId::Assumptio => 999990,
        EffectId::Hitline2 => 2000,
        EffectId::Bash3d2 => 2000,
        EffectId::Energydrain2 => 2000,
        EffectId::Transbluebody => 2000,
        EffectId::Magiccrasher => 1000,
        EffectId::Lightsphere2 => 4294967295,
        EffectId::Lightblade => 4294967295,
        EffectId::Energydrain3 => 2000,
        EffectId::Linelink2 => 2000,
        EffectId::Linklight => 2000,
        EffectId::Truesight => 2500,
        EffectId::Falconassault => 2000,
        EffectId::Tripleattack2 => 2000,
        EffectId::Portal4 => 2000,
        EffectId::Meltdown => 2500,
        EffectId::Cartboost => 1500,
        EffectId::Rejectsword => 2000,
        EffectId::Tripleattack3 => 2000,
        EffectId::Spherewind2 => 999990,
        EffectId::Linelink3 => 4294967295,
        EffectId::Pinkbody => 4294967295,
        EffectId::Level995 => 4294967295,
        EffectId::Level996 => 4294967295,
        EffectId::Bash3d3 => 2000,
        EffectId::Bash3d4 => 2000,
        EffectId::Napalmvalcan => 2000,
        EffectId::Portal5 => 2000,
        EffectId::Magiccrasher2 => 1000,
        EffectId::BottomSpider => 299990,
        EffectId::BottomFogwall => 299990,
        EffectId::Soulburn => 3333,
        EffectId::Soulchange => 3333,
        EffectId::Baby => 2000,
        EffectId::Soulbreaker2 => 3000,
        EffectId::Rainbow => 3000,
        EffectId::Peong => 5500,
        EffectId::Tanji2 => 3000,
        EffectId::Pressedbody => 500,
        EffectId::Spinedbody => 500,
        EffectId::Kickedbody => 1000,
        EffectId::Airtexture => 1000,
        EffectId::Hitbody => 1000,
        EffectId::Doublegumgang => 4294967295,
        EffectId::Reflectbody => 4294967295,
        EffectId::Babybody => 999990,
        EffectId::Babybody2 => 999990,
        EffectId::Giantbody => 999990,
        EffectId::Giantbody2 => 999990,
        EffectId::Asurabody => 999990,
        EffectId::Ef4waybody => 1000,
        EffectId::Quakebody => 140,
        EffectId::AsurabodyMonster => 999990,
        EffectId::Hitline3 => 2000,
        EffectId::Hitline4 => 2500,
        EffectId::Hitline5 => 2500,
        EffectId::Hitline6 => 3000,
        EffectId::Electric => 3000,
        EffectId::Electric2 => 1500,
        EffectId::Hitline7 => 1500,
        EffectId::Stormkick => 1000,
        EffectId::Halfsphere => 2000,
        EffectId::Attackenergy => 30000,
        EffectId::Attackenergy2 => 1000,
        EffectId::Chemical3 => 3000,
        EffectId::Assumptio2 => 3000,
        EffectId::Bluecasting => 600,
        EffectId::Run => 99990,
        EffectId::Stoprun => 300,
        EffectId::Stopeffect => 1000,
        EffectId::Jumpbody => 5000,
        EffectId::Landbody => 600,
        EffectId::Foot3 => 3400,
        EffectId::Foot4 => 3400,
        EffectId::TaeReady => 850,
        EffectId::Grandcross2 => 9990,
        EffectId::Soulstrike2 => 1000,
        EffectId::Yufitel2 => 2500,
        EffectId::NpcStop => 999990,
        EffectId::Darkcasting => 600,
        EffectId::Gumgangnpc => 15000,
        EffectId::Agiup => 1000,
        EffectId::Jumpkick => 1000,
        EffectId::Quakebody2 => 350,
        EffectId::Stormkick1 => 1000,
        EffectId::Stormkick2 => 1000,
        EffectId::Stormkick3 => 1000,
        EffectId::Stormkick4 => 2000,
        EffectId::Stormkick5 => 3000,
        EffectId::Stormkick6 => 1000,
        EffectId::Stormkick7 => 1000,
        EffectId::Spinedbody2 => 900,
        EffectId::Beginasura1 => 3000,
        EffectId::Beginasura2 => 3000,
        EffectId::Beginasura3 => 3000,
        EffectId::Beginasura4 => 3000,
        EffectId::Beginasura5 => 3000,
        EffectId::Beginasura6 => 3000,
        EffectId::Beginasura7 => 3000,
        EffectId::Aurablade2 => aura_blade::TOTAL_DURATION_MS,
        EffectId::Devil1 => 4294967295,
        EffectId::Devil2 => 4294967295,
        EffectId::Devil3 => 4294967295,
        EffectId::Devil4 => 4294967295,
        EffectId::Devil5 => 4294967295,
        EffectId::Devil6 => 4294967295,
        EffectId::Devil7 => 4294967295,
        EffectId::Devil8 => 4294967295,
        EffectId::Devil9 => 4294967295,
        EffectId::Devil10 => 4294967295,
        EffectId::Doublegumgang2 => 999990,
        EffectId::Doublegumgang3 => 999990,
        EffectId::Blackdevil => 2000,
        EffectId::Flowercast => 4000,
        EffectId::Flowercast2 => 4000,
        EffectId::Flowercast3 => 4000,
        EffectId::Mochi => 1000,
        EffectId::Lamadan => 1000,
        EffectId::Edp => 2000,
        EffectId::Shieldboomerang2 => 4990,
        EffectId::RgCoin2 => 3000,
        EffectId::Guard2 => 2000,
        EffectId::Slim => 2500,
        EffectId::Slim2 => 2500,
        EffectId::Slim3 => 2500,
        EffectId::Chemicalbody => 1200,
        EffectId::Castspin => 1000,
        EffectId::Piercebody => 2000,
        EffectId::Soullink => 5000,
        EffectId::Chookgi2 => 9999990,
        EffectId::Memorize => 2000,
        EffectId::Soullight => 1000,
        EffectId::Mapae => 1000,
        EffectId::Itempokjuk => 1000,
        EffectId::Ef05val => 1000,
        EffectId::Beginasura11 => 10000,
        EffectId::Night => 100,
        EffectId::Chemical2dash => 3000,
        EffectId::Groundsample => 30000,
        EffectId::GiExplosion => 3000,
        EffectId::Cloud4 => 4294967295,
        EffectId::Cloud5 => 4294967295,
        EffectId::BottomHermode => 299990,
        EffectId::Cartter => 2000,
        EffectId::Itemfast => 1000,
        EffectId::Shieldboomerang3 => 4990,
        EffectId::Doublecastbody => 1200,
        EffectId::Gravitation => 20000,
        EffectId::Tarotcard1 => 4067,
        EffectId::Tarotcard2 => 4067,
        EffectId::Tarotcard3 => 4067,
        EffectId::Tarotcard4 => 4067,
        EffectId::Tarotcard5 => 4067,
        EffectId::Tarotcard6 => 4067,
        EffectId::Tarotcard7 => 4067,
        EffectId::Tarotcard8 => 4067,
        EffectId::Tarotcard9 => 4067,
        EffectId::Tarotcard10 => 4067,
        EffectId::Tarotcard11 => 4067,
        EffectId::Tarotcard12 => 4067,
        EffectId::Tarotcard13 => 4067,
        EffectId::Tarotcard14 => 4067,
        EffectId::Aciddemon => 2000,
        EffectId::Greenbody => 1200,
        EffectId::Throwitem4 => 3000,
        EffectId::BabybodyBack => 500,
        EffectId::Throwitem5 => 3000,
        EffectId::Bluebody => 2000,
        EffectId::Hated => 5000,
        EffectId::Redlightbody => 4294967295,
        EffectId::Ro2year => 2000,
        EffectId::SmaReady => 2000,
        EffectId::Stin => 2000,
        EffectId::RedHit => 600,
        EffectId::BlueHit => 600,
        EffectId::Quakebody3 => 600,
        EffectId::Sma => 1000,
        EffectId::Sma2 => 1000,
        EffectId::Stin2 => 2000,
        EffectId::Hittexture => 2000,
        EffectId::Stin3 => 2000,
        EffectId::Sma3 => 500,
        EffectId::Bluefall => 4294967295,
        EffectId::Bluefall90 => 4294967295,
        EffectId::Fastbluefall => 4294967295,
        EffectId::Fastbluefall90 => 4294967295,
        EffectId::BigPortal => 20000,
        EffectId::BigPortal2 => 4294967295,
        EffectId::ScreenQuake => 2000,
        EffectId::Homuncasting => 600,
        EffectId::Hflimoon1 => 1000,
        EffectId::Hflimoon2 => 1000,
        EffectId::Hflimoon3 => 1000,
        EffectId::HoUp => 1000,
        EffectId::Hamidefence => 600,
        EffectId::Hamicastle => 1000,
        EffectId::Hamiblood => 1000,
        EffectId::Hated2 => 3000,
        EffectId::Twilight1 => 3000,
        EffectId::Twilight2 => 3000,
        EffectId::Twilight3 => 3000,
        EffectId::ItemThunder => 1667,
        EffectId::ItemCloud => 1667,
        EffectId::ItemCurse => 1667,
        EffectId::ItemZzz => 1667,
        EffectId::ItemRain => 1667,
        EffectId::ItemLight => 3000,
        EffectId::Angel3 => 2000,
        EffectId::M01 => 833,
        EffectId::M02 => 1667,
        EffectId::M03 => 1667,
        EffectId::M04 => 4294967295,
        EffectId::M05 => 1667,
        EffectId::M06 => 1667,
        EffectId::M07 => 1667,
        EffectId::Kaizel => 2000,
        EffectId::Kaahi => 2000,
        EffectId::Cloud6 => 4294967295,
        EffectId::Food01 => 1000,
        EffectId::Food02 => 1000,
        EffectId::Food03 => 1000,
        EffectId::Food04 => 1000,
        EffectId::Food05 => 1000,
        EffectId::Food06 => 1000,
        EffectId::Shrink => 2000,
        EffectId::Throwitem6 => 2000,
        EffectId::Sight2 => 9999990,
        EffectId::Quakebody4 => 600,
        EffectId::Firehit2 => 500,
        EffectId::NpcStop2 => 999990,
        EffectId::NpcStop2Del => 20,
        EffectId::Fvoice => 1667,
        EffectId::Wink => 1667,
        EffectId::CookingOk => 1000,
        EffectId::CookingFail => 1000,
        EffectId::TempOk => 1667,
        EffectId::TempFail => 1667,
        EffectId::Hapgyeok => 1000,
        EffectId::Throwitem7 => 2000,
        EffectId::Throwitem8 => 2000,
        EffectId::Throwitem9 => 2000,
        EffectId::Throwitem10 => 2000,
        EffectId::Bunsinjyutsu => 99990,
        EffectId::Kouenka => 3000,
        EffectId::Hyousensou => 2000,
        EffectId::BottomSuiton => 299990,
        EffectId::Stin4 => 2000,
        EffectId::Thunderstorm2 => 3333,
        EffectId::Chemical4 => 3000,
        EffectId::Stin5 => 2000,
        EffectId::MadnessBlue => 600,
        EffectId::MadnessRed => 600,
        EffectId::RgCoin3 => 3000,
        EffectId::Bash3d5 => 2000,
        EffectId::Chookgi3 => 9999990,
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
        EffectId::MapMagiczone => 4294967295,
        EffectId::MapMagiczone2 => 4294967295,
        EffectId::Damage1 => 500,
        EffectId::Damage12 => 500,
        EffectId::Damage13 => 500,
        EffectId::Undeadbody => 4294967295,
        EffectId::UndeadbodyDel => 400,
        EffectId::GreenNumber => 500,
        EffectId::BlueNumber => 500,
        EffectId::RedNumber => 500,
        EffectId::PurpleNumber => 500,
        EffectId::BlackNumber => 500,
        EffectId::WhiteNumber => 500,
        EffectId::YellowNumber => 500,
        EffectId::PinkNumber => 500,
        EffectId::BubbleDrop => 2000,
        EffectId::NpcEarthquake => 1000,
        EffectId::DaSpace => 4294967295,
        EffectId::Dragonfear => 280,
        EffectId::Bleeding => 700,
        EffectId::Wideconfuse => 280,
        EffectId::BottomRunner => 299990,
        EffectId::BottomTransfer => 299990,
        EffectId::CrystalBlue => 4294967295,
        EffectId::BottomEvilland => 299990,
        EffectId::Guard3 => 2000,
        EffectId::NpcSlowcast => 3100,
        EffectId::Criticalwound => 800,
        EffectId::Green993 => 4294967295,
        EffectId::Green995 => 4294967295,
        EffectId::Green996 => 4294967295,
        EffectId::Mapsphere => 360000,
        EffectId::PokLove => 1000,
        EffectId::PokWhite => 1000,
        EffectId::PokValen => 1000,
        EffectId::PokBirth => 1000,
        EffectId::PokChristmas => 1000,
        EffectId::MapMagiczone3 => 4294967295,
        EffectId::MapMagiczone4 => 4294967295,
        EffectId::Dust => 4294967295,
        EffectId::TorchRed => 4294967295,
        EffectId::TorchGreen => 4294967295,
        EffectId::MapGhost => 4294967295,
        EffectId::Glow1 => 4294967295,
        EffectId::Glow2 => 4294967295,
        EffectId::Glow4 => 4294967295,
        EffectId::TorchPurple => 4294967295,
        EffectId::Cloud7 => 4294967295,
        EffectId::Cloud8 => 4294967295,
        EffectId::Flowerleaf => 1000,
        EffectId::Mapsphere2 => 0,
        EffectId::Glow11 => 4294967295,
        EffectId::Glow12 => 4294967295,
        EffectId::Foot5 => 3400,
        EffectId::Foot6 => 3400,
        EffectId::Airtexture2 => 1000,
        EffectId::Airtexture3 => 1000,
        EffectId::Airtexture4 => 1000,
        EffectId::CodeEffectBegin => 0,
        EffectId::Makeblur3 => 99990,
        EffectId::Makeblur4 => 99990,
        EffectId::BloodFly => 500,
        EffectId::Hit7 => 500,
        EffectId::Teihit1reverse => 350,
        EffectId::Teihit2reverse => 350,
        EffectId::Makeblur5 => 4294967295,
        EffectId::EnchantpoisonFlow => 2500,
        EffectId::ArrowYellow => 4294967295,
        EffectId::ArrowRed => 4294967295,
        EffectId::Sight3 => 4294967295,
        EffectId::Teihit3reverse => 350,
        EffectId::Beginspellwhite => 4294967295,
        EffectId::Beginspell8 => 400,
        EffectId::EndureZhan => 800,
        EffectId::EndureSou => 800,
        EffectId::EndureShan => 800,
        EffectId::EndureJing => 800,
        EffectId::WindBuff => 4294967295,
        EffectId::Beginspellred => 400,
        EffectId::GreenPop => 4294967295,
        EffectId::BeginspellN => 4294967295,
        EffectId::ArrowDown => 4294967295,
        EffectId::CodeEffectEnd => 0,
        EffectId::Process2Begin => 0,
        EffectId::TextureFalling => 3000,
        EffectId::Spherewind3 => 4294967295,
        EffectId::Process2End => 0,
        EffectId::FileEffectBegin => 0,
        EffectId::Shake => 0,
        EffectId::Levelup => 600,
        EffectId::Joblevelup => 600,
        EffectId::Npcdead => 1000,
        EffectId::ClawAtk => 380,
        EffectId::SwordLight => 600,
        EffectId::Ring4 => 4294967295,
        EffectId::Hit8 => 600,
        EffectId::CastMagicRed => 4294967295,
        EffectId::CastMagicRed2 => 4294967295,
        EffectId::CastMagicBlue => 4294967295,
        EffectId::CastMagicBlue2 => 4294967295,
        EffectId::CastMagicWhite => 4294967295,
        EffectId::CastMagicWhite2 => 4294967295,
        EffectId::CastMagicYellow => 4294967295,
        EffectId::CastMagicYellow2 => 4294967295,
        EffectId::Flammule => 4294967295,
        EffectId::Blingline => 400,
        EffectId::Blingline2 => 400,
        EffectId::Groundimage => 4294967295,
        EffectId::Groundimage3 => 4294967295,
        EffectId::Groundimage5 => 4294967295,
        EffectId::Groundimage7 => 4294967295,
        EffectId::Groundimage9 => 4294967295,
        EffectId::Code2EffectBegin => 0,
        EffectId::Castflower => 4200,
        EffectId::Rotateflower => 3000,
        EffectId::Flyup => 4294967295,
        EffectId::ActorColor => 4294967295,
        EffectId::LightSword => 4294967295,
        EffectId::LightBody => 4294967295,
        EffectId::LightRide => 4294967295,
        EffectId::PrintFoot => 4294967295,
        EffectId::ColorSword => 4294967295,
        EffectId::ColorBody => 4294967295,
        EffectId::ColorRide => 4294967295,
        EffectId::MoveToSprite => 4294967295,
        EffectId::GetItem => 1800,
        EffectId::LightRoleshield => 4294967295,
        EffectId::LightHead1 => 4294967295,
        EffectId::LightHead2 => 4294967295,
        EffectId::LightHead3 => 4294967295,
        EffectId::ColorHead1 => 4294967295,
        EffectId::ColorHead2 => 4294967295,
        EffectId::ColorHead3 => 4294967295,
        EffectId::CodeEffectBegin2 => 0,
        EffectId::RippleYellow => 4294967295,
        EffectId::RippleBlackk => 4294967295,
        EffectId::RippleWhite => 4294967295,
        EffectId::RippleRed => 4294967295,
        EffectId::RipplePurple => 4294967295,
        EffectId::AggregationYellow => 4294967295,
        EffectId::AggregationBlackk => 4294967295,
        EffectId::AggregationWhite => 4294967295,
        EffectId::AggregationRed => 4294967295,
        EffectId::AggregationPurple => 4294967295,
        EffectId::CodeEffectEnd2 => 0,
        EffectId::TestEffectBegin => 0,
        EffectId::Selectring => 4294967295,
        EffectId::Testeffect => 400,
        EffectId::Testbodylight => 4294967295,
        EffectId::ZoomIn => 4294967295,
        EffectId::ZoomOut => 4294967295,
        EffectId::BlowLine => 4294967295,
        EffectId::LightShield => 4294967295,
        EffectId::Typing => 4294967295,
        EffectId::Smatk1 => 1000,
        EffectId::Smatk2 => 1000,
        EffectId::Smatk3 => 1000,
        EffectId::Smatk4 => 1000,
        EffectId::Smdef => 1000,
        EffectId::Mgattack1 => 200,
        EffectId::Mgattack2 => 200,
        EffectId::Alattack1 => 3000,
        EffectId::Alattack2 => 3000,
        EffectId::Alattack3 => 3000,
        EffectId::Alattack4 => 3000,
        EffectId::Aldef2 => 9990,
        EffectId::Aldef3 => 9990,
        EffectId::Mgdef1 => 2000,
        EffectId::Mgdef2 => 2000,
        EffectId::Mgdef3 => 2000,
        EffectId::Mgdef4 => 2000,
        EffectId::DevilRed => 750,
        EffectId::Decagilitybuf => 4294967295,
        EffectId::Energycoat => 3000,
        EffectId::Venomdust2 => 99990,
    }
}
