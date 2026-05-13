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
            Box::new(effects::bottom_sanc::BottomSancEffect::new(attach))
        }
        EffectId::Warpzone => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_BURST,
        )),
        EffectId::Warpzone2 => Box::new(effects::warp_zone::WarpZoneEffect::new(
            attach,
            effects::warp_zone::PARAMS_SUSTAINED,
        )),
        EffectId::Landprotector => Box::new(effects::land_protector::LandProtectorEffect::new(attach)),
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
