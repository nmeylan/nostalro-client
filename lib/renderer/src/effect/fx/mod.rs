//! Concrete custom-effect implementations grouped by primitive family.
//!
//! Each module here implements [`super::CustomEffect`] for one family (or
//! bespoke effect). The dispatch from `EffectId` → constructor goes through
//! [`super::custom_effect::make_custom`].

pub mod air_swirl;
pub mod aura;
pub mod cast_circle;
pub mod cross_beam;
pub mod cylinder_pillar;
pub mod flat_quad;
pub mod floating_spirit;
pub mod ground_ring;
pub mod heal_burst;
pub mod melee_impact;
pub mod radial_burst;
pub mod screen_flash;
pub mod spike_row;
pub mod spline_projectile;
pub mod status_orb;
pub mod waterfall;
pub mod wall;
