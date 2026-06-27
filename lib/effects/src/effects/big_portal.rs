//! `EF_BIGPORTAL` / `EF_BIGPORTAL2` (ids 561/562) — warp/recall portal composite.

use super::heal::{self, HealEffect};
use super::portal_wind::{self, PortalWindEffect};
use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["Magic_Violet.tga", "cloud11.tga"];

pub const TOTAL_DURATION_MS: u32 = 20000;
pub const TOTAL_DURATION_MS_PERSISTENT: u32 = 99990;

pub struct BigPortalEffect {
    rings: HealEffect,
    halo: PortalWindEffect,
}

impl BigPortalEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            rings: HealEffect::new(world_pos, &heal::BIGPORTAL),
            halo: PortalWindEffect::new(world_pos, portal_wind::BIGPORTAL_WIND),
        }
    }

    pub fn new_persistent(world_pos: [f32; 3]) -> Self {
        Self {
            rings: HealEffect::new(world_pos, &heal::BIGPORTAL2),
            halo: PortalWindEffect::new(world_pos, portal_wind::BIGPORTAL_WIND2),
        }
    }
}

impl Effect for BigPortalEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let status = self.rings.update(ctx);
        let _ = self.halo.update(ctx);
        status
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.halo.collect_draws(out, ctx);
        self.rings.collect_draws(out, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::EffectPrimitiveDraw;

    const FPS: f32 = 60.0;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut BigPortalEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &BigPortalEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_violet_rings_and_four_halo_cones() {
        let mut e = BigPortalEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 15.0);
        let prims = draws(&e);
        let rings = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::RadialRing { texture, .. } if *texture == "Magic_Violet.tga"))
            .count();
        let cones = prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { texture, .. } if *texture == "cloud11.tga"))
            .count();
        assert_eq!(rings, 3);
        assert_eq!(cones, 4);
    }

    #[test]
    fn persistent_variant_outlives_the_finite_one() {
        let mut finite = BigPortalEffect::new([0.0; 3]);
        let mut persistent = BigPortalEffect::new_persistent([0.0; 3]);
        let s_finite = step(&mut finite, 1300.0);
        let s_persistent = step(&mut persistent, 1300.0);
        assert!(matches!(s_finite, EffectStatus::Dead));
        assert!(matches!(s_persistent, EffectStatus::Running));
    }
}
