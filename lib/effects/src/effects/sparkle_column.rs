use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

const DRIFT_PER_FRAME: f32 = 0.15;
const WANDER_UP_DEG: f32 = 3.0;
const WANDER_DOWN_DEG: f32 = 2.0;
const ANGLE_MAX: f32 = 359.0;
/// Respawn depth, in world units below the ground plane.
const SEED_DEPTH: u32 = 100;
const ALPHA_FULL: f32 = 250.0;
const FADE_SLOPE: f32 = 25.0;

const DRIFT_SIGNS: [[f32; 2]; 4] = [[1.0, 1.0], [-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0]];

#[derive(Clone, Copy, Debug)]
pub struct SparkleColumnParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    /// Half-diagonal of the screen-facing quad; the side is this times √2.
    pub distance: f32,
    pub rise_per_frame: f32,
    pub count: usize,
    /// 0 spawns on the actor axis; otherwise the offset spans `±spawn_jitter`.
    pub spawn_jitter: i32,
    pub fade_start: f32,
    pub respawn_y: f32,
    pub draws: u8,
}

pub const FREEZING: SparkleColumnParams = SparkleColumnParams {
    texture: "freezing_circle.bmp",
    color_rgb: [1.00, 1.00, 1.00],
    distance: 0.8,
    rise_per_frame: 0.15,
    count: 16,
    spawn_jitter: 0,
    fade_start: -20.0,
    respawn_y: -30.0,
    draws: 2,
};

pub const WHITELIGHT: SparkleColumnParams = SparkleColumnParams {
    texture: "whitelight.tga",
    color_rgb: [80.0 / 255.0, 80.0 / 255.0, 1.00],
    distance: 2.4,
    rise_per_frame: 0.15,
    count: 16,
    spawn_jitter: 0,
    fade_start: -20.0,
    respawn_y: -30.0,
    draws: 2,
};

pub const GREEN99: SparkleColumnParams = SparkleColumnParams {
    texture: "whitelight.tga",
    color_rgb: [0.14, 1.00, 0.14],
    distance: 0.8,
    rise_per_frame: 0.15,
    count: 16,
    spawn_jitter: 0,
    fade_start: -20.0,
    respawn_y: -30.0,
    draws: 2,
};

pub const GHOST: SparkleColumnParams = SparkleColumnParams {
    texture: "ghost.bmp",
    color_rgb: [155.0 / 255.0, 155.0 / 255.0, 155.0 / 255.0],
    distance: 3.2,
    rise_per_frame: 0.6,
    count: 4,
    spawn_jitter: 8,
    fade_start: -40.0,
    respawn_y: -50.0,
    draws: 1,
};

pub const TEXTURES: &[&str] = &["freezing_circle.bmp", "whitelight.tga", "ghost.bmp"];

#[derive(Clone, Copy)]
struct Mote {
    /// Offset from the actor; native RO up is negative y, so a positive y is
    /// below the ground plane.
    offset: [f32; 3],
    angle: [f32; 2],
    target: [f32; 2],
    sign: [f32; 2],
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn rand_below(state: &mut u32, bound: u32) -> u32 {
    if bound == 0 {
        0
    } else {
        lcg_next(state) % bound
    }
}

fn rand_angle(state: &mut u32) -> f32 {
    rand_below(state, 360) as f32
}

fn spawn_offset(state: &mut u32, params: &SparkleColumnParams) -> f32 {
    if params.spawn_jitter == 0 {
        return 0.0;
    }
    let span = (params.spawn_jitter * 2 - 1) as u32;
    rand_below(state, span) as f32 - params.spawn_jitter as f32
}

fn respawn(m: &mut Mote, state: &mut u32, params: &SparkleColumnParams) {
    m.offset = [
        spawn_offset(state, params),
        rand_below(state, SEED_DEPTH) as f32,
        spawn_offset(state, params),
    ];
}

fn wander(m: &mut Mote, k: usize, steps: f32, state: &mut u32) {
    if m.target[k] > m.angle[k] {
        m.angle[k] = (m.angle[k] + WANDER_UP_DEG * steps).min(ANGLE_MAX);
        if m.angle[k] > m.target[k] {
            m.target[k] = rand_below(state, m.angle[k] as u32) as f32;
        }
    } else {
        m.angle[k] = (m.angle[k] - WANDER_DOWN_DEG * steps).max(0.0);
        if m.angle[k] <= m.target[k] {
            m.target[k] = ANGLE_MAX - rand_below(state, m.angle[k] as u32) as f32;
        }
    }
}

fn alpha_255(params: &SparkleColumnParams, y: f32) -> f32 {
    if y > params.fade_start {
        ALPHA_FULL
    } else {
        (ALPHA_FULL + (y - params.fade_start) * FADE_SLOPE).max(0.0)
    }
}

pub struct SparkleColumnEffect {
    params: SparkleColumnParams,
    world_pos: [f32; 3],
    motes: Vec<Mote>,
    rng_state: u32,
}

impl SparkleColumnEffect {
    pub fn new(world_pos: [f32; 3], params: SparkleColumnParams) -> Self {
        let mut rng_state =
            0x9E37_79B9 ^ world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13);
        let mut motes = Vec::with_capacity(params.count);
        for i in 0..params.count {
            let mut m = Mote {
                offset: [0.0; 3],
                angle: [rand_angle(&mut rng_state), rand_angle(&mut rng_state)],
                target: [rand_angle(&mut rng_state), rand_angle(&mut rng_state)],
                sign: DRIFT_SIGNS[i % DRIFT_SIGNS.len()],
            };
            respawn(&mut m, &mut rng_state, &params);
            motes.push(m);
        }
        Self {
            params,
            world_pos,
            motes,
            rng_state,
        }
    }
}

impl Effect for SparkleColumnEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let steps = ctx.delta * FRAMES_PER_SECOND;
        let params = &self.params;
        let state = &mut self.rng_state;
        for m in &mut self.motes {
            wander(m, 0, steps, state);
            wander(m, 1, steps, state);
            if m.offset[1] < 0.0 {
                m.offset[0] += DRIFT_PER_FRAME * m.angle[0].to_radians().sin() * m.sign[0] * steps;
                m.offset[2] += DRIFT_PER_FRAME * m.angle[1].to_radians().sin() * m.sign[1] * steps;
                if m.offset[1] < params.respawn_y {
                    respawn(m, state, params);
                }
            }
            m.offset[1] -= params.rise_per_frame * steps;
        }
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let side = self.params.distance * std::f32::consts::SQRT_2;
        for m in &self.motes {
            let y = m.offset[1];
            if y > 0.0 {
                continue;
            }
            let alpha = alpha_255(&self.params, y) / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let pos = [
                self.world_pos[0] + m.offset[0],
                self.world_pos[1] + y,
                self.world_pos[2] + m.offset[2],
            ];
            for _ in 0..self.params.draws {
                out.push(EffectPrimitiveDraw::Billboard {
                    pos,
                    size: [side, side],
                    uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                    rotation: 0.0,
                    texture: self.params.texture,
                    color: [r, g, b, alpha],
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

    fn draws(c: &SparkleColumnEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(c: &mut SparkleColumnEffect, dt: f32) {
        c.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn lit(c: &SparkleColumnEffect) -> usize {
        c.motes
            .iter()
            .filter(|m| m.offset[1] <= 0.0 && alpha_255(&c.params, m.offset[1]) > 0.0)
            .count()
    }

    #[test]
    fn freezing_column_spawns_on_axis_then_drifts_out_at_a_third_duty_cycle() {
        let mut c = SparkleColumnEffect::new([5.0, 0.0, 5.0], FREEZING);
        assert!(
            c.motes
                .iter()
                .all(|m| m.offset[0] == 0.0 && m.offset[2] == 0.0),
            "motes are born on the actor axis"
        );

        let frames = 600;
        let mut lit_total = 0usize;
        let mut max_horizontal = 0.0f32;
        for _ in 0..frames {
            step(&mut c, 1.0 / FRAMES_PER_SECOND);
            lit_total += lit(&c);
            for m in &c.motes {
                max_horizontal = max_horizontal.max(m.offset[0].abs().max(m.offset[2].abs()));
            }

            let d = draws(&c);
            assert_eq!(d.len(), lit(&c) * 2, "two additive quads per lit mote");
            for p in &d {
                let EffectPrimitiveDraw::Billboard {
                    pos,
                    size,
                    color,
                    blend,
                    ..
                } = p
                else {
                    panic!()
                };
                assert_eq!(*blend, BlendKind::Additive);
                assert!(pos[1] <= 1e-4, "lit motes have cleared the ground");
                assert!((size[0] - 0.8 * std::f32::consts::SQRT_2).abs() < 1e-5);
                let expected = alpha_255(&FREEZING, pos[1]);
                assert!((color[3] * 255.0 - expected).abs() < 1e-3, "alpha ramp");
            }
        }

        let avg = lit_total as f32 / frames as f32;
        assert!(
            (3.0..=9.0).contains(&avg),
            "about a third of the 16 motes should be lit, got {avg}"
        );
        assert!(
            max_horizontal > 3.0,
            "the random walk should carry motes well off the axis, got {max_horizontal}"
        );
    }

    #[test]
    fn mote_count_stays_constant_through_respawn() {
        let mut c = SparkleColumnEffect::new([0.0; 3], FREEZING);
        let before = c.motes.len();
        for _ in 0..2000 {
            step(&mut c, 1.0 / FRAMES_PER_SECOND);
        }
        assert_eq!(c.motes.len(), before, "respawn must not change population");
    }

    #[test]
    fn variants_use_real_distinct_textures() {
        let texs = [FREEZING.texture, WHITELIGHT.texture, GHOST.texture];
        for t in texs {
            assert!(TEXTURES.contains(&t));
        }
        assert_eq!(
            texs.iter().collect::<std::collections::HashSet<_>>().len(),
            3
        );
    }

    #[test]
    fn never_self_terminates() {
        let mut c = SparkleColumnEffect::new([0.0; 3], GHOST);
        for _ in 0..200 {
            assert_eq!(
                c.update(&EffectUpdateCtx {
                    delta: 0.1,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }
}
