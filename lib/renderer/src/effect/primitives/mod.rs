//! Shared GPU primitives used by custom effects. Each primitive converts a
//! variant of [`super::EffectPrimitiveDraw`] into renderer batches.

pub mod billboard;
pub mod frustum;
pub mod ground_disc;
pub mod quad_horn;

pub use billboard::build_billboard_batches;
pub use frustum::FrustumRenderer;
pub use ground_disc::GroundDiscRenderer;
pub use quad_horn::QuadHornRenderer;
