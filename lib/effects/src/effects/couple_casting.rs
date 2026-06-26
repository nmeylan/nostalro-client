//! `EF_COUPLECASTING` (id 342) — red/pink couple-skill cast aura.
//!
//! Two staggered casting passes over the `ring_red.tga` texture, the first
//! rising at 45° and the second at 25° a few frames later. Both reuse the
//! same casting cascade as the yellow `BeginSpell`, recoloured to rose.
//!
//! The rose variant selects the (255, 89, 182) rose tint and the
//! `max_height ∈ {20, 19, 18, 17}` size table — the same descending staircase
//! as the yellow `BeginSpell`. All other geometry (4 emitters at 90°,
//! `distance = 4.1`, `rise_angle = 80°`, bell-shaped flame envelope, two
//! staggered passes) is shared with the `BeginSpell*` family and lives in
//! [`super::saint_casting`].
//!
//! The two passes give the doubled, layered look the reference gif shows.

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURE: &str = "ring_red.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// Rose-tinted casting cascade: tint (255, 89, 182), additive,
/// `max_height = 20 - ec`.
const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: TEXTURE,
    pass_textures: None,
    max_heights: [20.0, 19.0, 18.0, 17.0],
    color_rgb: [255.0 / 255.0, 89.0 / 255.0, 182.0 / 255.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

pub struct CoupleCastingEffect(SaintCastingEffect);

impl CoupleCastingEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
    }
}

impl Effect for CoupleCastingEffect {
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
    fn emits_eight_rose_red_frustums_once_the_cascade_is_up() {
        let mut e = CoupleCastingEffect::new([0.0; 3]);
        for _ in 0..18 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let cones: Vec<_> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { texture, color, .. } => Some((*texture, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(cones.len(), 8, "two SAINTCASTING passes × 4 emitters");
        for (texture, color) in &cones {
            assert_eq!(*texture, TEXTURE);
            // Rose tint: red dominant, blue above green.
            assert!(
                color[0] > color[1] && color[2] > color[1],
                "rose tint {color:?}"
            );
        }
    }
}
