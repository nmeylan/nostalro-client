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

/// Transient tint applied to the master sprite while an effect is active.
/// Matches the original game's body recolour while a buff is up
/// (PortalWind, GumGang, etc.). RGB only — alpha is always opaque
/// (the body's own opacity is left untouched).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyTint {
    pub rgb: [u8; 3],
}

/// One-shot screen-shake request — the original game's screen quake. An effect
/// returns this once (then `None`); a camera-shake controller owns the
/// per-frame decay and offsets the view, so the whole scene trembles. The
/// effect does not track the shake itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraShake {
    /// Peak displacement in world units, decaying linearly to zero.
    pub amplitude: f32,
    pub duration_ms: u32,
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

    /// Per-frame body tint to apply to the master sprite. Returns `Some`
    /// only during the effect's tint window (e.g. PortalWind's
    /// frames 5..=25). Default `None` — most effects do not tint.
    /// The renderer's actor pass is responsible for reading this and
    /// composing it with the sprite's base colour.
    fn body_tint(&self) -> Option<BodyTint> {
        None
    }

    /// One-shot SFX request — the effect returns the wave path the *first*
    /// time it's ready, then `None` on every subsequent call. The
    /// holder/audio bridge drains this once per frame and queues the sound.
    /// Path uses the backslash-separated form (e.g.
    /// `"effect\\windwalk.wav"`) so the lookup matches GRF / file-system
    /// naming. Default `None` — most effects don't trigger SFX.
    fn take_sfx_request(&mut self) -> Option<&'static str> {
        None
    }

    /// One-shot screen-shake request (screen quake) — returned the *first* time
    /// it's ready, then `None`. The holder drains it once per frame and feeds
    /// a camera-shake controller. Default `None` — most effects don't shake
    /// the screen.
    fn take_camera_shake(&mut self) -> Option<CameraShake> {
        None
    }

    /// Per-frame **body shake** — a screen-space pixel offset to jitter the
    /// attached actor's sprite (the original game's body-quake light),
    /// distinct from the whole-screen [`take_camera_shake`]. Returns `Some`
    /// only during the effect's shake window. The client's actor pass adds
    /// this to the entity's screen anchor. Default `None`.
    fn body_shake(&self) -> Option<[f32; 2]> {
        None
    }
}
