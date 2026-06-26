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
    /// Y-offset in world units (negative = upward).
    pub pos_y: f32,
    /// ACT action index to play. Most SPR effects play action 0, but
    /// siblings that
    /// share one sprite (e.g. Vallentine vs Vallentine2) differ only by action.
    pub action: usize,
}

impl SprDef {
    const fn new(sprite: &'static str) -> Self {
        Self {
            sprite,
            size_scale: 1.0,
            anim_speed: 4.0,
            repeat: true,
            tint: [1.0, 1.0, 1.0, 1.0],
            pos_y: 0.0,
            action: 0,
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
    const fn with_pos_y(mut self, pos_y: f32) -> Self {
        self.pos_y = pos_y;
        self
    }
    const fn with_action(mut self, action: usize) -> Self {
        self.action = action;
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
        EffectId::Aqua => SprDef::new("data/sprite/이팩트/성수뜨기")
            .with_anim_speed(2.0)
            .one_shot()
            .with_pos_y(-20.0),
        // Vallentine action 0: anim speed 2, plays once.
        EffectId::Vallentine => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot(),
        // Vallentine2: same sprite as Vallentine, ACT action 1.
        EffectId::Vallentine2 => SprDef::new("data/sprite/이팩트/vallentine")
            .with_anim_speed(2.0)
            .one_shot()
            .with_action(1),
        // Itemfast: winged-boots sprite, anim speed 4, one-shot.
        EffectId::Itemfast => SprDef::new("data/sprite/이팩트/fast")
            .with_anim_speed(4.0)
            .one_shot(),
        EffectId::Blessing => SprDef::new("data/sprite/이팩트/축복").one_shot(),
        // Demonstration: the .act motion (action 0, 16 frames) loops over
        // the master duration. Size 1.2, alpha 200/255, y-offset -1.0,
        // anim speed 4 (the default), matching the original game's
        // on-screen look.
        EffectId::Demonstration => SprDef::new("data/sprite/이팩트/데몬스트레이션")
            .with_size(1.2)
            .with_pos_y(-1.0),
        EffectId::NpcStop => SprDef::new("data/sprite/이팩트/스톱"),
        // NpcStop2: the Close Confine ground sprite. Unlike NpcStop it uses
        // `cconfine.spr`, alpha 100, faster anim (12), no y-offset, and
        // plays once then holds the last frame for the effect's persistent
        // duration.
        EffectId::NpcStop2 => SprDef::new("data/sprite/이팩트/cconfine")
            .with_anim_speed(12.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 100.0 / 255.0]),
        // Wink is a Custom effect (`effects/wink.rs`), not a `spr_def` — it
        // picks one of wink.spr's four directional actions from the camera
        // angle, which the data-driven Spr path can't do.
        // Hamicastle: anim speed 2, plays once, default tint. The sprite
        // path is the EUC-KR transliteration of the original game's
        // kaeseulring sprite.
        EffectId::Hamicastle => SprDef::new("data/sprite/이팩트/캐슬링")
            .with_anim_speed(2.0)
            .one_shot(),
        // Item status billboards. Same setup as the other item-status
        // entries: anim speed 2, plays once, default tint. A static
        // one-shot billboard with no drift, so a held one-shot Spr
        // reproduces them.
        EffectId::ItemThunder => SprDef::new("data/sprite/이팩트/item_thunder")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemCloud => SprDef::new("data/sprite/이팩트/item_cloud")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemCurse => SprDef::new("data/sprite/이팩트/item_curse")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemZzz => SprDef::new("data/sprite/이팩트/item_zzz")
            .with_anim_speed(2.0)
            .one_shot(),
        EffectId::ItemRain => SprDef::new("data/sprite/이팩트/item_rain")
            .with_anim_speed(2.0)
            .one_shot(),
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
        // Firework banners — same setup as PokBirth above (anim speed 4,
        // plays once). The white-day / valentine firework sprites live
        // under the Korean `폭죽_<word>` resource names in the classic
        // GRF.
        EffectId::PokWhite => SprDef::new("data/sprite/이팩트/폭죽_화이트데이").one_shot(),
        EffectId::PokValen => SprDef::new("data/sprite/이팩트/폭죽_발렌타인").one_shot(),
        // Dragonsmoke is routed through `spr_burst_params` because it
        // drifts upward with a per-frame yaw spin and fades out at 2/3 of
        // its lifetime — none of which a static `SprDef` reproduces. See
        // `spr_burst.rs::spr_burst_params`.
        //
        // PoisonHit: size 1.5, anim speed 2, plays once. Without one-shot
        // the .act loops and re-renders the impact instead of holding the
        // final smoke puffs.
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
            // raise the particle onto the victim's head; native RO −Y is up.
            // (matches where the original game anchors this effect)
            .with_pos_y(-20.0)
            .with_tint([1.0, 0.0, 0.0, 1.0]),
        // Thunderstorm2: its dedicated thunder-storm sprite is a
        // renewal-era addition not present in the classic GRF. We route
        // this id to the `thunderstorm` STR file instead (see
        // `str_aliases.rs`) and leave SPR routing out for this id, since
        // there is no classic sprite to point at.

        // Monster effects M01..M13. Each is one billboard held until the
        // duration. Anim speed 4, plays once except M04. M02 is *not*
        // here — it's directional (see `effects/m_ef02.rs`).
        // M01: plain alpha blend (not the default per-pixel tint), alpha
        // 220, anim speed 3. There is no separate blend field on `SprDef`,
        // so the alpha folds into the tint to approximate the blend
        // on screen.
        EffectId::M01 => SprDef::new("data/sprite/이팩트/m_ef01")
            .with_anim_speed(3.0)
            .one_shot()
            .with_tint([1.0, 1.0, 1.0, 220.0 / 255.0]),
        EffectId::M03 => SprDef::new("data/sprite/이팩트/m_ef03").one_shot(),
        // M04: the one looping member — Somatology-lab mob aura, repeats over
        // its persistent duration (default `repeat = true` is correct).
        EffectId::M04 => SprDef::new("data/sprite/이팩트/m_ef04"),
        EffectId::M05 => SprDef::new("data/sprite/이팩트/m_ef05").one_shot(),
        EffectId::M06 => SprDef::new("data/sprite/이팩트/m_ef06").one_shot(),
        EffectId::M07 => SprDef::new("data/sprite/이팩트/m_ef07").one_shot(),
        _ => return None,
    })
}
