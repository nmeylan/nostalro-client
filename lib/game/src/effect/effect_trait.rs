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

#[derive(Default)]
pub struct EffectUpdateCtx {
    pub delta: f32,
    /// Current camera target in world coordinates, when available. Used by
    /// effects whose spawn point should follow the active view (snow, rain,
    /// other camera-anchored ambient burst emitters). `None` for callers
    /// that don't track a camera (most tests).
    pub camera_target: Option<[f32; 3]>,
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

    /// STR animation that plays alongside this effect's primitives. Holder
    /// emits a `StrSnapshot` for non-`None` returns each frame, attached to
    /// the same world position. Default `None` — pure-primitive effects.
    fn str_overlay(&self) -> Option<&'static str> {
        None
    }
}
