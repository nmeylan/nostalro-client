//! Renderer-agnostic effect draw primitives.
//!
//! Effect implementations (in `effects/`) emit `EffectPrimitiveDraw` entries
//! into an `EffectDrawList`; the renderer crate consumes the list each frame
//! and dispatches them to dedicated GPU pipelines.

/// Pre-classified blend mode for effect primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendKind {
    /// `src.rgb * src.a + dst.rgb * (1 - src.a)`
    Alpha,
    /// `src.rgb * src.a + dst.rgb` - used by most STR layers, auras, sparks.
    Additive,
    /// `src.rgb * dst.rgb` - darkening / shadow overlays.
    Multiply,
    /// Raw D3D source/dest factor pair from an STR frame.
    Raw { src: i32, dst: i32 },
}

/// Lifecycle signal returned by `Effect::update`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStatus {
    Running,
    Dead,
}

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
    /// Flat-on-ground textured annulus or partial-arc wedge.
    ///
    /// `arc_angle_deg` is the **total arc span** in degrees (`>= 360` = full
    /// ring; smaller clips to a wedge starting at `+X` and sweeping CCW).
    /// `uv_repeat` is how many times the texture tiles around the ring along
    /// `u`; `v` runs 0 at the outer edge to 1 at the inner edge (matching the
    /// original game's flat-ring convention). `rotation` is a u-offset in
    /// radians, useful for spinning textures.
    GroundDisc {
        center: [f32; 3],
        radius: f32,
        thickness: f32,
        rotation: f32,
        arc_angle_deg: f32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Vertical "tube" between two coaxial polygons.
    ///
    /// `base` is the bottom-ring centre; the top ring sits `height` units
    /// above it (native RO coords, so `-Y` is up — the renderer applies the
    /// sign). `bottom_size`/`top_size` are the bottom/top radii (equal →
    /// cylinder; `top_size == 0` → cone). `sides` controls the segment count
    /// (4 = square pillar, 16-24 ≈ smooth circle). `rotation` is the angle
    /// in radians around the vertical axis the geometry starts at (also
    /// shifts the u-coordinate so the texture follows the geometry).
    /// `uv_scroll` is an additive `[u, v]` scroll, `uv_repeat` how many
    /// times the texture tiles around the circumference.
    ///
    /// Each top-ring vertex's height is offset by
    /// `wave_amplitude * sin(angle * wave_frequency + wave_phase)` —
    /// driving `wave_phase` over time makes the height wave travel around
    /// the ring (used by LandProtector to make flame peaks rotate around
    /// the curtain). `wave_amplitude == 0` produces a flat top ring.
    Frustum {
        base: [f32; 3],
        bottom_size: f32,
        top_size: f32,
        height: f32,
        sides: u32,
        rotation: f32,
        uv_repeat: f32,
        uv_scroll: [f32; 2],
        wave_amplitude: f32,
        wave_frequency: f32,
        wave_phase: f32,
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
