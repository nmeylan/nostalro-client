//! Shared GPU primitives used by custom effects. Each primitive converts a
//! variant of [`super::EffectPrimitiveDraw`] into renderer batches.

pub mod billboard;
pub mod cylinder;
pub mod frustum;
pub mod ground_disc;
pub mod quad_horn;
pub mod radial_ring;
pub mod sphere;
pub mod texture3d;
pub mod world_quad;

pub use billboard::prepare_billboard_records;
pub use cylinder::{CylinderRenderer, prepare_cylinder_records};
pub use frustum::{FrustumRenderer, prepare_frustum_records};
pub use ground_disc::{GroundDiscRenderer, prepare_ground_disc_records};
pub use quad_horn::{QuadHornRenderer, prepare_quad_horn_records};
pub use radial_ring::{RadialRingRenderer, prepare_radial_ring_records};
pub use sphere::{SphereRenderer, prepare_sphere_records};
pub use texture3d::{Texture3DRenderer, prepare_texture3d_records};
pub use world_quad::{WorldQuadRenderer, prepare_world_quad_records};
