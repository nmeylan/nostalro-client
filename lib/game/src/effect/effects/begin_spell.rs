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

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];
pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// SAINTCASTING F1=1 size table: `max_height = 20 - ec`.
const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: TEXTURE,
    max_heights: [20.0, 19.0, 18.0, 17.0],
};

pub struct BeginSpellEffect(SaintCastingEffect);

impl BeginSpellEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self(SaintCastingEffect::new(world_pos, CONFIG))
    }
}

impl Effect for BeginSpellEffect {
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
    fn emits_eight_yellow_frustums_on_frame_zero() {
        let e = BeginSpellEffect::new([0.0; 3]);
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
