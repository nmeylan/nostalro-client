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
/// extend this struct rather than introducing a per-family variant - most
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
/// `None` for families that don't have a Rust implementation yet - callers
/// (the holder spawn path) should log and skip in that case.
pub fn make_custom(
    family: CustomFamily,
    params: &CustomParams,
) -> Option<Box<dyn CustomEffect>> {
    match family {
        CustomFamily::Aura => Some(Box::new(super::fx::aura::Aura::new(params))),
        CustomFamily::GroundRing => {
            Some(Box::new(super::fx::ground_ring::GroundRing::new(params)))
        }
        CustomFamily::CastCircle => {
            Some(Box::new(super::fx::cast_circle::CastCircle::new(params)))
        }
        CustomFamily::SpikeRow => {
            Some(Box::new(super::fx::spike_row::SpikeRow::new(params)))
        }
        CustomFamily::Wall => Some(Box::new(super::fx::wall::Wall::new(params))),
        CustomFamily::CylinderPillar => Some(Box::new(
            super::fx::cylinder_pillar::CylinderPillar::new(params),
        )),
        CustomFamily::CrossBeam => {
            Some(Box::new(super::fx::cross_beam::CrossBeam::new(params)))
        }
        CustomFamily::SplineProjectile => Some(Box::new(
            super::fx::spline_projectile::SplineProjectile::new(params),
        )),
        CustomFamily::RadialBurst => {
            Some(Box::new(super::fx::radial_burst::RadialBurst::new(params)))
        }
        CustomFamily::ScreenFlash => {
            Some(Box::new(super::fx::screen_flash::ScreenFlash::new(params)))
        }
        CustomFamily::FlatQuad => {
            Some(Box::new(super::fx::flat_quad::FlatQuad::new(params)))
        }
        CustomFamily::HealBurst => {
            Some(Box::new(super::fx::heal_burst::HealBurst::new(params)))
        }
        CustomFamily::MeleeImpact => {
            Some(Box::new(super::fx::melee_impact::MeleeImpact::new(params)))
        }
        CustomFamily::AirSwirl => {
            Some(Box::new(super::fx::air_swirl::AirSwirl::new(params)))
        }
        CustomFamily::StatusOrb => {
            Some(Box::new(super::fx::status_orb::StatusOrb::new(params)))
        }
        CustomFamily::FloatingSpirit => Some(Box::new(
            super::fx::floating_spirit::FloatingSpirit::new(params),
        )),
        CustomFamily::Waterfall => {
            Some(Box::new(super::fx::waterfall::Waterfall::new(params)))
        }
        CustomFamily::Bespoke(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ragnarok_game::effect::EffectId;

    #[test]
    fn unknown_bespoke_returns_none() {
        let result = make_custom(CustomFamily::Bespoke(EffectId::Bubble), &CustomParams::default());
        assert!(result.is_none());
    }
}
