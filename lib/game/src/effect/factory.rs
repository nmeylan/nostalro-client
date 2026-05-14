//! Single dispatch point from [`EffectId`] to a concrete [`Effect`]
//! implementation. The match below grows as effects get implemented;
//! [`make_effect`] returns `None` for ids that don't have one yet, and
//! callers report them as `CustomNotImpl`.

use super::effect_id::EffectId;
use super::effect_trait::Effect;
use super::effects;
use super::spec::Attach;

/// Build a concrete custom-effect instance. Returns `None` for ids that
/// aren't implemented yet — callers (the holder spawn path) should log and
/// report `CustomNotImpl`.
pub fn make_effect(id: EffectId, attach: Attach) -> Option<Box<dyn Effect>> {
    Some(match id {
        EffectId::Warp => Box::new(effects::warp::WarpEffect::new(attach)),
        EffectId::Magnumbreak => {
            Box::new(effects::magnum_break::MagnumBreakEffect::new(attach))
        }
        EffectId::BottomSanc => {
            Box::new(effects::bottom_sanctuary_pillar::BottomSanctuaryPillarEffect::new(attach))
        }
        EffectId::Warpzone => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_BURST,
        )),
        EffectId::Warpzone2 => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_SUSTAINED,
        )),
        EffectId::Landprotector => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::LANDPROTECTOR,
        )),
        EffectId::Volcano => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::VOLCANO,
        )),
        EffectId::Deluge => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::DELUGE,
        )),
        EffectId::Violentgale => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::VIOLENTGALE,
        )),
        EffectId::Ganbantein => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::GANBANTEIN,
        )),
        EffectId::Gumgang3 => Box::new(effects::volcano::VolcanoEffect::new(
            attach,
            effects::volcano::GUMGANG3,
        )),

        EffectId::Level99 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_LARGE,
        )),
        EffectId::Level992 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_MIDDLE,
        )),
        EffectId::Level993 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_BOTTOM,
        )),
        EffectId::Level994 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT,
        )),
        EffectId::Level995 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT_MIDDLE,
        )),
        EffectId::Level996 => Box::new(effects::aura::AuraEffect::new(
            attach,
            effects::aura::LV99_TRANSCENDANT_BOTTOM,
        )),

        EffectId::Beginspell => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::YELLOW,
        )),
        EffectId::Beginspell2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WATER,
        )),
        EffectId::Beginspell3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::FIRE,
        )),
        EffectId::Beginspell4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WIND,
        )),
        EffectId::Beginspell5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::EARTH,
        )),
        EffectId::Beginspell6 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::HOLY,
        )),
        EffectId::Beginspell7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::POISON,
        )),
        EffectId::Beginspellred => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::RED,
        )),
        EffectId::Beginspellwhite => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::WHITE,
        )),
        EffectId::BeginspellN => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::N_BLUE,
        )),
        EffectId::Beginasura => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA,
        )),
        EffectId::Beginasura1 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_EARTH,
        )),
        EffectId::Beginasura2 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_WIND,
        )),
        EffectId::Beginasura3 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_WATER,
        )),
        EffectId::Beginasura4 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_FIRE,
        )),
        EffectId::Beginasura5 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_UNDEAD,
        )),
        EffectId::Beginasura6 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_SHADOW,
        )),
        EffectId::Beginasura7 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_HOLY,
        )),
        EffectId::Beginasura11 => Box::new(effects::cast_circle::CastCircleEffect::new(
            attach,
            effects::cast_circle::ASURA_CHAMPION,
        )),
        _ => return None,
    })
}

/// `make_effect` predicate without paying for construction. Same arms as
/// `make_effect`; keep them in sync.
pub fn is_implemented(id: EffectId) -> bool {
    matches!(
        id,
        EffectId::Warp
            | EffectId::Magnumbreak
            | EffectId::BottomSanc
            | EffectId::Warpzone
            | EffectId::Warpzone2
            | EffectId::Landprotector
            | EffectId::Volcano
            | EffectId::Deluge
            | EffectId::Violentgale
            | EffectId::Ganbantein
            | EffectId::Gumgang3
            | EffectId::Level99
            | EffectId::Level992
            | EffectId::Level993
            | EffectId::Level994
            | EffectId::Level995
            | EffectId::Level996
            | EffectId::Beginspell
            | EffectId::Beginspell2
            | EffectId::Beginspell3
            | EffectId::Beginspell4
            | EffectId::Beginspell5
            | EffectId::Beginspell6
            | EffectId::Beginspell7
            | EffectId::Beginspellred
            | EffectId::Beginspellwhite
            | EffectId::BeginspellN
            | EffectId::Beginasura
            | EffectId::Beginasura1
            | EffectId::Beginasura2
            | EffectId::Beginasura3
            | EffectId::Beginasura4
            | EffectId::Beginasura5
            | EffectId::Beginasura6
            | EffectId::Beginasura7
            | EffectId::Beginasura11
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warp_dispatches() {
        let e = make_effect(EffectId::Warp, Attach::WorldPos([0.0; 3]));
        assert!(e.is_some());
        assert!(is_implemented(EffectId::Warp));
    }

    #[test]
    fn unimplemented_returns_none() {
        // Pick a definitely-unimplemented id.
        assert!(make_effect(EffectId::Hit2, Attach::WorldPos([0.0; 3])).is_none());
        assert!(!is_implemented(EffectId::Hit2));
    }
}
