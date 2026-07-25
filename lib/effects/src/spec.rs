#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectAnchor {
    Point([f32; 3]),
    Trail { from: [f32; 3], to: [f32; 3] },
}

impl EffectAnchor {
    pub fn point(self) -> [f32; 3] {
        match self {
            EffectAnchor::Point(p) | EffectAnchor::Trail { from: p, .. } => p,
        }
    }

    pub fn trail(self) -> ([f32; 3], [f32; 3]) {
        match self {
            EffectAnchor::Point(p) => (p, p),
            EffectAnchor::Trail { from, to } => (from, to),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Attach {
    Entity(u32),
    WorldPos([f32; 3]),
    Projectile { from: u32, to: u32 },
    Trail { from: [f32; 3], to: [f32; 3] },
    Link { caster: u32, target: u32 },
}

#[derive(Clone, Debug)]
pub enum EffectSpec {
    Str {
        file: &'static str,
        duration_ms: u32,
        repeat: bool,
    },
    Custom,
    Spr {
        sprite: &'static str,
        duration_ms: u32,
        size_scale: f32,
        anim_speed: f32,
        repeat: bool,
        tint: [f32; 4],
        pos_y: f32,
        action_index: usize,
        /// Drawn with no depth test at all, as the original's `Effect_SPR` family
        /// does (`m_renderFlag |= RF_NODEPTHCHECK`).
        no_depth: bool,
    },
    SprBurst {
        sprite: &'static str,
        duration_ms: u32,
        burst: SprBurstParams,
        body_recolor: Option<SprBodyRecolor>,
    },
    Noop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SprBodyRecolor {
    pub window_frames: (u32, u32),
    pub rgb: [u8; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct SprBurstParams {
    pub particle_lifetime_ms: f32,
    pub size: f32,
    pub alpha_max: f32,
    pub burst_count_range: (u32, u32),
    pub speed_range: (f32, f32),
    pub anim_speed: f32,
    pub pos_y_start: f32,
    pub spawn_radius_xz: f32,
    pub period_frames: Option<u32>,
    pub follow_camera: bool,
    pub gravity_world_per_sec2: f32,
    pub cone_latitude_deg: Option<(f32, f32)>,
    pub size_shrink: bool,
    pub twinkle: bool,
    pub curve: Option<CurveParams>,
    pub alpha_keyframes: &'static [AlphaKeyframe],
}

#[derive(Clone, Copy, Debug)]
pub struct CurveParams {
    pub initial_period_frames: (u32, u32),
    pub subsequent_period_frames: (u32, u32),
    pub angle_jitter_deg: f32,
    pub speed_resample: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct AlphaKeyframe {
    pub at_frame: u32,
    pub alpha_init: f32,
    pub alpha_max: f32,
}
