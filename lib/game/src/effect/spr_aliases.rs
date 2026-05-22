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
/// The defaults are anim speed 4, looping motions, and a white (no-tint)
/// colour — the same baseline the original game's effects start from.
/// Effects that never change one of these inherit the default, so
/// [`SprDef::new`] reproduces them. Callers override only what the effect's
/// recipe actually changes.
#[derive(Clone, Copy, Debug)]
pub struct SprDef {
    pub sprite: &'static str,
    pub size_scale: f32,
    pub anim_speed: f32,
    /// `true` = loop the .act motions.
    /// `false` = play once and hold the final motion until the effect's
    /// duration_ms expires.
    pub repeat: bool,
    /// RGBA multiplier applied per-pixel. `[1.0; 4]` = no tint (the sprite's
    /// original colours). Effects that zero a channel — e.g. DarkBreath
    /// rendering pure red — populate this.
    pub tint: [f32; 4],
}

impl SprDef {
    const fn new(sprite: &'static str) -> Self {
        Self {
            sprite,
            size_scale: 1.0,
            anim_speed: 4.0,
            repeat: true,
            tint: [1.0, 1.0, 1.0, 1.0],
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
    const fn with_tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }
}

pub fn spr_def(id: EffectId) -> Option<SprDef> {
    Some(match id {
        // Torch: anim speed comes from an effect parameter clamped to ≥1.
        // The ambient torch spawned by the client never sets it, so the
        // clamp picks 1.0.
        EffectId::Torch => SprDef::new("data/sprite/이팩트/torch_01").with_anim_speed(1.0),
        // Maple: the original game scatters falling petals; approximate with a
        // looping single Spr — animation cadence inherits the default
        // since there's no direct equivalent.
        EffectId::Maple => SprDef::new("data/sprite/이팩트/단풍"),
        // Aqua: anim speed 2, plays once (no repeat).
        EffectId::Aqua => SprDef::new("data/sprite/이팩트/아쿠아플레이")
            .with_anim_speed(2.0)
            .one_shot(),
        // Vallentine action 0: anim speed 2, plays once.
        EffectId::Vallentine => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::Blessing => SprDef::new("data/sprite/이팩트/축복").one_shot(),
        EffectId::Demonstration => SprDef::new("data/sprite/이팩트/데몬스트레이션").one_shot(),
        EffectId::NpcStop => SprDef::new("data/sprite/이팩트/스톱"),
        EffectId::Hamicastle => SprDef::new("data/sprite/이팩트/캐슬링").one_shot(),
        EffectId::Hamiblood => SprDef::new("data/sprite/이팩트/블러드러스트").one_shot(),
        EffectId::Kirikage => SprDef::new("data/sprite/이팩트/그림자베기").one_shot(),
        EffectId::Tatami => SprDef::new("data/sprite/이팩트/다다미 뒤집기").one_shot(),
        EffectId::Kasumikiri => SprDef::new("data/sprite/이팩트/안개베기").one_shot(),
        EffectId::Issen => SprDef::new("data/sprite/이팩트/일섬").one_shot(),
        EffectId::Kaen => SprDef::new("data/sprite/이팩트/화염진"),
        EffectId::Desperado => SprDef::new("data/sprite/이팩트/데스페라도").one_shot(),
        EffectId::LightningS => SprDef::new("data/sprite/아이템/라이트닝스피어").one_shot(),
        EffectId::BlindS => SprDef::new("data/sprite/아이템/블라인드스피어").one_shot(),
        EffectId::PoisonS => SprDef::new("data/sprite/아이템/포이즌스피어").one_shot(),
        EffectId::FreezingS => SprDef::new("data/sprite/아이템/프리징스피어").one_shot(),
        EffectId::FlareS => SprDef::new("data/sprite/아이템/플레어스피어").one_shot(),
        EffectId::Rapidshower => SprDef::new("data/sprite/이팩트/래피드샤워").one_shot(),
        EffectId::Magicalbullet => SprDef::new("data/sprite/이팩트/매지컬불릿").one_shot(),
        EffectId::Spreadattack => SprDef::new("data/sprite/이팩트/스프레드").one_shot(),
        EffectId::Tracking => SprDef::new("data/sprite/이팩트/트래킹").one_shot(),
        EffectId::Tripleaction => SprDef::new("data/sprite/이팩트/트리플액션").one_shot(),
        EffectId::NpcEarthquake => SprDef::new("data/sprite/이팩트/어스퀘이크").one_shot(),
        EffectId::PokLove => SprDef::new("data/sprite/이팩트/폭죽_러브").one_shot(),
        EffectId::PokBirth => SprDef::new("data/sprite/이팩트/폭죽_생일").one_shot(),
        EffectId::PokChristmas => SprDef::new("data/sprite/이팩트/폭죽_크리스마스").one_shot(),
        // Dragonsmoke: original game `DragonSmoke()` never calls SetAnimSpeed so the
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
        // DarkBreath: zero the green / blue channels so the sprite renders
        // pure red. Size 0.8, anim speed 1, duration 65 frames (overrides
        // the table value of 500). Fade-out from frame 60 isn't reproduced
        // yet — the renderer holds full alpha until the holder kills the
        // effect at duration.
        EffectId::Darkbreath => SprDef::new("data/sprite/이팩트/darkbreath")
            .with_size(0.8)
            .with_anim_speed(1.0)
            .with_tint([1.0, 0.0, 0.0, 1.0]),
        // Thunderstorm2: use-original-argb tint, size 2.5, anim speed 2,
        // looping for the full master duration. The original game
        // overlays this on the standard thunder_storm STR, but the
        // master switch routes by id so the SPR-only branch covers the
        // gun-skill variant the server emits.
        EffectId::Thunderstorm2 => SprDef::new("data/sprite/이팩트/thunder_storm")
            .with_size(2.5)
            .with_anim_speed(2.0),
        _ => return None,
    })
}
