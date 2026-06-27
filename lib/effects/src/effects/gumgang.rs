use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[
    "super1.bmp",
    "super2.bmp",
    "super3.bmp",
    "super4.bmp",
    "super5.bmp",
];

const FRAMES_PER_SECOND: f32 = 60.0;

const PERSISTENT_DURATION_MS: u32 = 999_990;

const RINGS_PER_EMITTER: usize = 4;

const GROW_FRAMES: f32 = 10.0;
const GROW_PER_FRAME: f32 = 25.0;
const SHRINK_PER_FRAME: f32 = 12.0;
const RESPAWN_BELOW: f32 = 12.0;
const COUNTER_INIT: f32 = 10.0;
const ALPHA_DIVISOR: f32 = 255.0;
const MIN_ALPHA: f32 = 1.0 / ALPHA_DIVISOR;

const SQRT2: f32 = std::f32::consts::SQRT_2;

const ORBIT_LENGTH: f32 = 4.0;
const Y_BASE: f32 = 3.0;
const Y_RAND: f32 = 8.0;
const WORLD_SCALE: f32 = 0.7;
const FADE_IN_FRAMES: f32 = 16.0;

#[derive(Clone, Copy, Debug)]
pub struct GumGangLayer {
    pub rdist: f32,
    pub color_rgb: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct GumGangParams {
    pub emitters: u8,
    pub distance: f32,
    pub ring_init: [f32; RINGS_PER_EMITTER],
    pub layers: &'static [GumGangLayer],
    pub lifetime_ms: Option<u32>,
}

impl GumGangParams {
    pub const fn total_duration_ms(&self) -> u32 {
        match self.lifetime_ms {
            Some(ms) => ms,
            None => PERSISTENT_DURATION_MS,
        }
    }
}

const fn norm(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

const GUMGANG_RINGS: [f32; RINGS_PER_EMITTER] = [70.0, 130.0, 190.0, 250.0];
const DOUBLE_RINGS: [f32; RINGS_PER_EMITTER] = [40.0, 80.0, 120.0, 160.0];

const BLUE_SINGLE: &[GumGangLayer] = &[GumGangLayer {
    rdist: 1.0,
    color_rgb: norm(100, 100, 255),
}];
const RED_SINGLE: &[GumGangLayer] = &[GumGangLayer {
    rdist: 1.0,
    color_rgb: norm(255, 20, 20),
}];

const RED_TRIPLE: &[GumGangLayer] = &[
    GumGangLayer {
        rdist: 1.0,
        color_rgb: norm(255, 100, 100),
    },
    GumGangLayer {
        rdist: 1.2,
        color_rgb: norm(120, 0, 0),
    },
    GumGangLayer {
        rdist: 2.0,
        color_rgb: norm(120, 0, 0),
    },
];
const WHITE_TRIPLE: &[GumGangLayer] = &[
    GumGangLayer {
        rdist: 1.0,
        color_rgb: norm(90, 90, 80),
    },
    GumGangLayer {
        rdist: 1.2,
        color_rgb: norm(60, 60, 50),
    },
    GumGangLayer {
        rdist: 2.0,
        color_rgb: norm(30, 30, 20),
    },
];
const BLUE_TRIPLE: &[GumGangLayer] = &[
    GumGangLayer {
        rdist: 1.0,
        color_rgb: norm(100, 100, 255),
    },
    GumGangLayer {
        rdist: 1.2,
        color_rgb: norm(0, 0, 120),
    },
    GumGangLayer {
        rdist: 2.0,
        color_rgb: norm(0, 0, 120),
    },
];

pub const GUMGANG: GumGangParams = GumGangParams {
    emitters: 1,
    distance: 3.5,
    ring_init: GUMGANG_RINGS,
    layers: BLUE_SINGLE,
    lifetime_ms: None,
};
pub const STEELBODY: GumGangParams = GumGangParams {
    emitters: 4,
    distance: 3.5,
    ring_init: GUMGANG_RINGS,
    layers: BLUE_SINGLE,
    lifetime_ms: None,
};
pub const GUMGANGNPC: GumGangParams = GumGangParams {
    emitters: 2,
    distance: 3.5,
    ring_init: GUMGANG_RINGS,
    layers: RED_SINGLE,
    lifetime_ms: Some(1500),
};
pub const DOUBLE_RED: GumGangParams = GumGangParams {
    emitters: 2,
    distance: 2.0,
    ring_init: DOUBLE_RINGS,
    layers: RED_TRIPLE,
    lifetime_ms: None,
};
pub const DOUBLE_WHITE: GumGangParams = GumGangParams {
    emitters: 2,
    distance: 2.0,
    ring_init: DOUBLE_RINGS,
    layers: WHITE_TRIPLE,
    lifetime_ms: None,
};
pub const DOUBLE_BLUE: GumGangParams = GumGangParams {
    emitters: 2,
    distance: 2.0,
    ring_init: DOUBLE_RINGS,
    layers: BLUE_TRIPLE,
    lifetime_ms: None,
};

fn lcg(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn rand_range(state: &mut u32, max: f32) -> f32 {
    ((lcg(state) >> 8) as f32 / (1u32 << 24) as f32) * max
}

struct Streak {
    rng: u32,
    counter: f32,
    pulse: f32,
    angle: f32,
    y_off: f32,
    tex: usize,
}

impl Streak {
    fn new(seed: u32, init_pulse: f32) -> Self {
        let mut s = Self {
            rng: seed,
            counter: COUNTER_INIT,
            pulse: init_pulse,
            angle: 0.0,
            y_off: 0.0,
            tex: 0,
        };
        s.scatter();
        s
    }

    fn scatter(&mut self) {
        self.angle = rand_range(&mut self.rng, std::f32::consts::TAU);
        self.y_off = -(Y_BASE + rand_range(&mut self.rng, Y_RAND));
        self.tex =
            (rand_range(&mut self.rng, TEXTURES.len() as f32) as usize).min(TEXTURES.len() - 1);
    }

    fn respawn(&mut self) {
        self.counter = 0.0;
        self.pulse = 0.0;
        self.scatter();
    }

    fn tick(&mut self, delta_frames: f32) {
        self.counter += delta_frames;
        if self.counter < GROW_FRAMES {
            self.pulse += GROW_PER_FRAME * delta_frames;
        } else {
            self.pulse -= SHRINK_PER_FRAME * delta_frames;
            if self.pulse < RESPAWN_BELOW {
                self.respawn();
            }
        }
    }
}

pub struct GumGangEffect {
    params: GumGangParams,
    world_pos: [f32; 3],
    age_frames: f32,
    streaks: Vec<Streak>,
}

impl GumGangEffect {
    pub fn new(world_pos: [f32; 3], params: GumGangParams) -> Self {
        let mut streaks = Vec::with_capacity(params.emitters as usize * RINGS_PER_EMITTER);
        for ec in 0..params.emitters as usize {
            for ring in 0..RINGS_PER_EMITTER {
                let idx = ec * RINGS_PER_EMITTER + ring;
                let seed = (idx as u32)
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(0x9E37_79B9);
                streaks.push(Streak::new(seed, params.ring_init[ring]));
            }
        }
        Self {
            params,
            world_pos,
            age_frames: 0.0,
            streaks,
        }
    }
}

fn alpha_at(frame: f32) -> f32 {
    (frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
}

fn mirror_texture(on_screen_right: bool, tex: usize) -> bool {
    let spikes_right_unflipped = tex == 0 || tex == 2;
    let want_spikes_right = !on_screen_right;
    want_spikes_right != spikes_right_unflipped
}

impl Effect for GumGangEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let delta_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += delta_frames;
        for streak in &mut self.streaks {
            streak.tick(delta_frames);
        }

        if let Some(ms) = self.params.lifetime_ms {
            let total_frames = ms as f32 / 1000.0 * FRAMES_PER_SECOND;
            if self.age_frames >= total_frames {
                return EffectStatus::Dead;
            }
        }
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        let fade = alpha_at(self.age_frames);
        if fade <= 0.0 {
            return;
        }
        let cam = &ctx.camera;
        let fwd = [
            cam.target[0] - cam.eye[0],
            cam.target[1] - cam.eye[1],
            cam.target[2] - cam.eye[2],
        ];
        let right = [
            fwd[1] * cam.up[2] - fwd[2] * cam.up[1],
            fwd[2] * cam.up[0] - fwd[0] * cam.up[2],
            fwd[0] * cam.up[1] - fwd[1] * cam.up[0],
        ];

        let [wx, wy, wz] = self.world_pos;
        for streak in &self.streaks {
            let alpha = (streak.pulse / ALPHA_DIVISOR).clamp(0.0, 1.0) * fade;
            if alpha < MIN_ALPHA {
                continue;
            }
            let (sin_a, cos_a) = streak.angle.sin_cos();
            let pos = [
                wx + cos_a * ORBIT_LENGTH * WORLD_SCALE,
                wy + streak.y_off * WORLD_SCALE,
                wz + sin_a * ORBIT_LENGTH * WORLD_SCALE,
            ];
            let on_screen_right = cos_a * right[0] + sin_a * right[2] > 0.0;
            let uv = if mirror_texture(on_screen_right, streak.tex) {
                [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
            } else {
                [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]
            };
            for layer in self.params.layers {
                let side = SQRT2 * self.params.distance * layer.rdist * WORLD_SCALE;
                let [cr, cg, cb] = layer.color_rgb;
                out.push(EffectPrimitiveDraw::Billboard {
                    pos,
                    size: [side, side],
                    uv,
                    rotation: 0.0,
                    texture: TEXTURES[streak.tex],
                    color: [cr, cg, cb, alpha],
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut GumGangEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &GumGangEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn billboard(
        p: &EffectPrimitiveDraw,
    ) -> ([f32; 3], [f32; 2], [f32; 4], &'static str, BlendKind) {
        match p {
            EffectPrimitiveDraw::Billboard {
                pos,
                size,
                color,
                texture,
                blend,
                ..
            } => (*pos, *size, *color, *texture, *blend),
            _ => panic!("expected Billboard, got {:?}", p),
        }
    }

    #[test]
    fn emitter_count_scales_billboard_count() {
        let mut single = GumGangEffect::new([5.0, 0.0, -3.0], GUMGANG);
        let mut steel = GumGangEffect::new([0.0; 3], STEELBODY);
        step(&mut single, 1.0);
        step(&mut steel, 1.0);
        assert_eq!(draws(&single).len(), RINGS_PER_EMITTER);
        assert_eq!(draws(&steel).len(), 4 * RINGS_PER_EMITTER);
        for p in draws(&single) {
            let (_, _, _, tex, blend) = billboard(&p);
            assert!(TEXTURES.contains(&tex));
            assert_eq!(blend, BlendKind::Additive);
        }
    }

    #[test]
    fn double_family_draws_three_concentric_darkening_layers() {
        let center = [0.0; 3];
        let mut e = GumGangEffect::new(center, DOUBLE_BLUE);
        step(&mut e, 1.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 2 * RINGS_PER_EMITTER * 3);

        let p0 = billboard(&prims[0]);
        let p1 = billboard(&prims[1]);
        let p2 = billboard(&prims[2]);
        assert_eq!(p0.0, p1.0, "layers share a position");
        assert_eq!(p1.0, p2.0, "layers share a position");

        let (s0, s1, s2) = (p0.1[0], p1.1[0], p2.1[0]);
        assert!(
            (s1 / s0 - 1.2).abs() < 0.01,
            "mid layer ≈ 1.2× ({s0} -> {s1})"
        );
        assert!(
            (s2 / s0 - 2.0).abs() < 0.01,
            "outer layer ≈ 2.0× ({s0} -> {s2})"
        );

        let lum = |c: [f32; 4]| c[0] + c[1] + c[2];
        assert!(lum(p0.2) > lum(p2.2), "outer layer must be darker");
    }

    #[test]
    fn quad_is_fixed_size_while_alpha_pulses() {
        let mut e = GumGangEffect::new([0.0; 3], GUMGANG);
        step(&mut e, FADE_IN_FRAMES + 1.0); // past fade-in so `fade` is 1.0
        let expected_side = SQRT2 * GUMGANG.distance * WORLD_SCALE;

        let mut alphas = Vec::new();
        for _ in 0..12 {
            step(&mut e, 2.0);
            for p in draws(&e) {
                let (_, size, color, ..) = billboard(&p);
                assert!((size[0] - expected_side).abs() < 1e-3, "side must be fixed");
                assert_eq!(size[0], size[1], "quad must be square");
                alphas.push(color[3]);
            }
        }
        let min = alphas.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = alphas.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!(max - min > 0.1, "alpha must pulse over time ({min}..{max})");
    }

    #[test]
    fn persistent_vs_finite_lifetime() {
        let mut buff = GumGangEffect::new([0.0; 3], GUMGANG);
        assert!(matches!(step(&mut buff, 600.0), EffectStatus::Running));

        let mut npc = GumGangEffect::new([0.0; 3], GUMGANGNPC);
        let finite = GUMGANGNPC.lifetime_ms.unwrap() as f32 / 1000.0 * FRAMES_PER_SECOND;
        assert!(matches!(step(&mut npc, finite + 1.0), EffectStatus::Dead));
    }

    #[test]
    fn variants_keep_distinct_tints() {
        assert_ne!(GUMGANG.layers[0].color_rgb, GUMGANGNPC.layers[0].color_rgb);
        assert_ne!(
            DOUBLE_RED.layers[0].color_rgb,
            DOUBLE_BLUE.layers[0].color_rgb
        );
        assert_ne!(
            DOUBLE_WHITE.layers[0].color_rgb,
            DOUBLE_BLUE.layers[0].color_rgb
        );
    }
}
