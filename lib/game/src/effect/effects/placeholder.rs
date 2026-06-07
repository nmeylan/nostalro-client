//! Placeholder effect for `EffectSpec::Custom` ids that don't yet have a
//! real Rust implementation. Renders a small hot-pink billboard at the
//! attach point so the spawn is visible in the effect viewer.
//!
//! Two flavors:
//!   * [`PlaceholderEffect`] — pure-custom (407 effects in the original game
//!     classification): pink square only.
//!   * [`HybridPlaceholderEffect`] — StrHybrid (12 effects): pink square
//!     *and* declares an `str_overlay()` so the holder plays the original
//!     STR file alongside the marker. Mirrors the Stormgust pattern.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// Soft-edged dot reused as the placeholder sprite. Already preloaded by
/// `cast_circle` so no extra texture entry is needed.
pub const PLACEHOLDER_TEXTURE: &str = "alpha_down.tga";

/// Half-extent of the billboard quad in world units. Big enough to be
/// obvious in the viewer; small enough not to dominate the scene.
const PLACEHOLDER_HALF_SIZE: f32 = 3.0;

/// Hot pink with full alpha — unmistakable as a debug marker.
const PLACEHOLDER_COLOR: [f32; 4] = [1.0, 0.2, 0.8, 1.0];

pub const TEXTURES: &[&str] = &[PLACEHOLDER_TEXTURE];

pub struct PlaceholderEffect {
    origin: [f32; 3],
}

impl PlaceholderEffect {
    pub fn new(origin: [f32; 3]) -> Self {
        Self { origin }
    }
}

impl Effect for PlaceholderEffect {
    fn update(&mut self, _ctx: &EffectUpdateCtx) -> EffectStatus {
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        out.push(EffectPrimitiveDraw::Billboard {
            pos: self.origin,
            size: [PLACEHOLDER_HALF_SIZE * 2.0, PLACEHOLDER_HALF_SIZE * 2.0],
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            rotation: 0.0,
            texture: PLACEHOLDER_TEXTURE,
            color: PLACEHOLDER_COLOR,
            blend: BlendKind::Alpha,
        });
    }
}

/// Placeholder that also drives the effect's original STR overlay. The
/// holder plays `str_overlay()` automatically; this struct just adds the
/// pink-square primitive on top to flag that the custom-primitive layer
/// isn't implemented yet.
pub struct HybridPlaceholderEffect {
    inner: PlaceholderEffect,
    str_file: &'static str,
}

impl HybridPlaceholderEffect {
    pub fn new(world_pos: [f32; 3], str_file: &'static str) -> Self {
        Self {
            inner: PlaceholderEffect::new(world_pos),
            str_file,
        }
    }
}

impl Effect for HybridPlaceholderEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.inner.update(ctx)
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        self.inner.collect_draws(out, ctx);
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(self.str_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn placeholder_emits_one_billboard() {
        let mut e = PlaceholderEffect::new([10.0, 0.0, 20.0]);
        assert_eq!(e.update(&EffectUpdateCtx { delta: 0.016, camera_target: None, caster_yaw: None }), EffectStatus::Running);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
        assert!(e.str_overlay().is_none());
    }

    #[test]
    fn hybrid_placeholder_renders_pink_and_declares_str_overlay() {
        let e = HybridPlaceholderEffect::new(
            [0.0; 3],
            "stormgust",
        );
        assert_eq!(e.str_overlay(), Some("stormgust"));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
    }
}
