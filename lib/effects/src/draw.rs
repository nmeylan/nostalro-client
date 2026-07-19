/// Selects how `Frustum` modulates each top-ring vertex's height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrustumWaveMode {
    /// `wave_amplitude * sin(angle * wave_frequency + wave_phase)` around the ring.
    #[default]
    Sine,
    /// Single positive lobe centred opposite the seam. `wave_amplitude` may go
    /// negative to flip the lobe inward.
    SaintBell,
}

/// Pre-classified blend mode for effect primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendKind {
    /// `src.rgb * src.a + dst.rgb * (1 - src.a)`
    Alpha,
    /// `src.rgb * src.a + dst.rgb`
    Additive,
    /// `src.rgb * dst.rgb`
    Multiply,
    Raw {
        src: i32,
        dst: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectStatus {
    Running,
    Dead,
}

pub fn aim_backward(position: [f32; 3], target: [f32; 3]) -> [f32; 3] {
    [
        2.0 * position[0] - target[0],
        2.0 * position[1] - target[1],
        2.0 * position[2] - target[2],
    ]
}

#[derive(Clone, Debug)]
pub enum EffectPrimitiveDraw {
    /// Camera-facing textured quad. `rotation` rotates in screen space (CCW radians).
    Billboard {
        pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Like [`Billboard`] but drawn as a near-plane overlay ignoring 3D depth.
    BillboardFlash {
        pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing textured quad whose depth is taken from `depth_pos` instead of `pos`.
    BillboardDepthAnchored {
        pos: [f32; 3],
        depth_pos: [f32; 3],
        size: [f32; 2],
        uv: [[f32; 2]; 4],
        rotation: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing filled disc with polar UV mapping (V=1 centre, V=0 rim).
    BillboardDisc {
        pos: [f32; 3],
        radius: f32,
        segments: u32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Camera-facing textured annulus (V=0 outer rim, V=1 inner rim).
    BillboardRing {
        pos: [f32; 3],
        radius: f32,
        thickness: f32,
        segments: u32,
        uv_repeat: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
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
        /// When true, render as an overlay the terrain never depth-occludes.
        no_depth: bool,
        /// Tilt of the disc plane about a horizontal axis: `0` = flat on the
        /// ground, `PI/2` = standing vertical.
        tilt_rad: f32,
        /// Rotation of the (tilted) disc plane about the vertical axis.
        spin_rad: f32,
    },
    Frustum {
        base: [f32; 3],
        bottom_size: f32,
        top_size: f32,
        height: f32,
        sides: u32,
        arc_angle_deg: f32,
        rotation: f32,
        uv_repeat: f32,
        uv_scroll: [f32; 2],
        wave_amplitude: f32,
        wave_frequency: f32,
        wave_phase: f32,
        wave_mode: FrustumWaveMode,
        tilt_x_rad: f32,
        rotation_y_rad: f32,
        cull_back: bool,
        base_alpha: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Square-based pyramid spike — four triangles meeting at an apex above a `2*size × 2*size` base.
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
    Sphere {
        center: [f32; 3],
        radius: f32,
        sides_lat: u32,
        sides_lon: u32,
        longitude_offset: f32,
        longitude_arc: f32,
        uv_repeat: [f32; 2],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
        /// When true, render as an overlay the terrain never depth-occludes.
        no_depth: bool,
    },
    Cylinder {
        base: [f32; 3],
        bottom_size: f32,
        top_size: f32,
        height: f32,
        sides: u32,
        rotation: f32,
        tilt_x_rad: f32,
        rotation_y_rad: f32,
        uv_scroll: [f32; 2],
        texture: &'static str,
        color: [f32; 4],
        alpha_bottom: f32,
        blend: BlendKind,
    },
    RadialRing {
        center: [f32; 3],
        distance: f32,
        rise_angle_rad: f32,
        rot_start_rad: f32,
        full_arc_rad: f32,
        segments: u32,
        height_scale: f32,
        heights: [f32; crate::radial_emitter::RADIAL_EMITTER_DIVISION],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    AuraQuad {
        center: [f32; 3],
        radius: f32,
        rotation: f32,
        vertical_offset: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    LineStrip {
        points: Vec<[f32; 3]>,
        uv_along: f32,
        u_along: bool,
        half_width: f32,
        texture: &'static str,
        color: [f32; 4],
        colors: Option<Vec<[f32; 4]>>,
        blend: BlendKind,
    },
    Spline {
        control_points: Vec<[f32; 3]>,
        segments: u32,
        half_width: f32,
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    SpriteParticle {
        sprite_path: &'static str,
        position: [f32; 3],
        action_index: usize,
        motion_index: usize,
        size_scale: f32,
        color: [f32; 4],
        blend: BlendKind,
        aim_target: Option<[f32; 3]>,
        no_depth: bool,
    },
    WorldQuad {
        corners: [[f32; 3]; 4],
        uv: [[f32; 2]; 4],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
        no_depth: bool,
    },
    Texture3D {
        center: [f32; 3],
        size: [f32; 2],
        plane: QuadPlane,
        uv: [[f32; 2]; 4],
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
    },
    /// Screen-space textured quad in NDC clip space (corners are `[-1,1]`).
    ScreenQuad {
        texture: &'static str,
        color: [f32; 4],
        blend: BlendKind,
        corners: [[f32; 2]; 4],
        uvs: [[f32; 2]; 4],
    },
    /// Screen-space triangle mesh in NDC clip space with per-vertex colour.
    ScreenMesh {
        texture: &'static str,
        blend: BlendKind,
        vertices: Vec<([f32; 2], [f32; 4])>,
        indices: Vec<u32>,
    },
}

/// Orientation selector for [`EffectPrimitiveDraw::Texture3D`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuadPlane {
    Horizontal,
    HorizontalYaw(f32),
    VerticalYaw(f32),
}

impl QuadPlane {
    pub fn corners(self, center: [f32; 3], size: [f32; 2]) -> [[f32; 3]; 4] {
        let [cx, cy, cz] = center;
        let [hx, hy] = size;
        match self {
            QuadPlane::Horizontal => [
                [cx - hx, cy, cz - hy],
                [cx + hx, cy, cz - hy],
                [cx + hx, cy, cz + hy],
                [cx - hx, cy, cz + hy],
            ],
            QuadPlane::HorizontalYaw(yaw) => {
                let (s, c) = yaw.sin_cos();
                let fx = hx * c;
                let fz = hx * s;
                let rx = -hy * s;
                let rz = hy * c;
                [
                    [cx - fx - rx, cy, cz - fz - rz],
                    [cx + fx - rx, cy, cz + fz - rz],
                    [cx + fx + rx, cy, cz + fz + rz],
                    [cx - fx + rx, cy, cz - fz + rz],
                ]
            }
            QuadPlane::VerticalYaw(yaw) => {
                let (s, c) = yaw.sin_cos();
                let dx = hx * c;
                let dz = hx * s;
                let top = cy - hy;
                let bot = cy + hy;
                [
                    [cx - dx, bot, cz - dz],
                    [cx + dx, bot, cz + dz],
                    [cx + dx, top, cz + dz],
                    [cx - dx, top, cz - dz],
                ]
            }
        }
    }
}

#[derive(Default)]
pub struct EffectDrawList {
    pub primitives: Vec<EffectPrimitiveDraw>,
    pub behind: Vec<EffectPrimitiveDraw>,
}

impl EffectDrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, prim: EffectPrimitiveDraw) {
        self.primitives.push(prim);
    }

    pub fn push_behind(&mut self, prim: EffectPrimitiveDraw) {
        self.behind.push(prim);
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
        self.behind.clear();
    }

    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty() && self.behind.is_empty()
    }

    pub fn behind_as_list(&self) -> EffectDrawList {
        EffectDrawList {
            primitives: self.behind.clone(),
            behind: Vec::new(),
        }
    }
}
