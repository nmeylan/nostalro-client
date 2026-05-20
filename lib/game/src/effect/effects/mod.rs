//! Per-effect implementations of [`super::effect_trait::Effect`].
//!
//! Each effect lives in its own module (or shares a module with sibling
//! variants that reuse the same struct via parameter sets). The factory
//! ([`super::factory::make_effect`]) is the single dispatch point.

pub mod aura;
pub mod begin_spell_6;
pub mod bottom_hermode;
pub mod bottom_landprotector;
pub mod bottom_light;
pub mod bottom_magnus;
pub mod bottom_out;
pub mod bottom_sanctuary_pillar;
pub mod bottom_song;
pub mod bottom_vertical;
pub mod cast_circle;
pub mod hit;
pub mod hit2;
pub mod hit5_6;
pub mod magnum_break;
pub mod placeholder;
pub mod stormgust;
pub mod animated_texture_billboard;
pub mod volcano;
pub mod warp;
pub mod warp_zone;
