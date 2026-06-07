//! Radial "slash blade" burst.
//!
//! `EF_KAIZEL` fires two sets of four blades (base azimuth + ec*90°) →
//! **eight blades** at azimuths `0,45,…,315`, all on `magic_blue.tga`. Each
//! blade is a thin radial sheet that flies outward from the caster, rising
//! slightly, fading in then out.
//!
//! Per blade we render **three adjacent slices** one degree apart: the middle
//! (`i==1`) at full alpha and full height, the outer two at `alpha/3` and
//! shorter, which feathers the edge into a soft taper. Each slice is a
//! `WorldQuad` from inner radius `height[0]` to outer radius
//! `height[0] + distance`, the bottom edge on the ground and the outer/inner
//! top edges lifted (native −Y up).
//!
//! State per frame (Kaizel): inner radius `+1`, blade length
//! `+distance_per_frame`, height grows toward its cap, alpha ramps up over the
//! first `alpha_rise_frames`, then once the inner radius passes 10 it fades out
//! and the blade dies. Alpha blended.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// `magic_blue.tga` (renderer prepends `data/texture/effect/`).
pub const TEXTURE: &str = "magic_blue.tga";
/// `ring_blue.tga` — the SuperAngel level-up ring flash uses the same
/// blade burst on this texture instead of `magic_blue`.
pub const RING_BLUE: &str = "ring_blue.tga";

/// Preload set (see [`crate::effect::effect_texture_paths`]).
pub const TEXTURES: &[&str] = &[TEXTURE, RING_BLUE];

const RADIUS_INIT: f32 = 1.0;
const RADIUS_PER_FRAME: f32 = 1.0;
const DISTANCE_INIT: f32 = 2.0;
const ALPHA_PER_FRAME: f32 = 5.0;
const ALPHA_FADE_PER_FRAME: f32 = 2.0;
/// Inner radius beyond which the blade starts fading out (`height[0] > 10`).
const FADE_START_RADIUS: f32 = 10.0;
const ALPHA_DIVISOR: f32 = 255.0;

const EMITTERS_PER_SET: usize = 4;
const SLICES: usize = 3;
const SLICE_STEP_DEG: f32 = 1.0;

/// World-unit scale. The raw blade literals belong to the engine-scaled
/// family (blades fly to ~30+ units raw); this hugs them to the caster,
/// calibrated against the original game's on-screen look. The
/// *ratios* (radius : length : height) are preserved; only absolute size scales.
const WORLD_SCALE: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct SlashParams {
    /// Base azimuth (degrees) of each four-blade set. Kaizel fires two sets —
    /// `0` and `45` — interleaving into an eight-pointed star.
    pub emitter_sets: &'static [f32],
    /// Blade length growth per frame (`distance += …`).
    pub distance_per_frame: f32,
    /// Initial `max_height` (vertical lift). Kaizel starts lifted (`1.5`);
    /// Stopeffect starts flat (`0.0`) and grows the lift in.
    pub max_height_init: f32,
    pub max_height_cap: f32,
    pub max_height_per_frame: f32,
    /// Frames over which alpha ramps in (+5/frame).
    pub alpha_rise_frames: f32,
    /// Blade texture (`magic_blue.tga` for Kaizel, `ring_blue.tga` for the
    /// SuperAngel ring flash).
    pub texture: &'static str,
}

impl SlashParams {
    fn peak_alpha(&self) -> f32 {
        ALPHA_PER_FRAME * self.alpha_rise_frames
    }
    fn fade_start_frame(&self) -> f32 {
        (FADE_START_RADIUS - RADIUS_INIT) / RADIUS_PER_FRAME
    }
    /// Frame at which a blade's alpha reaches zero and it dies.
    fn life_end_frame(&self) -> f32 {
        self.fade_start_frame() + self.peak_alpha() / ALPHA_FADE_PER_FRAME
    }
    pub fn total_duration_ms(&self) -> u32 {
        (self.life_end_frame() / FRAMES_PER_SECOND * 1000.0).ceil() as u32
    }
}

/// `EF_KAIZEL` — eight blue blades (sets at base 0° and 45°).
pub const KAIZEL: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 2.0,
    max_height_init: 1.5,
    max_height_cap: 4.0,
    max_height_per_frame: 0.4,
    alpha_rise_frames: 7.0,
    texture: TEXTURE,
};

/// The SuperAngel (Angel2/Angel3) level-up ring — same eight-blade burst
/// (flag1==0) as Kaizel, on `ring_blue.tga`. Spawned at frame 65.
pub const SUPERANGEL_RING: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 2.0,
    max_height_init: 1.5,
    max_height_cap: 4.0,
    max_height_per_frame: 0.4,
    alpha_rise_frames: 7.0,
    texture: RING_BLUE,
};

/// `EF_STOPEFFECT` — the same eight-blade star as Kaizel but the flag1==1
/// branch — blades start flat (`max_height` 0), grow a shorter lift (cap 2,
/// `+0.2`/frame), stretch slower (`distance +1`/frame) and ramp alpha over the
/// first 5 frames.
pub const STOPEFFECT: SlashParams = SlashParams {
    emitter_sets: &[0.0, 45.0],
    distance_per_frame: 1.0,
    max_height_init: 0.0,
    max_height_cap: 2.0,
    max_height_per_frame: 0.2,
    alpha_rise_frames: 5.0,
    texture: TEXTURE,
};

/// Wall-clock end of the Kaizel burst (last blade fully faded).
pub const TOTAL_DURATION_MS: u32 = 450;
/// Wall-clock end of the Stopeffect burst (`STOPEFFECT.total_duration_ms()`).
pub const STOPEFFECT_DURATION_MS: u32 = 359;

pub struct SlashEffect {
    params: SlashParams,
    world_pos: [f32; 3],
    age_frames: f32,
}

impl SlashEffect {
    pub fn new(world_pos: [f32; 3], params: SlashParams) -> Self {
        Self { params, world_pos, age_frames: 0.0 }
    }

    fn radius(&self) -> f32 {
        RADIUS_INIT + RADIUS_PER_FRAME * self.age_frames
    }
    fn distance(&self) -> f32 {
        DISTANCE_INIT + self.params.distance_per_frame * self.age_frames
    }
    fn max_height(&self) -> f32 {
        (self.params.max_height_init + self.params.max_height_per_frame * self.age_frames)
            .min(self.params.max_height_cap)
    }
    /// The pulsing alpha value (0..peak), driving the blade alpha.
    fn alpha_b(&self) -> f32 {
        let t = self.age_frames;
        let rise = (ALPHA_PER_FRAME * t).min(self.params.peak_alpha());
        let fade = if t > self.params.fade_start_frame() {
            ALPHA_FADE_PER_FRAME * (t - self.params.fade_start_frame())
        } else {
            0.0
        };
        (rise - fade).max(0.0)
    }
}

impl Effect for SlashEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= self.params.life_end_frame() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha_b = self.alpha_b();
        if alpha_b <= 0.0 {
            return;
        }
        let [wx, wy, wz] = self.world_pos;
        let r_in = self.radius() * WORLD_SCALE;
        let r_out = (self.radius() + self.distance()) * WORLD_SCALE;
        let mh = self.max_height() * WORLD_SCALE;

        for &base in self.params.emitter_sets {
            for ec in 0..EMITTERS_PER_SET {
                let rot_start = base + ec as f32 * 90.0;
                for i in 0..SLICES {
                    let angle = (rot_start - SLICE_STEP_DEG + i as f32 * SLICE_STEP_DEG).to_radians();
                    let (sn, cs) = angle.sin_cos();
                    let mid = i == 1;
                    // Top-edge lift (native −Y up): the middle slice is tallest.
                    let top_inner = if mid { -mh * 0.2 } else { -mh * 0.1 };
                    let top_outer = if mid { -mh } else { -mh * 0.4 };
                    let alpha = if mid { alpha_b } else { alpha_b / 3.0 } / ALPHA_DIVISOR;

                    let inner_bottom = [wx + cs * r_in, wy, wz + sn * r_in];
                    let inner_top = [wx + cs * r_in, wy + top_inner, wz + sn * r_in];
                    let outer_bottom = [wx + cs * r_out, wy, wz + sn * r_out];
                    let outer_top = [wx + cs * r_out, wy + top_outer, wz + sn * r_out];

                    out.push(EffectPrimitiveDraw::WorldQuad {
                        // Corner order: inner-bottom, outer-bottom, outer-top,
                        // inner-top.
                        corners: [inner_bottom, outer_bottom, outer_top, inner_top],
                        uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                        texture: self.params.texture,
                        color: [1.0, 1.0, 1.0, alpha],
                        // `magic_blue.tga` is a bright spike glow on black; the
                        // reference shows it glowing additively (additive looks
                        // right for this dark-keyed texture, not alpha blend).
                        blend: BlendKind::Additive,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut SlashEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None })
    }

    fn draws(e: &SlashEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn quad(p: &EffectPrimitiveDraw) -> ([[f32; 3]; 4], [f32; 4]) {
        match p {
            EffectPrimitiveDraw::WorldQuad { corners, color, texture, blend, .. } => {
                assert_eq!(*texture, TEXTURE);
                assert_eq!(*blend, BlendKind::Additive);
                (*corners, *color)
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn eight_blades_three_slices_each() {
        // Kaizel = two sets × four emitters × three slices = 48 quads, once the
        // alpha has ramped up past zero.
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 2.0);
        assert_eq!(draws(&e).len(), 2 * EMITTERS_PER_SET * SLICES);
    }

    #[test]
    fn blade_grows_outward_and_rises() {
        // Sociable: radius + length + height all integrate forward, so a later
        // frame's outer corner is further from the caster than an earlier one.
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 2.0);
        let early = quad(&draws(&e)[1]).0; // middle slice of first blade
        let early_out = (early[1][0].powi(2) + early[1][2].powi(2)).sqrt();
        step(&mut e, 6.0);
        let late = quad(&draws(&e)[1]).0;
        let late_out = (late[1][0].powi(2) + late[1][2].powi(2)).sqrt();
        assert!(late_out > early_out, "blade flies outward: {early_out} -> {late_out}");
        // Middle slice's outer top is lifted above its outer bottom (native −Y up).
        assert!(late[2][1] < late[1][1], "outer edge rises");
    }

    #[test]
    fn alpha_ramps_in_then_out_and_effect_dies() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 1.0);
        let a_early = quad(&draws(&e)[1]).1[3];
        step(&mut e, 6.0); // near peak (rise caps at frame 7)
        let a_peak = quad(&draws(&e)[1]).1[3];
        assert!(a_peak > a_early, "fades in: {a_early} -> {a_peak}");

        // Run to the end: the burst self-terminates once every blade fades out.
        let mut status = EffectStatus::Running;
        for _ in 0..60 {
            status = step(&mut e, 1.0);
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn stopeffect_starts_flat_and_grows_lift() {
        // Stopeffect blades start with no vertical lift (max_height 0) and grow
        // it in, unlike Kaizel which starts lifted. Same eight-blade star.
        let mut e = SlashEffect::new([0.0; 3], STOPEFFECT);
        step(&mut e, 1.0);
        let early = quad(&draws(&e)[1]).0; // middle slice, outer-top vs outer-bottom
        let early_lift = (early[1][1] - early[2][1]).abs();
        step(&mut e, 6.0);
        let late = quad(&draws(&e)[1]).0;
        let late_lift = (late[1][1] - late[2][1]).abs();
        assert!(late_lift > early_lift, "lift grows in: {early_lift} -> {late_lift}");
        assert_eq!(draws(&e).len(), 2 * EMITTERS_PER_SET * SLICES);
    }

    #[test]
    fn stopeffect_duration_const_matches_computed() {
        assert_eq!(STOPEFFECT.total_duration_ms(), STOPEFFECT_DURATION_MS);
    }

    #[test]
    fn middle_slice_brighter_than_outer_slices() {
        let mut e = SlashEffect::new([0.0; 3], KAIZEL);
        step(&mut e, 4.0);
        let prims = draws(&e);
        let outer0 = quad(&prims[0]).1[3];
        let middle = quad(&prims[1]).1[3];
        let outer2 = quad(&prims[2]).1[3];
        assert!(middle > outer0 && middle > outer2, "middle slice is the bright one");
    }
}
