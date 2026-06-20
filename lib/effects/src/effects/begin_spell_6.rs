//! `EF_BEGINSPELL6` (id 106) — white cast aura.
//!
//! Two saint-casting passes of white rings (`ring_white.tga`) at radii 45 and
//! 25, with no `F1` argument. The size table uses
//! `max_height = (F1==2) ? 25-ec : (F1==22)
//! ? 15-ec : 20-ec` — both BeginSpell (F1=1) and BeginSpell6 (F1=0) fall
//! through to the default `20-ec` branch. So this effect is geometrically
//! identical to BeginSpell, differing only in texture colour.

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURE: &str = "ring_white.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// Saint-casting default-F1 size table: `max_height = 20 - ec`. Identical
/// to BeginSpell (F1=1) — colour is the only difference. Untinted white,
/// additive.
const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: TEXTURE,
    pass_textures: None,
    max_heights: [20.0, 19.0, 18.0, 17.0],
    color_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

pub struct BeginSpell6Effect(SaintCastingEffect);

impl BeginSpell6Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
    }

    /// Stretch the cast aura to the skill's cast time (`None` keeps the
    /// authored default). See [`SaintCastingEffect::with_life_ms`].
    pub fn with_life_ms(self, ms: Option<u32>) -> Self {
        Self(self.0.with_life_ms(ms))
    }
}

impl Effect for BeginSpell6Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.0.update(ctx)
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.0.collect_draws(out, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::EffectPrimitiveDraw;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn emits_eight_white_frustums_once_the_cascade_is_up() {
        // The cones fade in on a staggered schedule (`process = -ec·5`); by
        // ~frame 18 the last emitter has started and all 8 are visible.
        let mut e = BeginSpell6Effect::new([0.0; 3]);
        for _ in 0..18 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let frustums: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert_eq!(frustums.len(), 8, "two SAINTCASTING passes × 4 emitters");
        assert!(
            frustums.iter().all(|t| *t == TEXTURE),
            "every cone uses ring_white.tga"
        );
    }
}
