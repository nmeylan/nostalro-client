//! Bulk classification of `EffectId`s per the original game's behaviour.
//!
//!
//! * **Custom bucket** — original game's dispatch emits PP_* primitives but
//!   has no STR file (407 ids). Plus the 12 StrHybrid ids that emit *both*
//!   an STR animation and PP_* primitives — both flavours render as
//!   `EffectSpec::Custom`; the factory wraps hybrids so an STR overlay plays
//!   alongside the placeholder.
//! * **Noop bucket** — original game has neither STR load nor PP_* dispatch
//!   (235 ids). Pass-through / status-marker / no-op packet hooks. Rendered
//!   as `EffectSpec::Noop`; viewers exclude them.
//! * **Hybrid (12)** — strict subset of the Custom bucket; flagged separately
//!   so the factory can pick `HybridPlaceholderEffect` over `PlaceholderEffect`.
//!
//! Everything not in Custom or Noop falls through to the default STR path.

use models::enums::effect_id::EffectId;

/// `true` if the original game dispatches this id to PP_* primitives —
/// includes pure custom (407) and StrHybrid (12). Total: 419.
pub fn is_custom_bucket(id: EffectId) -> bool {
    matches!(
        id,
        // Pure custom (407)
        EffectId::Ef05val | EffectId::Absorbspirits | EffectId::Aciddemon
            | EffectId::Agiup | EffectId::Airtexture | EffectId::Airtexture2
            | EffectId::Airtexture3 | EffectId::Airtexture4 | EffectId::Alattack1
            | EffectId::Alattack2 | EffectId::Alattack3 | EffectId::Alattack4
            | EffectId::Angel2 | EffectId::Angel3 | EffectId::Aqua
            | EffectId::Attackenergy | EffectId::Attackenergy2 | EffectId::Baby
            | EffectId::Backstap | EffectId::Banjjakii | EffectId::Barrier
            | EffectId::Bash | EffectId::Bash3d | EffectId::Bash3d2
            | EffectId::Bash3d3 | EffectId::Bash3d4 | EffectId::Bash3d5
            | EffectId::Bat | EffectId::Bat2 | EffectId::Beginspell
            | EffectId::Blackdevil
            | EffectId::Giantbody | EffectId::Giantbody2
            | EffectId::Bluecasting | EffectId::Darkcasting
            | EffectId::Electric | EffectId::Electric2
            | EffectId::Hitline | EffectId::Hitline2
            | EffectId::Hitline3 | EffectId::Hitline4
            | EffectId::Hitline5 | EffectId::Hitline6
            | EffectId::Hitline7
            | EffectId::BigPortal | EffectId::BigPortal2 | EffectId::Blessing
            | EffectId::Blind | EffectId::BlindS | EffectId::Blitzbeat
            | EffectId::Blooddrain | EffectId::BlowLine | EffectId::Bluefall
            | EffectId::Bluefall90 | EffectId::Bottom | EffectId::Bottom2
            | EffectId::BottomAppleidun | EffectId::BottomAssassincross
            | EffectId::BottomBasilica | EffectId::BottomDe
            | EffectId::BottomDissonance | EffectId::BottomDontforgetme
            | EffectId::BottomDrumbattlefield | EffectId::BottomEternalchaos
            | EffectId::BottomEvilland | EffectId::BottomFogwall
            | EffectId::BottomFortunekiss | EffectId::BottomGospel
            | EffectId::BottomHermode | EffectId::BottomHumming
            | EffectId::BottomIntoabyss | EffectId::BottomLa
            | EffectId::BottomLullaby | EffectId::BottomMag
            | EffectId::BottomPoembragi | EffectId::BottomRichmankim
            | EffectId::BottomRingnibelungen | EffectId::BottomRokisweil
            | EffectId::BottomRunner | EffectId::BottomSanc
            | EffectId::BottomServiceforyou | EffectId::BottomSiegfried
            | EffectId::BottomSpider | EffectId::BottomSuiton
            | EffectId::BottomTransfer | EffectId::BottomUglydance
            | EffectId::BottomVi | EffectId::BottomVo | EffectId::BottomWhistle
            | EffectId::Bowlingbash | EffectId::BubbleDrop | EffectId::Callzone
            | EffectId::Cartter | EffectId::Chemical2 | EffectId::Chemical2dash
            | EffectId::Chemical3 | EffectId::Chemical4
            | EffectId::Chemicalprotection | EffectId::Chimto | EffectId::Chookgi
            | EffectId::Cloud | EffectId::Cloud2 | EffectId::Cloud3
            | EffectId::Cloud4 | EffectId::Cloud5 | EffectId::Cloud6
            | EffectId::Cloud7 | EffectId::Cloud8 | EffectId::Colorpaper
            | EffectId::Cone | EffectId::CrystalBlue | EffectId::Curseattack
            | EffectId::Darkattack | EffectId::Darkbreath | EffectId::Decagility
            | EffectId::Defender | EffectId::Deluge | EffectId::Demonstration
            | EffectId::Desperado | EffectId::Detecting | EffectId::Detoxication
            | EffectId::Devil1 | EffectId::Devil10 | EffectId::Devil2
            | EffectId::Devil3 | EffectId::Devil4 | EffectId::Devil5
            | EffectId::Devil6 | EffectId::Devil7 | EffectId::Devil8
            | EffectId::Devil9 | EffectId::DevilRed | EffectId::Doublegumgang
            | EffectId::Doublegumgang2 | EffectId::Doublegumgang3
            | EffectId::Dragonsmoke | EffectId::Dust | EffectId::Earthspike
            | EffectId::Edp | EffectId::Enchantpoison | EffectId::EnchantpoisonFlow
            | EffectId::Endure | EffectId::Energydrain | EffectId::Energydrain2
            | EffectId::Energydrain3 | EffectId::Enhance | EffectId::Entry
            | EffectId::Entry2 | EffectId::Exit | EffectId::Exit2
            | EffectId::Fastbluefall | EffectId::Fastbluefall90
            | EffectId::Firearrow | EffectId::Fireball | EffectId::Firefly
            | EffectId::Fireivy | EffectId::Firepillaron | EffectId::Firstaid
            | EffectId::FlareS | EffectId::Flasher | EffectId::Flowercast
            | EffectId::Flowercast3 | EffectId::Foot | EffectId::Foot2
            | EffectId::Foot3 | EffectId::Foot4 | EffectId::Foot5
            | EffectId::Foot6 | EffectId::Forestlight | EffectId::Forestlight2
            | EffectId::Forestlight3 | EffectId::Forestlight4
            | EffectId::FreezingS | EffectId::Frostdiver | EffectId::Frostdiver2
            | EffectId::Fvoice | EffectId::Ganbantein | EffectId::Ghost
            | EffectId::GiExplosion | EffectId::Glow1 | EffectId::Glow11
            | EffectId::Glow12 | EffectId::Glow2 | EffectId::Glow4
            | EffectId::Grandcross | EffectId::Grandcross2 | EffectId::Gravitation
            | EffectId::Grimtooth | EffectId::Grimtoothatk | EffectId::Groundsample
            | EffectId::Guard | EffectId::Guard2 | EffectId::Guard3
            | EffectId::Gumgang | EffectId::Gumgang2 | EffectId::Gumgang3
            | EffectId::Gumgangnpc | EffectId::Halfsphere | EffectId::Hamiblood
            | EffectId::Hamicastle | EffectId::Hasteup | EffectId::Hated
            | EffectId::Hated2 | EffectId::Healsp | EffectId::Heartcasting
            | EffectId::Heavensdrive | EffectId::Hit1 | EffectId::Hit2
            | EffectId::Hit3 | EffectId::Hit4 | EffectId::Hit5
            | EffectId::Hit6 | EffectId::Hitdark | EffectId::Hittexture
            | EffectId::Tarotcard1 | EffectId::Tarotcard2 | EffectId::Tarotcard3
            | EffectId::Tarotcard4 | EffectId::Tarotcard5 | EffectId::Tarotcard6
            | EffectId::Tarotcard7 | EffectId::Tarotcard8 | EffectId::Tarotcard9
            | EffectId::Tarotcard10 | EffectId::Tarotcard11 | EffectId::Tarotcard12
            | EffectId::Tarotcard13 | EffectId::Tarotcard14
            | EffectId::Hptime | EffectId::Hyousensou | EffectId::Icearrow
            | EffectId::Icewall | EffectId::Incagidex | EffectId::Incagility
            | EffectId::Intimidate | EffectId::Issen | EffectId::Itemfast
            | EffectId::ItemCloud | EffectId::ItemCurse | EffectId::ItemLight
            | EffectId::ItemRain | EffectId::ItemThunder | EffectId::ItemZzz
            | EffectId::Kaen | EffectId::Kasumikiri | EffectId::Kirikage
            | EffectId::Kouenka | EffectId::Landprotector | EffectId::Level99
            | EffectId::Level992 | EffectId::Level993 | EffectId::Level995
            | EffectId::Level996 | EffectId::LightningS | EffectId::Lightsphere
            | EffectId::Lightsphere2 | EffectId::Linelink | EffectId::Linelink2
            | EffectId::Linelink3 | EffectId::M01 | EffectId::M02
            | EffectId::M03 | EffectId::M04 | EffectId::M05
            | EffectId::M06 | EffectId::M07 | EffectId::Magicalbullet
            | EffectId::Magnum2 | EffectId::Magnumbreak | EffectId::Maple
            | EffectId::Mappillar | EffectId::Mappillar2 | EffectId::Mappillar3
            | EffectId::Mappillar4 | EffectId::Mapsphere | EffectId::Mapsphere2
            | EffectId::MapGhost | EffectId::MapMagiczone | EffectId::MapMagiczone2
            | EffectId::MapMagiczone3 | EffectId::MapMagiczone4
            | EffectId::Mgattack2 | EffectId::Mgdef1 | EffectId::Mgdef2
            | EffectId::Mgdef3 | EffectId::Mgdef4 | EffectId::Napalmbeat
            | EffectId::Napalmvalcan | EffectId::NpcEarthquake
            | EffectId::NpcSlowcast | EffectId::NpcStop | EffectId::NpcStop2
            | EffectId::Overthrust | EffectId::Party | EffectId::Pattack
            | EffectId::Peong | EffectId::Pierce | EffectId::Poison
            | EffectId::PoisonS | EffectId::Pokjuk | EffectId::PokBirth
            | EffectId::PokChristmas | EffectId::PokLove | EffectId::PokValen
            | EffectId::PokWhite | EffectId::Portal | EffectId::Portal2
            | EffectId::Portal3 | EffectId::Portal4 | EffectId::Portal5
            | EffectId::Potionpillar | EffectId::Pressure | EffectId::Rainbow
            | EffectId::Rapidshower | EffectId::Reflectshield | EffectId::Removetrap
            | EffectId::Revive | EffectId::RgCoin | EffectId::RgCoin2
            | EffectId::RgCoin3 | EffectId::Ro2year | EffectId::Ruwach
            | EffectId::Saintwing | EffectId::Sakura | EffectId::Sandwind
            | EffectId::Shieldboomerang | EffectId::Shieldboomerang2
            | EffectId::Shieldboomerang3 | EffectId::Sight | EffectId::Sight2
            | EffectId::Slim | EffectId::Slim2 | EffectId::Slim3
            | EffectId::Sma | EffectId::Sma2 | EffectId::Sma3
            | EffectId::Smatk1 | EffectId::Smatk2 | EffectId::Smatk3
            | EffectId::Smatk4 | EffectId::SmaReady | EffectId::Smdef
            | EffectId::Smoke | EffectId::Snow | EffectId::Sonicblow
            | EffectId::Sonicblowhit | EffectId::Soulbreaker | EffectId::Soulbreaker2
            | EffectId::Soulstrike | EffectId::Soulstrike2 | EffectId::Spearbmr
            | EffectId::Sphere | EffectId::Spherewind | EffectId::Spherewind2
            | EffectId::Spraypond | EffectId::Spreadattack | EffectId::Sprinklesand
            | EffectId::Sptime | EffectId::Steal | EffectId::Steelbody
            | EffectId::Stin | EffectId::Stin2 | EffectId::Stin3
            | EffectId::Stin4 | EffectId::Stin5 | EffectId::Stopeffect
            | EffectId::Stormkick | EffectId::Stormkick1 | EffectId::Stormkick2
            | EffectId::Stormkick3 | EffectId::Stormkick4 | EffectId::Stormkick5
            | EffectId::Stormkick6 | EffectId::Stormkick7 | EffectId::Summonslave
            | EffectId::Tanji | EffectId::Tanji2 | EffectId::Tatami
            | EffectId::Teihit1 | EffectId::Teihit1x | EffectId::Teihit2
            | EffectId::Teihit3 | EffectId::Teleportation2 | EffectId::TempFail
            | EffectId::TempOk | EffectId::Testeffect | EffectId::TextureFalling
            | EffectId::Throwitem | EffectId::Throwitem10 | EffectId::Throwitem2
            | EffectId::Throwitem3 | EffectId::Throwitem4 | EffectId::Throwitem5
            | EffectId::Throwitem6 | EffectId::Throwitem7 | EffectId::Throwitem8
            | EffectId::Throwitem9 | EffectId::Thunderstorm2 | EffectId::Toprank
            | EffectId::Torch | EffectId::TorchGreen | EffectId::TorchPurple
            | EffectId::TorchRed | EffectId::Tracking | EffectId::Tripleaction
            | EffectId::Tripleattack | EffectId::Tripleattack2 | EffectId::Tripleattack3
            | EffectId::Truesight | EffectId::Turnundead | EffectId::Twilight1
            | EffectId::Twilight2 | EffectId::Twilight3 | EffectId::Typing
            | EffectId::Vallentine | EffectId::Vallentine2 | EffectId::Violentgale
            | EffectId::Volcano | EffectId::Warp | EffectId::Waterball
            | EffectId::Waterball2 | EffectId::Waterfall | EffectId::Waterfall90
            | EffectId::WaterfallSmall | EffectId::WaterfallSmall90
            | EffectId::WaterfallSmallT2 | EffectId::WaterfallSmallT290
            | EffectId::WaterfallT2 | EffectId::WaterfallT290 | EffectId::Wind
            | EffectId::WindBuff | EffectId::Wink | EffectId::Yufitel
            | EffectId::Yufitel2 | EffectId::Yufitelhit
            // StrHybrid (12) — also dispatched to PP_* primitives
            | EffectId::Coin | EffectId::Glasswall | EffectId::Thunderstorm
            | EffectId::Aspersio | EffectId::Stormgust | EffectId::Cartrevolution
            | EffectId::PotionBerserk | EffectId::Providence | EffectId::Glasswall2
            | EffectId::Joblvup50 | EffectId::Hapgyeok | EffectId::Bleeding
    )
}

/// `true` if the original game emits **both** an STR file and PP_* primitives
/// for this id (12 ids). Strict subset of [`is_custom_bucket`]; lets the
/// factory pick the hybrid-placeholder variant so the STR plays alongside.
pub fn is_hybrid(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Coin
            | EffectId::Glasswall
            | EffectId::Thunderstorm
            | EffectId::Aspersio
            | EffectId::Stormgust
            | EffectId::Cartrevolution
            | EffectId::PotionBerserk
            | EffectId::Providence
            | EffectId::Glasswall2
            | EffectId::Joblvup50
            | EffectId::Hapgyeok
            | EffectId::Bleeding
    )
}

/// `true` if the original game has neither sprintf-STR nor PP_* dispatch
/// for this id (235 ids). Rendered as `EffectSpec::Noop` so viewers can
/// exclude them and the holder skips the spawn.
pub fn is_noop_bucket(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Ef4waybody | EffectId::ActorColor | EffectId::AggregationBlackk
            | EffectId::AggregationPurple | EffectId::AggregationRed
            | EffectId::AggregationWhite | EffectId::AggregationYellow
            | EffectId::ArrowDown | EffectId::ArrowRed | EffectId::ArrowYellow
            | EffectId::Assumptio | EffectId::Asurabody | EffectId::AsurabodyMonster
            | EffectId::Aurablade | EffectId::Aurablade2 | EffectId::Babybody
            | EffectId::Babybody2 | EffectId::BabybodyBack | EffectId::Beginasura
            | EffectId::Beginasura1 | EffectId::Beginasura11 | EffectId::Beginasura2
            | EffectId::Beginasura3 | EffectId::Beginasura4 | EffectId::Beginasura5
            | EffectId::Beginasura6 | EffectId::Beginasura7 | EffectId::Beginspell2
            | EffectId::Beginspell3 | EffectId::Beginspell4 | EffectId::Beginspell5
            | EffectId::Beginspell6 | EffectId::Beginspell7 | EffectId::Beginspell8
            | EffectId::Beginspellred | EffectId::Beginspellwhite
            | EffectId::BeginspellN | EffectId::BlackNumber
            | EffectId::Blastmine | EffectId::BloodFly | EffectId::Bluebody
            | EffectId::BlueHit | EffectId::BlueNumber
            | EffectId::Bunsinjyutsu | EffectId::Castflower | EffectId::Castspin
            | EffectId::CastMagicBlue | EffectId::CastMagicBlue2
            | EffectId::CastMagicRed | EffectId::CastMagicRed2
            | EffectId::CastMagicWhite | EffectId::CastMagicWhite2
            | EffectId::CastMagicYellow | EffectId::CastMagicYellow2
            | EffectId::Chaingeholy | EffectId::Changecold | EffectId::Changedark
            | EffectId::Changeearth | EffectId::Changefire | EffectId::Changeflame
            | EffectId::Changepoison | EffectId::Changewind | EffectId::Chemicalbody
            | EffectId::Chookgi2 | EffectId::Chookgi3 | EffectId::Cloaking
            | EffectId::Code2EffectBegin | EffectId::CodeEffectBegin
            | EffectId::CodeEffectBegin2 | EffectId::CodeEffectEnd
            | EffectId::CodeEffectEnd2 | EffectId::Coldhit | EffectId::ColorBody
            | EffectId::ColorHead1 | EffectId::ColorHead2 | EffectId::ColorHead3
            | EffectId::ColorRide | EffectId::ColorSword | EffectId::Couplecasting
            | EffectId::Damage1 | EffectId::Damage12 | EffectId::Damage13
            | EffectId::DaSpace | EffectId::Decagilitybuf
            | EffectId::Doublecastbody
            | EffectId::EndureJing | EffectId::EndureShan | EffectId::EndureSou
            | EffectId::EndureZhan | EffectId::Falconassault | EffectId::Fastmove
            | EffectId::FileEffectBegin | EffectId::Firesplashhit
            | EffectId::Flammule | EffectId::Flowercast2 | EffectId::Flyup
            | EffectId::GetItem
            | EffectId::Green993 | EffectId::Green995 | EffectId::Green996
            | EffectId::Greenbody | EffectId::GreenNumber | EffectId::Groundimage
            | EffectId::Groundimage3 | EffectId::Groundimage5 | EffectId::Groundimage7
            | EffectId::Groundimage9 | EffectId::Heal | EffectId::Heal2
            | EffectId::Heal3 | EffectId::Heal4 | EffectId::Hiding
            | EffectId::Hit7 | EffectId::Hitbody
            | EffectId::Homuncasting | EffectId::Jumpbody | EffectId::Jumpkick
            | EffectId::Kaahi | EffectId::Kaizel | EffectId::Kickedbody
            | EffectId::Landbody | EffectId::Level994 | EffectId::Lightblade
            | EffectId::LightBody | EffectId::LightHead1 | EffectId::LightHead2
            | EffectId::LightHead3 | EffectId::LightRide | EffectId::LightRoleshield
            | EffectId::LightShield | EffectId::LightSword | EffectId::Linklight
            | EffectId::Lockon | EffectId::MadnessBlue | EffectId::MadnessRed
            | EffectId::Magiccrasher | EffectId::Magiccrasher2 | EffectId::Makeblur
            | EffectId::Makeblur3 | EffectId::Makeblur4 | EffectId::Makeblur5
            | EffectId::Memorize | EffectId::Mgattack1 | EffectId::MiniTetris
            | EffectId::MoveToSprite | EffectId::Night | EffectId::NpcStop2Del
            | EffectId::Piercebody | EffectId::Pinkbody | EffectId::PinkNumber
            | EffectId::PokjukSound | EffectId::Pressedbody
            | EffectId::PrintFoot | EffectId::Process2Begin | EffectId::Process2End
            | EffectId::PurpleNumber | EffectId::Quakebody | EffectId::Quakebody2
            | EffectId::Quakebody3 | EffectId::Quakebody4 | EffectId::Rain
            | EffectId::Readyportal | EffectId::Readyportal2 | EffectId::Redbody
            | EffectId::Redlightbody | EffectId::RedHit | EffectId::RedNumber
            | EffectId::Reflectbody | EffectId::RippleBlackk | EffectId::RipplePurple
            | EffectId::RippleRed | EffectId::RippleWhite | EffectId::RippleYellow
            | EffectId::Rotateflower | EffectId::Run | EffectId::ScreenQuake
            | EffectId::Shake | EffectId::Shrink | EffectId::Sight3
            | EffectId::Sightrasher | EffectId::Soullight | EffectId::Soullink
            | EffectId::Spherewind3 | EffectId::Spinedbody | EffectId::Spinedbody2
            | EffectId::StatusState | EffectId::Stoprun | EffectId::TaeReady
            | EffectId::TalkFrostjoke | EffectId::TalkScream
            | EffectId::Teihit1reverse | EffectId::Teihit2reverse
            | EffectId::Teihit3reverse | EffectId::Telekhit | EffectId::Teleportation
            | EffectId::Testbodylight | EffectId::TestEffectBegin
            | EffectId::Transbluebody | EffectId::Undeadbody | EffectId::UndeadbodyDel
            | EffectId::Warpzone | EffectId::Warpzone2 | EffectId::WhiteNumber
            | EffectId::YellowNumber | EffectId::ZoomIn | EffectId::ZoomOut
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_disjoint() {
        // An effect can't be both Custom and Noop. Hybrid is a strict
        // subset of Custom.
        let samples = [
            EffectId::Warp,
            EffectId::Stormgust,
            EffectId::Hiding,
            EffectId::Beginasura,
            EffectId::Coin,
        ];
        for id in samples {
            assert!(
                !(is_custom_bucket(id) && is_noop_bucket(id)),
                "{:?} marked both Custom and Noop",
                id
            );
            if is_hybrid(id) {
                assert!(is_custom_bucket(id), "{:?} hybrid but not in custom bucket", id);
            }
        }
    }

    #[test]
    fn classifies_known_examples() {
        assert!(is_custom_bucket(EffectId::Warp));
        assert!(is_custom_bucket(EffectId::Aciddemon));
        assert!(is_custom_bucket(EffectId::Stormgust));
        assert!(is_hybrid(EffectId::Stormgust));
        assert!(is_hybrid(EffectId::Coin));
        assert!(!is_hybrid(EffectId::Warp));
        assert!(is_noop_bucket(EffectId::Hiding));
        assert!(is_noop_bucket(EffectId::Beginasura));
        assert!(is_noop_bucket(EffectId::Warpzone));
        assert!(!is_noop_bucket(EffectId::Warp));
        assert!(!is_custom_bucket(EffectId::Hiding));
    }
}
