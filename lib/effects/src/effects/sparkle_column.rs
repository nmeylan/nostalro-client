use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const COLUMN_HEIGHT: f32 = 11.0;
const SEED_DEPTH: f32 = 5.0;
const DRIFT_AMP_PER_S: f32 = 1.2;
const DRIFT_SPEED_RAD_PER_S: f32 = 2.4;
const FADE_TOP_FRACTION: f32 = 0.35;
const FADE_IN_DEPTH: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct SparkleColumnParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub alpha_max: f32,
    pub sprite_radius: f32,
    pub rise_speed: f32,
    pub count: usize,
    pub scatter: f32,
}

pub const FREEZING: SparkleColumnParams = SparkleColumnParams {
    texture: "freezing_circle.bmp",
    color_rgb: [1.00, 1.00, 1.00],
    alpha_max: 0.80,
    sprite_radius: 0.8,
    rise_speed: 6.0,
    count: 16,
    scatter: 0.5,
};

pub const WHITELIGHT: SparkleColumnParams = SparkleColumnParams {
    texture: "whitelight.tga",
    color_rgb: [0.31, 0.31, 1.00],
    alpha_max: 0.85,
    sprite_radius: 3.6,
    rise_speed: 6.0,
    count: 20,
    scatter: 2.0,
};

pub const GHOST: SparkleColumnParams = SparkleColumnParams {
    texture: "ghost.bmp",
    color_rgb: [0.60, 0.60, 0.60],
    alpha_max: 0.70,
    sprite_radius: 3.2,
    rise_speed: 9.0,
    count: 4,
    scatter: 7.0,
};

pub const TEXTURES: &[&str] = &["freezing_circle.bmp", "whitelight.tga", "ghost.bmp"];

#[derive(Clone, Copy)]
struct Mote {
    base_xz: [f32; 2],
    y_offset: f32,
    wobble: f32,
    wobble_dir: f32,
}

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

fn lcg_float(state: &mut u32) -> f32 {
    (lcg_next(state) >> 8) as f32 / ((1u32 << 24) as f32)
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
        for _ in 0..params.count {
            motes.push(seed_mote(&mut rng_state, &params, true));
        }
        Self {
            params,
            world_pos,
            motes,
            rng_state,
        }
    }
}

/// Build a fresh mote. `initial` spreads the y-offset across the whole rise
/// span so the column is already populated at spawn; respawns start below the
/// ground in `[0, SEED_DEPTH]`.
fn seed_mote(rng: &mut u32, params: &SparkleColumnParams, initial: bool) -> Mote {
    let angle = lcg_float(rng) * std::f32::consts::TAU;
    let radius = params.scatter * lcg_float(rng).sqrt();
    let y_offset = if initial {
        // Span ground..top minus a bit below, so motes already fill the column.
        SEED_DEPTH - lcg_float(rng) * (SEED_DEPTH + COLUMN_HEIGHT)
    } else {
        lcg_float(rng) * SEED_DEPTH
    };
    Mote {
        base_xz: [radius * angle.cos(), radius * angle.sin()],
        y_offset,
        wobble: lcg_float(rng) * std::f32::consts::TAU,
        wobble_dir: lcg_float(rng) * std::f32::consts::TAU,
    }
}

impl Effect for SparkleColumnEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta;
        for i in 0..self.motes.len() {
            let m = &mut self.motes[i];
            m.y_offset -= self.params.rise_speed * dt;
            m.wobble += DRIFT_SPEED_RAD_PER_S * dt;
            if m.y_offset < -COLUMN_HEIGHT {
                self.motes[i] = seed_mote(&mut self.rng_state, &self.params, false);
            }
        }
        EffectStatus::Running
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let size = self.params.sprite_radius * 2.0;
        let fade_top_start = -COLUMN_HEIGHT * (1.0 - FADE_TOP_FRACTION);
        for m in &self.motes {
            // Only visible once it clears the ground.
            if m.y_offset > 0.0 {
                continue;
            }
            // Fade in just above ground, fade out near the top.
            let fade_in = (-m.y_offset / FADE_IN_DEPTH).clamp(0.0, 1.0);
            let fade_out = if m.y_offset > fade_top_start {
                1.0
            } else {
                ((m.y_offset - (-COLUMN_HEIGHT)) / (fade_top_start - (-COLUMN_HEIGHT)))
                    .clamp(0.0, 1.0)
            };
            let alpha = self.params.alpha_max * fade_in * fade_out;
            if alpha <= 0.0 {
                continue;
            }
            let drift = DRIFT_AMP_PER_S * (m.wobble).sin();
            let pos = [
                self.world_pos[0] + m.base_xz[0] + drift * m.wobble_dir.cos(),
                self.world_pos[1] + m.y_offset,
                self.world_pos[2] + m.base_xz[1] + drift * m.wobble_dir.sin(),
            ];
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                size: [size, size],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: self.params.texture,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
            });
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

    #[test]
    fn only_motes_above_ground_are_drawn_as_additive_billboards() {
        let c = SparkleColumnEffect::new([5.0, 0.0, 5.0], FREEZING);
        for p in draws(&c) {
            let EffectPrimitiveDraw::Billboard { pos, blend, .. } = p else {
                panic!()
            };
            assert_eq!(blend, BlendKind::Additive);
            // Visible motes are at/above ground: native RO up = negative y.
            assert!(pos[1] <= 0.0 + 1e-4);
        }
    }

    #[test]
    fn motes_rise_over_time() {
        let mut c = SparkleColumnEffect::new([0.0; 3], FREEZING);
        // Track the lowest mote (largest y_offset) so it can't respawn during
        // the step and the comparison stays on the same particle.
        let lowest = |c: &SparkleColumnEffect| {
            c.motes
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.y_offset.partial_cmp(&b.1.y_offset).unwrap())
                .map(|(i, m)| (i, m.y_offset))
                .unwrap()
        };
        let (idx, y0) = lowest(&c);
        step(&mut c, 0.1);
        let y1 = c.motes[idx].y_offset;
        assert!(
            y1 < y0,
            "mote should rise (y decreases in native RO): {y0} → {y1}"
        );
    }

    #[test]
    fn mote_count_stays_constant_through_respawn() {
        let mut c = SparkleColumnEffect::new([0.0; 3], FREEZING);
        let before = c.motes.len();
        // Run long enough that several motes cross the top and respawn.
        for _ in 0..200 {
            step(&mut c, 0.05);
        }
        assert_eq!(c.motes.len(), before, "respawn must not change population");
    }

    #[test]
    fn ghost_scatters_much_wider_than_freezing() {
        let ghost = SparkleColumnEffect::new([0.0; 3], GHOST);
        let freezing = SparkleColumnEffect::new([0.0; 3], FREEZING);
        let spread = |c: &SparkleColumnEffect| {
            c.motes
                .iter()
                .map(|m| (m.base_xz[0].powi(2) + m.base_xz[1].powi(2)).sqrt())
                .fold(0.0f32, f32::max)
        };
        // Ghost's wide footprint (scatter 7) vs the freezing column's tight
        // cluster (scatter 1.5) — both bounded by their `scatter` radius.
        assert!(
            spread(&ghost) > spread(&freezing) * 2.0,
            "ghost should scatter far wider"
        );
        assert!(spread(&freezing) <= FREEZING.scatter + 1e-4);
        assert!(spread(&ghost) <= GHOST.scatter + 1e-4);
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
