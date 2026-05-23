//! `EF_BEGINSPELL6` (id 106) — white cast aura.
//!
//! Two saint-casting passes of white rings (`ring_white.tga`) at radii 45 and
//! 25, with no `F1` argument. The size table uses
//! `max_height = (F1==2) ? 25-ec : (F1==22)
//! ? 15-ec : 20-ec` — both BeginSpell (F1=1) and BeginSpell6 (F1=0) fall
//! through to the default `20-ec` branch. So this effect is geometrically
//! identical to BeginSpell, differing only in texture colour.

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURE: &str = "ring_white.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// `SAINTCASTING` default-F1 size table: `max_height = 20 - ec`. Identical
/// to BeginSpell (F1=1) — colour is the only difference.
const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: TEXTURE,
    max_heights: [20.0, 19.0, 18.0, 17.0],
};

pub struct BeginSpell6Effect(SaintCastingEffect);

impl BeginSpell6Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
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
    use crate::effect::draw::EffectPrimitiveDraw;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn emits_eight_white_frustums_on_frame_zero() {
        let e = BeginSpell6Effect::new([0.0; 3]);
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
