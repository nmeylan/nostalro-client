//! `EF_LEVEL993` (#202) / `EF_LEVEL994` (#362) / `EF_MAP_GHOST` (#692) — the
//! rising sparkle-mote layer of the level-99 aura (and a map-ambient reuse).
//!
//! A set of camera-facing motes that rise out of the ground, drift sideways
//! on a sine wobble, fade as they reach the top, then respawn at the bottom.
//! A variant selector chooses the texture, mote count, rise speed and
//! scatter:
//!
//! | id  | F1 | texture            | sprite radius | rise | scatter | tint        |
//! |-----|----|--------------------|---------------|------|---------|-------------|
//! | 202 | 0  | freezing_circle    | small         | slow | none    | white       |
//! | 362 | 1  | whitelight         | medium        | slow | none    | blue        |
//! | 692 | 3  | ghost              | large         | fast | yes     | grey        |
//!
//! Each mote seeds below the ground (in
//! native RO coords where positive y is *below* ground), rises by
//! a fixed step per frame (decreasing y), draws only once `y <= 0`, fades
//! near the top, and respawns it past the top edge. We keep the same
//! lifecycle with an explicit mote list (the [`super::stormgust`] pattern) and
//! emit one additive [`Billboard`] per visible mote.
//!
//! Persistent: lives until the server clears it (table ships `u32::MAX`).
//!
//! [`Billboard`]: EffectPrimitiveDraw::Billboard

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// World-space rise span: motes are visible from ground (offset 0) up to
/// `-COLUMN_HEIGHT` (native RO: negative y = up), then respawn. ~2 character
/// heights of streaming sparkles.
const COLUMN_HEIGHT: f32 = 11.0;

/// Motes spawn staggered below ground in `[0, SEED_DEPTH]` so they emerge over
/// time instead of all at once.
const SEED_DEPTH: f32 = 5.0;

/// Sideways wobble: horizontal drift amplitude (world units/sec) and angular
/// speed of the wobble phase.
const DRIFT_AMP_PER_S: f32 = 1.2;
const DRIFT_SPEED_RAD_PER_S: f32 = 2.4;

/// Fraction of the rise over which a mote fades out as it nears the top.
const FADE_TOP_FRACTION: f32 = 0.35;
/// World-space distance over which a mote fades in as it clears the ground.
const FADE_IN_DEPTH: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct SparkleColumnParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub alpha_max: f32,
    /// Half-extent of each mote billboard, in world units.
    pub sprite_radius: f32,
    /// Rise speed, world units/sec (native RO up).
    pub rise_speed: f32,
    /// Number of motes alive at once.
    pub count: usize,
    /// Horizontal scatter radius around the caster (0 = column hugs the axis).
    pub scatter: f32,
}

/// `EF_LEVEL993` — freezing-circle motes hugging the caster.
pub const FREEZING: SparkleColumnParams = SparkleColumnParams {
    texture: "freezing_circle.bmp",
    color_rgb: [1.00, 1.00, 1.00],
    alpha_max: 0.85,
    sprite_radius: 0.9,
    rise_speed: 6.0,
    count: 20,
    scatter: 1.5,
};

/// `EF_LEVEL994` — blue whitelight motes, wider spread.
pub const WHITELIGHT: SparkleColumnParams = SparkleColumnParams {
    texture: "whitelight.tga",
    color_rgb: [0.31, 0.31, 1.00],
    alpha_max: 0.85,
    sprite_radius: 3.6,
    rise_speed: 6.0,
    count: 20,
    scatter: 2.0,
};

/// `EF_MAP_GHOST` — large grey ghost motes scattered across a
/// wide footprint, rising faster. Map-ambient rather than caster-attached.
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
    /// Horizontal offset from the caster (world units).
    base_xz: [f32; 2],
    /// Vertical offset in native RO coords: positive = below ground (hidden),
    /// 0 = ground, negative = up.
    y_offset: f32,
    /// Wobble phase (radians) and per-mote phase offset.
    wobble: f32,
    wobble_dir: f32,
}

/// Deterministic LCG (no `rand` dependency, reproducible viewer exports).
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
        let mut rng_state = 0x9E37_79B9 ^ world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13);
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
                ((m.y_offset - (-COLUMN_HEIGHT)) / (fade_top_start - (-COLUMN_HEIGHT))).clamp(0.0, 1.0)
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
        c.update(&EffectUpdateCtx { delta: dt, camera_target: None });
    }

    #[test]
    fn only_motes_above_ground_are_drawn_as_additive_billboards() {
        let c = SparkleColumnEffect::new([5.0, 0.0, 5.0], FREEZING);
        for p in draws(&c) {
            let EffectPrimitiveDraw::Billboard { pos, blend, .. } = p else { panic!() };
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
            c.motes.iter().enumerate().max_by(|a, b| {
                a.1.y_offset.partial_cmp(&b.1.y_offset).unwrap()
            }).map(|(i, m)| (i, m.y_offset)).unwrap()
        };
        let (idx, y0) = lowest(&c);
        step(&mut c, 0.1);
        let y1 = c.motes[idx].y_offset;
        assert!(y1 < y0, "mote should rise (y decreases in native RO): {y0} → {y1}");
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
        assert!(spread(&ghost) > spread(&freezing) * 2.0, "ghost should scatter far wider");
        assert!(spread(&freezing) <= FREEZING.scatter + 1e-4);
        assert!(spread(&ghost) <= GHOST.scatter + 1e-4);
    }

    #[test]
    fn variants_use_real_distinct_textures() {
        let texs = [FREEZING.texture, WHITELIGHT.texture, GHOST.texture];
        for t in texs {
            assert!(TEXTURES.contains(&t));
        }
        assert_eq!(texs.iter().collect::<std::collections::HashSet<_>>().len(), 3);
    }

    #[test]
    fn never_self_terminates() {
        let mut c = SparkleColumnEffect::new([0.0; 3], GHOST);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1, camera_target: None }), EffectStatus::Running);
        }
    }
}
