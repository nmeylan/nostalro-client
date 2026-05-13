pub mod blend;
pub mod custom_effect;
pub mod fx;
pub mod holder;
pub mod primitives;
pub mod str_pipeline;

pub use blend::{BlendKind, d3d_blend_to_wgpu};
pub use custom_effect::{
    CustomEffect, CustomParams, EffectRenderCtx, EffectStatus, EffectUpdateCtx, make_custom,
};
pub use holder::{EffectHandle, EffectHolder, SpawnOutcome, SpawnStatus};
pub use primitives::build_billboard_batches;
pub use str_pipeline::{
    StrEffectCache, StrEffectEntry, StrEmitterInput, build_str_effect_batches,
};

/// One renderable primitive emitted by an effect. Effects don't depend on
/// wgpu types directly - they describe what they want drawn, and the effect
/// render pass turns each variant into pipeline calls.
#[derive(Clone, Debug)]
pub enum EffectPrimitiveDraw {
    /// Camera-facing textured quad.
    Billboard {
        pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Flat ground ring/circle (Land Protector, Pneuma, Sanctuary base).
    Ring {
        center: [f32; 3],
        radius: f32,
        thickness: f32,
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Vertical cylinder of light (Magnus, Sanctuary).
    Cylinder {
        base: [f32; 3],
        height: f32,
        radius: f32,
        segments: u32,
        uv_scroll: [f32; 2],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Rotating textured ring around an actor (Lv99 aura).
    AuraQuad {
        center: [f32; 3],
        radius: f32,
        rotation: f32,
        vertical_offset: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Connected line strip (Grand Cross beams, Spear Boomerang trail).
    LineStrip {
        points: Vec<[f32; 3]>,
        uv_along: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Bezier/Catmull-Rom curve, CPU-tessellated into a line strip
    /// (Soul Strike, Napalm Beat).
    Spline {
        control_points: Vec<[f32; 3]>,
        segments: u32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
}

/// Collected primitive draws for a single frame. Effects push into this;
/// the effect render pass drains it.
#[derive(Default)]
pub struct EffectDrawList {
    pub primitives: Vec<EffectPrimitiveDraw>,
}

impl EffectDrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, prim: EffectPrimitiveDraw) {
        self.primitives.push(prim);
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}
