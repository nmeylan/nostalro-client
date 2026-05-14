//! Per-effect implementations of [`super::effect_trait::Effect`].
//!
//! Each effect lives in its own module (or shares a module with sibling
//! variants that reuse the same struct via parameter sets). The factory
//! ([`super::factory::make_effect`]) is the single dispatch point.

pub mod aura;
pub mod bottom_sanctuary_pillar;
pub mod cast_circle;
pub mod magnum_break;
pub mod placeholder;
pub mod stormgust;
pub mod volcano;
pub mod warp;
pub mod warp_zone;
