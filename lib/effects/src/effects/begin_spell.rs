//! `EF_BEGINSPELL` (id 12) — yellow cast aura.
//!
//! Two casting passes of yellow rings (`ring_yellow.tga`) at radii 45
//! and 25, with an `EF_BeginSpell.wav` cue, mirroring the in-game cast:
//! ```text
//! play sound effect\\EF_BeginSpell.wav
//! casting cone, radius 45, ring_yellow.tga
//! casting cone, radius 25, ring_yellow.tga
//! ```
//! The larger size table descends per
//! emitter: `max_height ∈ {20, 19, 18, 17}`, descending
//! alongside an alpha staircase. All other geometry
//! (4 emitters at 90°, `distance = 4.1`, `rise_angle = 80°`, time
//! deltas, bell-shaped per-segment flame envelope) is shared with the rest
//! of the casting-aura family and lives in [`super::saint_casting`].

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// Saint-casting larger size table: `max_height = 20 - emitter_index`.
/// Warm-yellow (255,255,170) tint, additive.
const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: TEXTURE,
    pass_textures: None,
    max_heights: [20.0, 19.0, 18.0, 17.0],
    color_rgb: [1.0, 1.0, 170.0 / 255.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

pub struct BeginSpellEffect(SaintCastingEffect);

impl BeginSpellEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
    }

    /// Stretch the cast aura to the skill's cast time (`None` keeps the
    /// authored default). See [`SaintCastingEffect::with_life_ms`].
    pub fn with_life_ms(self, ms: Option<u32>) -> Self {
        Self(self.0.with_life_ms(ms))
    }
}

impl Effect for BeginSpellEffect {
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
    fn emits_eight_yellow_frustums_once_the_cascade_is_up() {
        // The cones fade in on a staggered schedule (`process = -ec·5`); by
        // ~frame 18 the last emitter has started and all 8 are visible.
        let mut e = BeginSpellEffect::new([0.0; 3]);
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
            "every cone uses ring_yellow.tga"
        );
    }
}
