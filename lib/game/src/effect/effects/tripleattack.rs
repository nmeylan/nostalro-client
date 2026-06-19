//! TripleAttack family — `EF_TRIPLEATTACK` (329), `EF_TRIPLEATTACK2` (388,
//! sharpshooting) and `EF_TRIPLEATTACK3` (393, arrow vulcan).
//!
//! Faithful reproduction of the original game's triple-attack streaks. Each
//! launch builds four thin streaks pointing from the
//! caster toward the target. A streak is a static, short-lived elongated tube
//! running from its base to `base + distance·(cos h, 0, sin h)`
//! — it does not travel; it flashes in (alpha steps up for the first five
//! frames) then fades out. The launcher re-fires every frame for the first
//! `N` frames, so the effect is a staggered volley of streaks; each
//! emitter also starts with a random negative spawn delay.
//!
//! * **Tripleattack** (329): yellow, `cloud11.tga`, two launch frames, a fixed
//!   20-unit reach (independent of the actual target distance), bright snappy
//!   fade — the three quick melee slashes.
//! * **Tripleattack2** (388, sharpshooting): white, `alpha_center.tga`, four
//!   launch frames, reaches the target, long faint fade.
//! * **Tripleattack3** (393, arrow vulcan): magenta, `alpha_center.tga`, nine
//!   launch frames, reaches the target, the densest/longest stream.
//!
//! Each streak reads as a pentagon-section tube; the visible
//! silhouette is a thin camera-facing ribbon following that world segment, so we
//! render it as an [`EffectPrimitiveDraw::LineStrip`]. Two layers per streak — a
//! bright inner core and a wider paler halo — give the
//! inner / outer radii. Additive blend; the heading
//! follows the shared convention `dz.atan2(dx)` (`cos → x`, `sin → z`).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TEXTURES: &[&str] = &["cloud11.tga", "alpha_center.tga"];

/// Each launch wave fires four streaks.
const STREAKS_PER_WAVE: usize = 4;
/// The first 5 frames ramp the alpha up; afterwards it ramps down.
const RAMP_FRAMES: i32 = 5;

/// Per-variant parameters. All spatial literals are in world units
/// (`fixed_reach = 20` maps ~1:1 onto our caster→target trail length), scaled
/// uniformly by [`WORLD_SCALE`].
#[derive(Clone, Copy)]
pub struct TripleAttackParams {
    texture: &'static str,
    /// Bright inner-core tint (RGB, 0..1); alpha is filled per-frame.
    inner_color: [f32; 3],
    /// Wider pale halo tint (RGB, 0..1).
    outer_color: [f32; 3],
    /// Number of per-frame launch waves.
    waves: u32,
    /// Half-width of the random XZ spawn jitter (`±offset_xz`).
    offset_xz: f32,
    /// Base + random vertical lift of the streak above the caster.
    lift_base: f32,
    lift_rand: f32,
    /// Inner-core ribbon half-width; random component for 393.
    radius_base: f32,
    radius_rand: f32,
    /// Outer halo radius rule: `radius + 0.25` (false) or `radius * 2`
    /// (true, arrow vulcan).
    outer_double: bool,
    /// Streak length. The original game uses a fixed 20 for 329 and the live
    /// target distance for 388/393; we keep every variant at a fixed reach so
    /// the slash stays the same size whatever the actual target distance — only
    /// the *heading* tracks the target.
    fixed_reach: f32,
    /// Random reach slack added on top of the base reach (length jitter).
    reach_rand: f32,
    /// Spawn delay magnitude: the streak starts at `-(rand(delay_rand) + delay_base)`.
    delay_base: i32,
    delay_rand: u32,
    /// Per-frame alpha step up (frames ≤ 5) and down (after).
    fade_in: f32,
    fade_out: f32,
}

impl TripleAttackParams {
    /// Wall-clock end = longest streak's delay + ramp + fade-out, plus the
    /// launch-wave stagger. The parent emitter's duration is far shorter.
    pub const fn total_duration_ms(&self) -> u32 {
        let peak = RAMP_FRAMES as f32 * self.fade_in;
        let fade_frames = peak / self.fade_out;
        let max_delay = self.delay_base + self.delay_rand as i32;
        let frames = max_delay as f32 + self.waves as f32 + RAMP_FRAMES as f32 + fade_frames;
        (frames / FRAMES_PER_SECOND * 1000.0) as u32 + 100
    }
}

/// The reach/radius/lift literals map ~1:1 onto our world
/// (`fixed_reach = 20` ≈ our short-melee reach), so the family ports unscaled.
const WORLD_SCALE: f32 = 1.0;

const YELLOW: [f32; 3] = [1.0, 1.0, 55.0 / 255.0];
const PALE_YELLOW: [f32; 3] = [1.0, 1.0, 200.0 / 255.0];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];
const MAGENTA: [f32; 3] = [1.0, 89.0 / 255.0, 182.0 / 255.0];

pub const TRIPLEATTACK: TripleAttackParams = TripleAttackParams {
    texture: "cloud11.tga",
    inner_color: YELLOW,
    outer_color: PALE_YELLOW,
    waves: 2,
    offset_xz: 3.0,
    lift_base: 8.0,
    lift_rand: 4.0,
    radius_base: 0.25,
    radius_rand: 0.0,
    outer_double: false,
    fixed_reach: 20.0,
    reach_rand: 20.0,
    delay_base: 0,
    delay_rand: 20,
    fade_in: 8.0,
    fade_out: 4.0,
};

pub const TRIPLEATTACK2: TripleAttackParams = TripleAttackParams {
    texture: "alpha_center.tga",
    inner_color: WHITE,
    outer_color: WHITE,
    waves: 4,
    offset_xz: 4.0,
    lift_base: 8.0,
    lift_rand: 4.0,
    radius_base: 0.05,
    radius_rand: 0.0,
    outer_double: false,
    fixed_reach: 20.0,
    reach_rand: 20.0,
    delay_base: 45,
    delay_rand: 20,
    fade_in: 2.0,
    fade_out: 1.0,
};

pub const TRIPLEATTACK3: TripleAttackParams = TripleAttackParams {
    texture: "alpha_center.tga",
    inner_color: MAGENTA,
    outer_color: MAGENTA,
    waves: 9,
    offset_xz: 4.0,
    lift_base: 4.0,
    lift_rand: 6.0,
    radius_base: 0.02,
    radius_rand: 0.088,
    outer_double: true,
    fixed_reach: 20.0,
    reach_rand: 15.0,
    delay_base: 30,
    delay_rand: 110,
    fade_in: 8.0,
    fade_out: 4.0,
};

/// Deterministic LCG so an effect spawned twice at the same spot looks the same
/// (mirrors `soul_breaker.rs`).
struct Rng(u32);
impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// Uniform integer in `0..n`.
    fn random(&mut self, n: u32) -> u32 {
        self.next_u32() % n.max(1)
    }
    /// `random(100) * scale` — a `0..100*scale` float draw.
    fn frand(&mut self, scale: f32) -> f32 {
        self.random(100) as f32 * scale
    }
}

struct Streak {
    /// Frame counter; starts negative so the streak is delayed before it appears.
    process: i32,
    base: [f32; 3],
    tip: [f32; 3],
    inner_w: f32,
    outer_w: f32,
    /// One full texture map over the streak's length.
    uv_along: f32,
    alpha: f32,
    alive: bool,
}

pub struct TripleAttackEffect {
    params: TripleAttackParams,
    inner_color: [f32; 4],
    outer_color: [f32; 4],
    texture: &'static str,
    streaks: Vec<Streak>,
    frame_accum: f32,
}

impl TripleAttackEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: TripleAttackParams) -> Self {
        let seed = from[0].to_bits() ^ to[2].to_bits() ^ 0x73_1A_44_9C;
        let mut rng = Rng::from_seed(seed);
        // Only the heading tracks the target; the streak length is fixed so the
        // slash stays the same size regardless of how far the target is.
        let heading = heading_of(from, to);
        let (dx, dz) = (heading.cos(), heading.sin());

        let mut streaks = Vec::with_capacity(params.waves as usize * STREAKS_PER_WAVE);
        for wave in 0..params.waves {
            for _ in 0..STREAKS_PER_WAVE {
                // Base = caster pos + jitter, then pulled back along the heading
                // so the streaks stagger along the line.
                let ox = rng.frand(params.offset_xz * 0.02) - params.offset_xz;
                let oz = rng.frand(params.offset_xz * 0.02) - params.offset_xz;
                let pullback = rng.frand(0.15);
                let lift = params.lift_base + rng.frand(params.lift_rand * 0.01);
                let radius = params.radius_base
                    + if params.radius_rand > 0.0 {
                        rng.frand(params.radius_rand / 100.0)
                    } else {
                        0.0
                    };
                let reach = params.fixed_reach + rng.frand(params.reach_rand / 100.0);

                let bx = from[0] + ox - pullback * dx;
                let bz = from[2] + oz - pullback * dz;
                let by = from[1] - lift; // native RO: -Y is up
                let base = [bx, by, bz];
                let tip = [bx + reach * dx, by, bz + reach * dz];

                let outer_w = if params.outer_double {
                    radius * 2.0
                } else {
                    radius + 0.25
                };
                let delay = params.delay_base + rng.random(params.delay_rand) as i32 + wave as i32;

                streaks.push(Streak {
                    process: -delay,
                    base: scale(base, from, WORLD_SCALE),
                    tip: scale(tip, from, WORLD_SCALE),
                    inner_w: radius * WORLD_SCALE,
                    outer_w: outer_w * WORLD_SCALE,
                    uv_along: 1.0 / (reach * WORLD_SCALE).max(0.001),
                    alpha: 0.0,
                    alive: true,
                });
            }
        }

        Self {
            inner_color: rgba(params.inner_color, 0.0),
            outer_color: rgba(params.outer_color, 0.0),
            texture: params.texture,
            params,
            streaks,
            frame_accum: 0.0,
        }
    }

    fn step_frame(&mut self) {
        for s in &mut self.streaks {
            if !s.alive {
                continue;
            }
            s.process += 1;
            if s.process <= 0 {
                continue;
            }
            if s.process <= RAMP_FRAMES {
                s.alpha += self.params.fade_in;
            } else {
                s.alpha -= self.params.fade_out;
                if s.alpha <= 0.0 {
                    s.alpha = 0.0;
                    s.alive = false;
                }
            }
        }
        self.streaks.retain(|s| s.alive);
    }
}

/// Uniformly scale a point about the caster anchor, preserving every ratio.
fn scale(p: [f32; 3], anchor: [f32; 3], k: f32) -> [f32; 3] {
    [
        anchor[0] + (p[0] - anchor[0]) * k,
        anchor[1] + (p[1] - anchor[1]) * k,
        anchor[2] + (p[2] - anchor[2]) * k,
    ]
}

fn rgba(rgb: [f32; 3], a: f32) -> [f32; 4] {
    [rgb[0], rgb[1], rgb[2], a]
}

fn heading_of(from: [f32; 3], to: [f32; 3]) -> f32 {
    let dx = to[0] - from[0];
    let dz = to[2] - from[2];
    if dx == 0.0 && dz == 0.0 {
        0.0
    } else {
        dz.atan2(dx) // cos(h) = dx, sin(h) = dz
    }
}

impl Effect for TripleAttackEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        if self.streaks.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.streaks {
            if s.alpha <= 0.0 || s.process <= 0 {
                continue;
            }
            let a = (s.alpha / 255.0).clamp(0.0, 1.0);
            let points = vec![s.base, s.tip];
            // Wider pale halo first, then the bright core on top (additive — the
            // overlap reads as the glowing centre line).
            for (half_width, mut color) in [(s.outer_w, self.outer_color), (s.inner_w, self.inner_color)] {
                color[3] = a;
                out.push(EffectPrimitiveDraw::LineStrip {
                    points: points.clone(),
                    uv_along: s.uv_along,
                    u_along: false,
                    half_width,
                    texture: self.texture,
                    color,
                    colors: None,
                    blend: BlendKind::Additive,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None }
    }

    fn tick(e: &mut TripleAttackEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&ctx());
        }
        st
    }

    fn strips(e: &TripleAttackEffect) -> Vec<(Vec<[f32; 3]>, f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::LineStrip { points, half_width, color, .. } => {
                    (points.clone(), *half_width, color[3])
                }
                other => panic!("expected LineStrip, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn streaks_point_from_caster_toward_the_target() {
        let from = [0.0, 0.0, 0.0];
        let to = [0.0, 0.0, 22.0]; // heading +Z
        let mut e = TripleAttackEffect::new(from, to, TRIPLEATTACK);
        // Past the spawn delay (≤20) + ramp.
        tick(&mut e, 24);
        let drawn = strips(&e);
        assert!(!drawn.is_empty(), "streaks visible after the spawn delay");
        // Each streak's tip advances toward +Z relative to its base, and two
        // ribbon layers (inner + outer) come out.
        for (points, _, _) in &drawn {
            assert_eq!(points.len(), 2, "two-point ribbon");
            assert!(points[1][2] > points[0][2], "tip is further along +Z than base");
        }
    }

    #[test]
    fn arrow_vulcan_emits_far_more_streaks_than_melee() {
        let from = [0.0, 0.0, 0.0];
        let to = [0.0, 0.0, 22.0];
        let melee = TripleAttackEffect::new(from, to, TRIPLEATTACK);
        let vulcan = TripleAttackEffect::new(from, to, TRIPLEATTACK3);
        // waves * 4: 329 → 8, 393 → 36.
        assert!(vulcan.streaks.len() > melee.streaks.len() * 3);
    }

    #[test]
    fn alpha_fades_in_then_out_and_self_terminates() {
        let mut e = TripleAttackEffect::new([0.0; 3], [0.0, 0.0, 22.0], TRIPLEATTACK);
        tick(&mut e, 23); // just into the ramp for the earliest streaks
        let a_in: f32 = strips(&e).iter().map(|(_, _, a)| *a).sum();
        tick(&mut e, 12);
        let a_out: f32 = strips(&e).iter().map(|(_, _, a)| *a).sum();
        assert!(a_in > 0.0, "faded in: {a_in}");
        assert!(a_out < a_in, "faded out: {a_in} -> {a_out}");
        assert_eq!(tick(&mut e, 200), EffectStatus::Dead);
    }
}
