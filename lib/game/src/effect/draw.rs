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
        /// When `true`, the renderer skips quads whose outward-facing normal
        /// points away from the camera — back-face culling
        /// for cones that should only show their outer surface
        /// (e.g. the flat-flaring cast aura). Default `false`
        /// preserves the existing behaviour where both faces of the cone
        /// are visible (BottomSanc pillar, cast-circle petals).
        cull_back: bool,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Square-based pyramid spike — four outward-facing triangles meeting at
    /// a single apex above the centre of a `2*size` × `2*size` base. Matches
    /// the original game's quad-horn spike.
    /// Used by Stormgust ice shards plus eight other effects in the
    /// original game.
    ///
    /// Local frame: base square on the
    /// local XY plane, apex at `(0, 0, height)` along local +Z. Rotations
    /// use row-vector matrix conventions, so callers can pass the original
    /// game's pitch / yaw values directly:
    /// * `tilt_x_deg`  — pitch. 0° = horizontal along
    ///   world +Z, 90° = straight UP (native RO), 100° = nearly vertical
    ///   with slight backward lean (Stormgust), 270° = straight DOWN.
    /// * `rotation_y_deg` — yaw. Rotates the tilted
    ///   spike around the world up-axis to face a compass heading.
    ///
    /// `base` is the bottom-centre point in world coords; the unrotated
    /// spike's apex sits at `base + (0, 0, height)`. This primitive uses
    /// native RO `-Y = up`, so no Y flip is applied at world
    /// output — `base[1]` lives directly in world Y.
    ///
    /// UV layout: each of the four triangles is a vertical strip
    /// on the texture; base vertices at `v = 1`, apex at `v = 0`; `u`
    /// advances by 0.2 per triangle starting at 0 (so the four strips
    /// cover `[0, 0.8]`, leaving `[0.8, 1.0]` unused).
    QuadHorn {
        base: [f32; 3],
        size: f32,
        height: f32,
        tilt_x_deg: f32,
        rotation_y_deg: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// UV sphere centered at `center`. A closed mesh with latitude sweeping
    /// `-90°..+90°` and
    /// longitude sweeping `0°..360°`, two triangles per `(lat × lon)` cell.
    ///
    /// `sides_lat` / `sides_lon` are segment counts (the original game's
    /// default `36°` arc gives `sides_lat = 5`, `sides_lon = 10`).
    /// `longitude_offset` is added to every vertex's longitude angle in
    /// radians — driving it over time slides the texture pattern around
    /// the sphere, as in the original game.
    ///
    /// `uv_repeat` tiles the texture: `[u_repeat, v_repeat]`. v runs from 0
    /// at the south pole to `v_repeat` at the north pole, u from 0 to
    /// `u_repeat` around the equator.
    ///
    /// The lower hemisphere can sit below the impact ground plane — depth
    /// testing against the ground geometry hides whatever portion is below.
    Sphere {
        center: [f32; 3],
        radius: f32,
        sides_lat: u32,
        sides_lon: u32,
        longitude_offset: f32,
        uv_repeat: [f32; 2],
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
