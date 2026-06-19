//! `Cloud(map)` family — ambient drifting cloud quads (ids 229,
//! 230, 233, 515, 516, 592, 697, 698).
//!
//! The original game scatters a field of large camera-facing cloud billboards
//! across a 300×300 area around the player and lets them drift, breathe in size,
//! fade in, hold, then fade out and respawn at a fresh spot — a persistent map
//! atmosphere. Each `Cloud(map)` call spawns four
//! quads; the dispatch fires it 40–80× per id, so one of our effects owns
//! `calls × 4` clouds. The `map` byte selects everything that varies:
//!
//! | id  | name   | map | texture set      | tint          | elevation | spread   | count |
//! |-----|--------|-----|------------------|---------------|-----------|----------|-------|
//! | 229 | Cloud  | 0   | cloud4/1/2       | white         | −125 (sky)| centered | 160   |
//! | 230 | Cloud2 | 1   | cloud4/1/2       | white         | +40       | ring     | 240   |
//! | 233 | Cloud3 | 2   | cloud4/1/2       | white         | 0         | centered | 160   |
//! | 515 | Cloud4 | 3   | fog1/2/3         | (252,171,143) | ground−20 | centered | 320   |
//! | 516 | Cloud5 | 4   | cloud4/1/2       | white         | +40       | ring     | 320   |
//! | 592 | Cloud6 | 5   | cloud4/1/2       | (94,0,0)      | +20       | centered | 320   |
//! | 697 | Cloud7 | 7   | cloud4/1/2       | (0,0,0)       | +40       | ring     | 320   |
//! | 698 | Cloud8 | 8   | cloud4/1/2       | (255,180,180) | +40       | ring     | 320   |
//!
//! Every variant is a plain alpha-blended camera-facing
//! square of side ≈ `distance·√2` (`distance` 30–55), breathing ±5% on a slow
//! sine, tinted per the `map` byte. Each quad ramps its alpha
//! up over a per-map window (peak ≈ `rate · window`), holds until a per-quad
//! hold timer in `[300,500)`, fades out, then relocates and repeats.
//!
//! Native RO coords (−Y up): a negative elevation lifts the cloud above the
//! caster (the sky overcast), a positive one drops it below (sky-city / boss
//! maps where the player stands above the cloud deck).
//!
//! Two parts of the original depend on world context an effect doesn't carry, so
//! they are approximated (and only matter for two of the eight maps):
//!   * einbroch (map 3) samples the terrain height under each quad; we anchor at
//!     `caster.y − 20` instead.
//!   * map 0 only ramps alpha while the player is *above* the cloud deck; we
//!     always ramp so the overcast is visible wherever it is spawned.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
/// A camera-facing square whose corners sit at `distance` from centre has a
/// side of `distance·√2`.
const SQRT2: f32 = std::f32::consts::SQRT_2;

const CLOUD_TEX: [&str; 3] = ["cloud4.tga", "cloud1.tga", "cloud2.tga"];
const FOG_TEX: [&str; 3] = ["fog1.tga", "fog2.tga", "fog3.tga"];

pub const TEXTURES: &[&str] = &[
    "cloud4.tga", "cloud1.tga", "cloud2.tga", "fog1.tga", "fog2.tga", "fog3.tga",
];

/// How a quad's centre wanders each frame (per-map drift).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drift {
    /// `x,z += speed · sin(phase)` — isotropic meander.
    Isotropic(f32),
    /// airplane (map 4): steady `x += 0.20·|sin|` wind plus a `z` wobble.
    Airplane,
}

#[derive(Clone, Copy, Debug)]
pub struct CloudParams {
    pub textures: [&'static str; 3],
    /// RGB tint (0..1) selected by the `map` byte.
    pub tint: [f32; 3],
    /// Base elevation offset from the caster (native −Y up). einbroch resolves
    /// it against `caster.y` (see [`use_ground`]).
    pub elevation: f32,
    /// einbroch: anchor at `caster.y + elevation` (a terrain approximation)
    /// rather than treating `elevation` as a sky offset. Visual only.
    pub use_ground: bool,
    /// `true`: scatter in a ±150 box; `false`: a 25..225 signed ring (the
    /// sky-city maps push clouds away from the centre).
    pub centered: bool,
    /// Quad size base / random span (`distance = base + rand·hash`).
    pub size_base: f32,
    pub size_rand: f32,
    pub drift: Drift,
    /// Alpha gained per frame while ramping, and the ramp window in frames.
    /// Peak alpha (out of 255) is `alpha_rate · ramp_frames`.
    pub alpha_rate: f32,
    pub ramp_frames: f32,
    /// `calls × 4` — total quads this effect owns.
    pub count: u32,
}

const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

/// `EF_CLOUD` → `Cloud(0)` ×40. Sky overcast (mjolnir / gef_fil07).
pub const CLOUD: CloudParams = CloudParams {
    textures: CLOUD_TEX,
    tint: WHITE,
    elevation: -125.0,
    use_ground: false,
    centered: true,
    size_base: 30.0,
    size_rand: 20.0,
    drift: Drift::Isotropic(0.05),
    alpha_rate: 2.0,
    ramp_frames: 80.0,
    count: 160,
};
/// `EF_CLOUD2` → `Cloud(1)` ×60. yuno field clouds.
pub const CLOUD2: CloudParams =
    CloudParams { elevation: 40.0, centered: false, alpha_rate: 3.0, count: 240, ..CLOUD };
/// `EF_CLOUD3` → `Cloud(2)` ×40.
pub const CLOUD3: CloudParams = CloudParams { elevation: 0.0, ..CLOUD };
/// `EF_CLOUD4` → `Cloud(3)` ×80. einbroch warm ground fog.
pub const CLOUD4: CloudParams = CloudParams {
    textures: FOG_TEX,
    tint: [252.0 / 255.0, 171.0 / 255.0, 143.0 / 255.0],
    elevation: -20.0,
    use_ground: true,
    size_base: 35.0,
    size_rand: 10.0,
    drift: Drift::Isotropic(0.015),
    alpha_rate: 1.0,
    ramp_frames: 170.0,
    count: 320,
    ..CLOUD
};
/// `EF_CLOUD5` → `Cloud(4)` ×80. airplane drift.
pub const CLOUD5: CloudParams = CloudParams {
    elevation: 40.0,
    centered: false,
    drift: Drift::Airplane,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};
/// `EF_CLOUD6` → `Cloud(5)` ×80. thana_boss dark red haze.
pub const CLOUD6: CloudParams = CloudParams {
    tint: [94.0 / 255.0, 0.0, 0.0],
    elevation: 20.0,
    drift: Drift::Isotropic(0.035),
    count: 320,
    ..CLOUD
};
/// `EF_CLOUD7` → `Cloud(7)` ×80. black tower clouds.
pub const CLOUD7: CloudParams = CloudParams {
    tint: [0.0, 0.0, 0.0],
    elevation: 40.0,
    centered: false,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};
/// `EF_CLOUD8` → `Cloud(8)` ×80. pink tower clouds.
pub const CLOUD8: CloudParams = CloudParams {
    tint: [1.0, 180.0 / 255.0, 180.0 / 255.0],
    elevation: 40.0,
    centered: false,
    alpha_rate: 3.0,
    count: 320,
    ..CLOUD
};

/// Alpha (out of 255) for a quad at frame `process`: linear ramp `0 → peak`
/// over `ramp_frames`, hold until `rot_start`, then linear fade `−1/frame`.
/// `peak = alpha_rate · ramp_frames`.
fn cloud_alpha(p: &CloudParams, process: f32, rot_start: f32) -> f32 {
    let peak = p.alpha_rate * p.ramp_frames;
    if process < p.ramp_frames {
        p.alpha_rate * process
    } else if process <= rot_start {
        peak
    } else {
        (peak - (process - rot_start)).max(0.0)
    }
}

fn hash01(i: u32, salt: u32) -> f32 {
    let x = i
        .wrapping_mul(2_654_435_761)
        .wrapping_add(salt.wrapping_mul(40_503))
        .wrapping_add(0x9E37_79B9);
    let x = x ^ (x >> 15);
    (x % 100_000) as f32 / 100_000.0
}

/// One cloud quad, position in world space (the original game anchors each
/// quad at spawn and never re-bases, so we keep absolute world coords too).
#[derive(Clone, Copy)]
struct Cloud {
    pos: [f32; 3],
    distance: f32,
    drift_phase: [f32; 2],
    drift_rate: [f32; 2],
    breath_phase: f32,
    process: f32,
    rot_start: f32,
    alpha: f32,
    generation: u32,
}

pub struct CloudEffect {
    world_pos: [f32; 3],
    params: CloudParams,
    clouds: Vec<Cloud>,
}

impl CloudEffect {
    pub fn new(world_pos: [f32; 3], params: CloudParams) -> Self {
        let clouds = (0..params.count).map(|i| spawn_cloud(i, 0, &params, world_pos)).collect();
        Self { world_pos, params, clouds }
    }

    fn step(&mut self, df: f32) {
        let peak = self.params.alpha_rate * self.params.ramp_frames;
        for (i, c) in self.clouds.iter_mut().enumerate() {
            c.process += df;
            // Relocate once the fade-out has run its full `peak` frames.
            if c.process >= c.rot_start + peak {
                *c = spawn_cloud(i as u32, c.generation + 1, &self.params, self.world_pos);
                continue;
            }
            c.alpha = cloud_alpha(&self.params, c.process, c.rot_start);
            match self.params.drift {
                Drift::Isotropic(s) => {
                    c.pos[0] += s * c.drift_phase[0].sin() * df;
                    c.pos[2] += s * c.drift_phase[1].sin() * df;
                }
                Drift::Airplane => {
                    c.pos[0] += 0.20 * c.drift_phase[0].sin().abs() * df;
                    c.pos[2] += 0.05 * c.drift_phase[1].sin() * df;
                }
            }
            c.drift_phase[0] += c.drift_rate[0] * df;
            c.drift_phase[1] += c.drift_rate[1] * df;
            c.breath_phase += df.to_radians();
        }
    }
}

/// Spawn (or respawn) a quad: pick a position in the map's spread, an elevation,
/// a size and per-quad wander/breathe phases. `generation` salts the hash so a
/// looping quad lands somewhere new each cycle.
fn spawn_cloud(i: u32, generation: u32, p: &CloudParams, world_pos: [f32; 3]) -> Cloud {
    let s = generation.wrapping_mul(11);
    let (dx, dz) = if p.centered {
        (hash01(i, s + 1) * 300.0 - 150.0, hash01(i, s + 2) * 300.0 - 150.0)
    } else {
        let sign = |h: f32| if h < 0.5 { -1.0 } else { 1.0 };
        (
            (hash01(i, s + 1) * 200.0 + 25.0) * sign(hash01(i, s + 8)),
            (hash01(i, s + 2) * 200.0 + 25.0) * sign(hash01(i, s + 9)),
        )
    };
    let y = world_pos[1] + p.elevation + if p.use_ground { -hash01(i, s + 3) * 5.0 } else { hash01(i, s + 3) * 10.0 };
    Cloud {
        pos: [world_pos[0] + dx, y, world_pos[2] + dz],
        distance: p.size_base + hash01(i, s + 4) * p.size_rand,
        drift_phase: [hash01(i, s + 5) * std::f32::consts::TAU, hash01(i, s + 6) * std::f32::consts::TAU],
        drift_rate: [0.3 + hash01(i, s + 10) * 0.5, 0.3 + hash01(i, s + 11) * 0.5],
        breath_phase: hash01(i, s + 7) * std::f32::consts::TAU,
        process: 0.0,
        rot_start: 300.0 + hash01(i, s + 12) * 200.0,
        alpha: 0.0,
        generation,
    }
}

impl Effect for CloudEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.step(ctx.delta * FRAMES_PER_SECOND);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.tint;
        for c in &self.clouds {
            if c.alpha <= 0.0 {
                continue;
            }
            // ±5% size breathing on a slow sine (`side += sin·distance·0.05`).
            let side = c.distance * (1.0 + 0.05 * c.breath_phase.sin()) * SQRT2;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: c.pos,
                size: [side, side],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: 0.0,
                texture: self.params.textures[(c.generation as usize).wrapping_add(c.distance as usize) % 3],
                color: [r, g, b, (c.alpha / 255.0).min(1.0)],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut CloudEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None })
    }

    fn draws(e: &CloudEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn ramps_in_to_an_alpha_blended_tinted_billboard_field() {
        // After the ramp window every quad is visible: alpha-blended camera-facing
        // billboards on the cloud textures, tinted per the variant, at the peak
        // alpha `rate · window` (≈160/255 here). None before the first frame.
        let mut e = CloudEffect::new([0.0, 0.0, 0.0], CLOUD3);
        assert!(draws(&e).is_empty(), "alpha starts at 0");
        step(&mut e, CLOUD3.ramp_frames);
        let prims = draws(&e);
        assert_eq!(prims.len(), CLOUD3.count as usize, "all quads visible at peak");
        let peak = CLOUD3.alpha_rate * CLOUD3.ramp_frames / 255.0;
        for p in &prims {
            let EffectPrimitiveDraw::Billboard { blend, texture, color, .. } = p else { unreachable!() };
            assert_eq!(*blend, BlendKind::Alpha);
            assert!(CLOUD_TEX.contains(texture));
            assert!((color[3] - peak).abs() < 0.05, "near peak alpha: {}", color[3]);
        }
    }

    #[test]
    fn variants_differ_in_texture_set_tint_and_count() {
        // einbroch fog: fog textures, warm tint, 320 quads. Black tower clouds:
        // zero tint. Both distinct from the plain white overcast.
        let mut fog = CloudEffect::new([0.0, 0.0, 0.0], CLOUD4);
        step(&mut fog, CLOUD4.ramp_frames);
        let fp = draws(&fog);
        assert_eq!(fp.len(), 320);
        let EffectPrimitiveDraw::Billboard { texture, color, .. } = &fp[0] else { unreachable!() };
        assert!(FOG_TEX.contains(texture), "fog textures");
        assert!(color[0] > color[2], "warm peach tint (r>b)");

        let mut black = CloudEffect::new([0.0, 0.0, 0.0], CLOUD7);
        step(&mut black, CLOUD7.ramp_frames);
        let EffectPrimitiveDraw::Billboard { color, .. } = &draws(&black)[0] else { unreachable!() };
        assert_eq!([color[0], color[1], color[2]], [0.0, 0.0, 0.0], "black tint");
    }

    #[test]
    fn quads_drift_breathe_and_persist_through_a_full_loop() {
        // The field never dies; quads drift, their size breathes, and after the
        // fade-out a relocating quad keeps the population bounded.
        let mut e = CloudEffect::new([0.0, 0.0, 0.0], CLOUD5); // airplane drift
        let pos0 = e.clouds[0].pos;
        let mut status = EffectStatus::Running;
        for _ in 0..600 {
            status = step(&mut e, 1.0);
        }
        assert_eq!(status, EffectStatus::Running, "persistent atmosphere");
        assert!(e.clouds[0].pos[0] != pos0[0], "airplane wind drifts +x");
        // Size breathes around the base side over the run.
        let mut e2 = CloudEffect::new([0.0, 0.0, 0.0], CLOUD3);
        step(&mut e2, CLOUD3.ramp_frames);
        let side_a = match &draws(&e2)[0] { EffectPrimitiveDraw::Billboard { size, .. } => size[0], _ => unreachable!() };
        step(&mut e2, 90.0);
        let side_b = match &draws(&e2)[0] { EffectPrimitiveDraw::Billboard { size, .. } => size[0], _ => unreachable!() };
        assert!((side_a - side_b).abs() > 1e-3, "size breathes over time");
    }
}
