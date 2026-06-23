//! `BlueCasting` / `DarkCasting` — begin-spell casting rings that differ from
//! the yellow `BeginSpell` only by ring texture and colour.
//!
//! Each fires two casting passes (one at frame 45, one at frame 25):
//!
//!   BlueCasting:  blue ring,  flag 5
//!   DarkCasting:  black ring, flag 6
//!
//! Both flags fall through the casting aura's default size table
//! (max-height ∈ {20, 19, 18, 17} per emitter — only the asura/aura-blade
//! variants differ), so the geometry is identical to the yellow `BeginSpell`.
//! The whole 4-emitter cone seed + per-frame bell envelope lives in [`super::saint_casting`].
//!
//! Colour and blend come from a shared per-size vertex-tint table:
//! flag 5 → size 4 → (100,100,255) **additive** blue glow; flag 6 → size 12 →
//! (50,50,50) **alpha-blended** — a dark dome that genuinely darkens what's
//! behind it (additive dark would be invisible).

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{
    SaintCastingConfig, SaintCastingEffect, TOTAL_DURATION_MS as SAINT_TOTAL_DURATION_MS,
};

pub const TOTAL_DURATION_MS: u32 = SAINT_TOTAL_DURATION_MS;

/// Shared default size table — every casting pass except the
/// asura and aura-blade variants uses it.
const DEFAULT_HEIGHTS: [f32; 4] = [20.0, 19.0, 18.0, 17.0];

/// `EF_BLUECASTING` — blue casting ring (flag 5).
pub const BLUE: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_blue.tga",
    pass_textures: None,
    max_heights: DEFAULT_HEIGHTS,
    color_rgb: [100.0 / 255.0, 100.0 / 255.0, 1.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

/// `EF_DARKCASTING` — black casting ring (flag 6).
pub const DARK: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_black.tga",
    pass_textures: None,
    max_heights: DEFAULT_HEIGHTS,
    color_rgb: [50.0 / 255.0, 50.0 / 255.0, 50.0 / 255.0],
    blend: BlendKind::Alpha,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

pub const TEXTURES: &[&str] = &["ring_blue.tga", "ring_black.tga"];

pub struct ColorCastingEffect(SaintCastingEffect);

impl ColorCastingEffect {
    pub fn new(world_pos: [f32; 3], cfg: SaintCastingConfig) -> Self {
        Self(SaintCastingEffect::new(world_pos, cfg))
    }

    /// Stretch the cast aura to the skill's cast time (`None` keeps the
    /// authored default). See [`SaintCastingEffect::with_life_ms`].
    pub fn with_life_ms(self, ms: Option<u32>) -> Self {
        Self(self.0.with_life_ms(ms))
    }
}

impl Effect for ColorCastingEffect {
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

    fn frustums(cfg: SaintCastingConfig) -> Vec<(&'static str, [f32; 4], BlendKind)> {
        let mut e = ColorCastingEffect::new([0.0; 3], cfg);
        // Cones fade in on a staggered schedule; step until all 8 are up.
        for _ in 0..18 {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 800.0,
                screen_h: 600.0,
                elapsed: 0.0,
            },
        );
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum {
                    texture,
                    color,
                    blend,
                    ..
                } => Some((*texture, *color, *blend)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn blue_and_dark_emit_eight_tinted_cones_of_their_own_ring_texture() {
        // Sociable: through SaintCastingEffect — blue is an additive blue
        // glow, dark is an alpha-blended dark-gray dome.
        let blue = frustums(BLUE);
        assert_eq!(blue.len(), 8, "two SAINTCASTING passes × 4 emitters");
        for (tex, color, blend) in &blue {
            assert_eq!(*tex, "ring_blue.tga");
            assert!(color[2] > color[0] && color[2] > 0.9, "blue-dominant tint");
            assert_eq!(*blend, BlendKind::Additive);
        }

        let dark = frustums(DARK);
        assert_eq!(dark.len(), 8);
        for (tex, color, blend) in &dark {
            assert_eq!(*tex, "ring_black.tga");
            assert!(
                color[0] < 0.3 && color[1] < 0.3 && color[2] < 0.3,
                "dark-gray tint, got {color:?}"
            );
            assert_eq!(*blend, BlendKind::Alpha);
        }
    }
}
