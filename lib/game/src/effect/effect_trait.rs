//! Behavior contract for custom effects.
//!
//! Each effect under `effects/` implements [`Effect`]. The renderer crate
//! drives them through `EffectHolder`, calling `update` each frame and
//! `collect_draws` to gather the [`super::draw::EffectPrimitiveDraw`] entries
//! to render.

use super::draw::{EffectDrawList, EffectStatus};

/// Minimal renderer-agnostic camera snapshot. Effects that need orientation
/// (billboards, screen-space flashes) read this; full wgpu `Camera` stays in
/// the renderer.
#[derive(Clone, Copy, Debug, Default)]
pub struct CameraView {
    pub eye: [f32; 3],
    pub target: [f32; 3],
    pub up: [f32; 3],
}

pub struct EffectUpdateCtx {
    pub dt: f32,
}

pub struct EffectRenderCtx {
    pub camera: CameraView,
    pub screen_w: f32,
    pub screen_h: f32,
    pub elapsed: f32,
}

pub trait Effect: Send {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus;
    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx);
}
