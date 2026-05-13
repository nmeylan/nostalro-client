//! Shared GPU primitives used by custom effects. Each primitive converts a
//! variant of [`super::EffectPrimitiveDraw`] into renderer batches.
//!
//! Today everything routes through the existing `SpriteRenderer` via
//! `SpriteBatch`. As we add primitives that need their own pipeline (custom
//! WGSL shaders, instanced strips, etc.) they'll grow their own batch type.

pub mod billboard;
pub mod ring;

pub use billboard::build_billboard_batches;
pub use ring::RingRenderer;
