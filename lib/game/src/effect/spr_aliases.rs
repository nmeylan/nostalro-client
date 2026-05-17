//! GRF SPR-sprite descriptors per `EffectId`.
//!
//! Parallel to [`super::str_aliases`]: each implemented effect returns a
//! [`SprDef`] carrying the canonical GRF path (without the `.spr`/`.act`
//! extension), plus its rendered size, animation speed and repeat flag
//! tuned to match the original game.
//! Returning `None` means "this id is not an SPR-billboard effect" — the
//! caller then routes the id through the regular Custom/STR/Noop
//! fall-through.

use models::enums::effect_id::EffectId;

/// Per-id SPR billboard parameters tuned to match the original game.
///
/// The original game's particle constructor sets anim speed = 4
/// and repeat-anim = true by default. Effects that never call the
/// corresponding setter inherit those defaults, so [`SprDef::new`] mirrors
/// them. Callers override only what the effect's recipe actually changes.
#[derive(Clone, Copy, Debug)]
pub struct SprDef {
    pub sprite: &'static str,
    pub size_scale: f32,
    pub anim_speed: f32,
    /// `true` = loop the .act motions.
    /// `false` = play once and hold the final motion until the effect's
    /// duration_ms expires.
    pub repeat: bool,
}

impl SprDef {
    const fn new(sprite: &'static str) -> Self {
        Self {
            sprite,
            size_scale: 1.0,
            anim_speed: 4.0,
            repeat: true,
        }
    }
    const fn with_size(mut self, size_scale: f32) -> Self {
        self.size_scale = size_scale;
        self
    }
    const fn with_anim_speed(mut self, anim_speed: f32) -> Self {
        self.anim_speed = anim_speed;
        self
    }
    const fn one_shot(mut self) -> Self {
        self.repeat = false;
        self
    }
}

pub fn spr_def(id: EffectId) -> Option<SprDef> {
    Some(match id {
        // Torch: the original reads anim speed from param[1] (clamped to ≥1). The
        // ambient torch spawned by the client never sets its param, so the
        // clamp picks 1.0.
        EffectId::Torch => SprDef::new("data/sprite/이팩트/torch_01").with_anim_speed(1.0),
        // Maple: original game uses a weather sakura primitive; approximate with a
        // looping single Spr — animation cadence inherits the constructor
        // default since there's no direct equivalent.
        EffectId::Maple => SprDef::new("data/sprite/이팩트/단풍"),
        // Aqua: anim speed 2, no repeat, in the original game.
        EffectId::Aqua => SprDef::new("data/sprite/이팩트/아쿠아플레이")
            .with_anim_speed(2.0)
            .one_shot(),
        // Vallentine action 0: anim speed 2, plays once.
        EffectId::Vallentine => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot(),
        // Dragonsmoke: the original game's `DragonSmoke()` never sets anim speed so the
        // Constructor default (4) sticks. Size 1.5.
        // Tilt/drift aren't reproduced (renderer is axis-aligned), so the
        // puff appears static — acceptable.
        EffectId::Dragonsmoke => SprDef::new("data/sprite/이팩트/굴뚝연기").with_size(1.5),
        // PoisonHit: use-org-argb, size=1.5, anim speed=2,
        // no anim repeat. Without it the .act loops and
        // re-renders the impact instead of holding the final smoke puffs.
        EffectId::Poisonhit => SprDef::new("data/sprite/이팩트/poisonhit")
            .with_size(1.5)
            .with_anim_speed(2.0)
            .one_shot(),
        _ => return None,
    })
}
