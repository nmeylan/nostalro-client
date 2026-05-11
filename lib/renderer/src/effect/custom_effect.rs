//! Custom (non-STR) effect trait + family registry.
//!
//! Each variant of [`ragnarok_game::effect::CustomFamily`] maps to one Rust
//! module under `lib/renderer/src/effect/fx/`. [`make_custom`] is the single
//! dispatch point; new families add a match arm here.

use ragnarok_game::effect::CustomFamily;

use super::EffectDrawList;
use crate::camera::Camera;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStatus {
    Running,
    Dead,
}

pub struct EffectUpdateCtx {
    pub dt: f32,
}

pub struct EffectRenderCtx<'a> {
    pub camera: &'a Camera,
    pub screen_w: f32,
    pub screen_h: f32,
    pub elapsed: f32,
}

/// What every custom-effect implementation provides. The game-side
/// [`ragnarok_game::effect::ActiveEffect`] trait wraps a `Box<dyn CustomEffect>`
/// when bridging into the holder.
pub trait CustomEffect: Send {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus;
    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx);
}

/// Caller-provided parameters for a custom effect instance. Today this is a
/// single shared shape; if a family needs a larger / richer parameter set we
/// extend this struct rather than introducing a per-family variant — most
/// fields are optional.
#[derive(Clone, Debug, Default)]
pub struct CustomParams {
    /// World-space spawn position (most effects).
    pub world_pos: [f32; 3],
    /// Optional target position for line-shaped effects (Grimtooth, Grand Cross).
    pub target_pos: Option<[f32; 3]>,
    /// Override of the default texture path for the family.
    pub texture: Option<&'static str>,
    /// Tint applied on top of the family's default color.
    pub tint: Option<[f32; 4]>,
}

/// Build a concrete custom-effect instance for the requested family. Returns
/// `None` for families that don't have a Rust implementation yet — callers
/// (the holder spawn path) should log and skip in that case.
pub fn make_custom(
    family: CustomFamily,
    _params: &CustomParams,
) -> Option<Box<dyn CustomEffect>> {
    match family {
        CustomFamily::Aura
        | CustomFamily::GroundRing
        | CustomFamily::CastCircle
        | CustomFamily::SpikeRow
        | CustomFamily::Wall
        | CustomFamily::CylinderPillar
        | CustomFamily::CrossBeam
        | CustomFamily::SplineProjectile
        | CustomFamily::RadialBurst
        | CustomFamily::ScreenFlash
        | CustomFamily::Bespoke(_) => None,
    }
}
