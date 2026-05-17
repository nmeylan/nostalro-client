//! GRF SPR-sprite candidates per `EffectId`.
//!
//! Parallel to [`super::str_aliases`]: each entry returns at least one GRF path
//! (without the `.spr`/`.act` extension). The first is the canonical path used
//! as the cache key in `EffectSpec::Spr { sprite, .. }`; subsequent entries are
//! fallbacks for the SPR loader to try (none used yet).
//!
//! Returning `&[]` means "this id is not an SPR-billboard effect" — `effect_spec`
//! then routes the id through the regular Custom/STR/Noop fall-through.

use models::enums::effect_id::EffectId;

pub fn spr_aliases(id: EffectId) -> &'static [&'static str] {
    match id {
        EffectId::Torch => &["data/sprite/이팩트/torch_01"],
        EffectId::Maple => &["data/sprite/이팩트/단풍"],
        EffectId::Aqua => &["data/sprite/이팩트/아쿠아플레이"],
        // Original game Vallentine(0): uses original vertex color, action 0,
        // MT_ONETIME — small red heart pop.
        EffectId::Vallentine => &["data/sprite/이팩트/vallentine"],
        // Original game's DragonSmoke: single particle w/ chimneysmoke sprite,
        // random tilt + drift. Tilt/drift aren't reproduced (renderer is
        // axis-aligned), so the puff appears static — acceptable.
        EffectId::Dragonsmoke => &["data/sprite/이팩트/굴뚝연기"],
        _ => &[],
    }
}
