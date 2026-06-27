//! `EF_AURABLADE` (id 367) — Aura Blade cast aura.

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURES: &[&str] = &["ring_white.tga", "ring_yellow.tga"];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_white.tga",
    pass_textures: Some(["ring_white.tga", "ring_yellow.tga"]),
    max_heights: [15.0, 14.0, 13.0, 12.0],
    color_rgb: [1.0, 1.0, 1.0],
    blend: BlendKind::Additive,
    refill_per_frame: 5.0,
    reset_rise_deg: 64.0,
};

pub struct AuraBladeEffect(SaintCastingEffect);

impl AuraBladeEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
    }
}

impl Effect for AuraBladeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.0.update(ctx)
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.0.set_position(pos);
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
    fn stacks_white_over_yellow_rings() {
        let mut e = AuraBladeEffect::new([0.0; 3]);
        for _ in 0..18 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let textures: Vec<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { texture, .. } => Some(*texture),
                _ => None,
            })
            .collect();
        assert_eq!(textures.len(), 8, "two SAINTCASTING passes × 4 emitters");
        let white = textures.iter().filter(|t| **t == "ring_white.tga").count();
        let yellow = textures.iter().filter(|t| **t == "ring_yellow.tga").count();
        assert_eq!(white, 4, "pass 0 (time=45) is the white ring");
        assert_eq!(yellow, 4, "pass 1 (time=25) is the yellow ring");
    }
}
