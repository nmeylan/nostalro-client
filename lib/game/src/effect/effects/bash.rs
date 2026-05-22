//! `EF_BASH` — Swordman Bash skill impact.
//!
//!
//! Pure screen-space recipe in the original (2D-circle + 20 ×
//! 2D-flash):
//!
//!   * Frame 0 — a filled 2D halo disc (`alpha_down.tga`, radius 100 px,
//!     max_alpha 170/255, fade-in 6 frames, fade-out 10 frames at the
//!     end of a 40-frame lifetime).
//!   * Frame 0 — 20 radial spikes (`alpha_center.tga`). Each spawns at
//!     a random longitude with a per-spike random
//!     texture rotation speed (1..7°/frame, decelerating), random initial
//!     `height_size` (20..60 px) growing at `height_speed` (2..5/frame),
//!     `arcAngle` jitter (0.5..3). The spikes rotate around the centre
//!     and elongate over their 40-frame lifetime, with the same alpha
//!     fade as the disc.
//!
//! Halo: two stacked `BillboardDisc` primitives textured with
//! `alpha_down.tga`. `BillboardDisc` projects the anchor to screen and
//! builds a triangle fan in screen pixels with polar UV mapping (V=1
//! at the centre, V=0 at the rim) — exactly the mapping the original
//! game's `Render2DCircle` uses on the same texture, so the disc reads
//! as a radial alpha gradient (opaque centre → transparent rim). Two
//! discs (a tight bright core + a wider softer rim) reproduce the
//! layered "outer lighter than centre" silhouette visible in the gif.
//! Spike rays are camera-facing billboards textured with
//! `alpha_center.tga`. Both layers use alpha blending (matching
//! alpha-blend flag on the 2D circle / 2D flash render). All spikes
//! rotate the same direction with per-spike speed (no sign
//! randomization — the original samples the longitudinal speed as an unsigned
//! magnitude).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const HALO_TEXTURE: &str = "alpha_down.tga";
pub const SPIKE_TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[HALO_TEXTURE, SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: f32 = 40.0;

// Halo billboard — the central radial glow at the impact point. Two
// stacked alpha-down discs: a tight bright core and a wider softer
// outer halo, matching the layered "bright centre, faded surround"
// silhouette in the original game's gif (where the outer ring reads
// as a separate, dimmer layer rather than a single uniform gradient).
// Sits one character above the ground so it hovers at the target's
// centre of mass.
// Two stacked filled discs: a smaller bright core and a wider softer
// outer halo. `*_RADIUS` is the disc's outer radius in world units
// (the original game's radius 100 is screen pixels — we scale to
// world units that read as a roughly torso-sized glow at the default
// camera).
// Inner halo sits inside the spikes' starting length so it reads as a
// bright core under the burst. Outer halo extends past the average
// spike length so it shows around them as a soft surrounding glow.
const HALO_INNER_RADIUS: f32 = 3.5;
const HALO_INNER_MAX_ALPHA: f32 = 220.0 / 255.0;
const HALO_OUTER_RADIUS: f32 = 9.0;
const HALO_OUTER_MAX_ALPHA: f32 = 130.0 / 255.0;
// Warm tint matching the original game's golden halo (vs. pure white).
const HALO_TINT: [f32; 3] = [1.0, 0.95, 0.75];
const HALO_FADE_IN_FRAMES: f32 = 6.0;
const HALO_FADE_OUT_AT: f32 = DURATION_FRAMES - 10.0;
const HALO_HEIGHT_OFFSET: f32 = -5.0;

// 20 radial spikes (a screen-space flash).
const SPIKE_COUNT: usize = 20;
const SPIKE_THICKNESS: f32 = 0.5;
// Original literal height = random(40) + 20 px → 2..6 wu;
// `height_speed = random(30)+20` /10 → 2..5 px/frame ≈ 0.2..0.5 wu/frame.
// We've measured against the gif and scaled the literal-derived values
// so spikes span roughly the outer-halo diameter by mid-life.
const SPIKE_LENGTH_INIT_MIN: f32 = 2.8;
const SPIKE_LENGTH_INIT_MAX: f32 = 5.6;
const SPIKE_GROWTH_MIN: f32 = 0.28;
const SPIKE_GROWTH_MAX: f32 = 0.7;
const SPIKE_MAX_ALPHA: f32 = 200.0 / 255.0;
const SPIKE_FADE_IN_FRAMES: f32 = 10.0;
const SPIKE_FADE_OUT_AT: f32 = DURATION_FRAMES - DURATION_FRAMES / 3.0;
// rotation speed = (random(60)+10)/10 deg/frame → 1..7°/frame; decel
// brings it close to 0 by end of lifetime. Stored per-spike.
const SPIKE_ANGULAR_SPEED_MIN_DEG: f32 = 1.0;
const SPIKE_ANGULAR_SPEED_MAX_DEG: f32 = 7.0;

pub const TOTAL_DURATION_MS: u32 =
    (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

fn fade_in_out(frame: f32, peak: f32, fade_in: f32, fade_out_at: f32, total: f32) -> f32 {
    let rise = (frame / fade_in).clamp(0.0, 1.0);
    let fall = if frame < fade_out_at {
        1.0
    } else {
        let span = (total - fade_out_at).max(1e-3);
        (1.0 - (frame - fade_out_at) / span).clamp(0.0, 1.0)
    };
    peak * rise * fall
}

#[derive(Clone, Copy, Debug)]
struct Spike {
    /// Initial longitude in radians; combined with the per-frame angular
    /// drift to give the current outward direction.
    initial_longitude_rad: f32,
    /// rotation speed in radians/frame at spawn (decelerated by
    /// integrating `long_accel = -long_speed/duration/1.5`).
    angular_speed_rad_per_frame: f32,
    length_init: f32,
    growth_per_frame: f32,
}

impl Spike {
    fn longitude(&self, frame: f32) -> f32 {
        // Integrate v(N) = v0 + accel * N where accel = -v0/duration/1.5.
        // Position = v0*N + accel*N*(N+1)/2.
        let accel = -self.angular_speed_rad_per_frame / DURATION_FRAMES / 1.5;
        let travel = self.angular_speed_rad_per_frame * frame
            + accel * frame * (frame + 1.0) / 2.0;
        self.initial_longitude_rad + travel
    }

    fn length(&self, frame: f32) -> f32 {
        self.length_init + self.growth_per_frame * frame
    }
}

pub struct BashEffect {
    world_pos: [f32; 3],
    spikes: Vec<Spike>,
    age_frames: f32,
}

impl BashEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut rng_state = 0x9E37_79B9
            ^ world_pos[0].to_bits()
            ^ world_pos[2].to_bits().rotate_left(13);
        let mut lcg = || {
            rng_state = rng_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            (rng_state >> 8) as f32 / ((1u32 << 24) as f32)
        };

        let mut spikes = Vec::with_capacity(SPIKE_COUNT);
        for _ in 0..SPIKE_COUNT {
            let longitude_deg = lcg() * 360.0;
            let angular_deg = SPIKE_ANGULAR_SPEED_MIN_DEG
                + lcg() * (SPIKE_ANGULAR_SPEED_MAX_DEG - SPIKE_ANGULAR_SPEED_MIN_DEG);
            let length_init = SPIKE_LENGTH_INIT_MIN
                + lcg() * (SPIKE_LENGTH_INIT_MAX - SPIKE_LENGTH_INIT_MIN);
            let growth = SPIKE_GROWTH_MIN + lcg() * (SPIKE_GROWTH_MAX - SPIKE_GROWTH_MIN);
            spikes.push(Spike {
                initial_longitude_rad: longitude_deg.to_radians(),
                // Original rotation-speed > 0 advances longitude in
                // its screen convention (Y flipped) — which projects to
                // a clockwise rotation in our standard CCW-positive
                // screen-space `rotation` field. Negate to match.
                angular_speed_rad_per_frame: -angular_deg.to_radians(),
                length_init,
                growth_per_frame: growth,
            });
        }

        Self { world_pos, spikes, age_frames: 0.0 }
    }
}

impl Effect for BashEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let halo_pos = [
            self.world_pos[0],
            self.world_pos[1] + HALO_HEIGHT_OFFSET,
            self.world_pos[2],
        ];
        let push_halo = |out: &mut EffectDrawList, radius: f32, max_alpha: f32| {
            let a = fade_in_out(
                self.age_frames,
                max_alpha,
                HALO_FADE_IN_FRAMES,
                HALO_FADE_OUT_AT,
                DURATION_FRAMES,
            );
            if a > 0.0 {
                out.push(EffectPrimitiveDraw::BillboardDisc {
                    pos: halo_pos,
                    radius,
                    segments: 32,
                    uv_repeat: 1.0,
                    texture: HALO_TEXTURE,
                    color: [HALO_TINT[0], HALO_TINT[1], HALO_TINT[2], a],
                    blend: BlendKind::Alpha,
                });
            }
        };
        push_halo(out, HALO_OUTER_RADIUS, HALO_OUTER_MAX_ALPHA);
        push_halo(out, HALO_INNER_RADIUS, HALO_INNER_MAX_ALPHA);

        let spike_alpha = fade_in_out(
            self.age_frames,
            SPIKE_MAX_ALPHA,
            SPIKE_FADE_IN_FRAMES,
            SPIKE_FADE_OUT_AT,
            DURATION_FRAMES,
        );
        if spike_alpha <= 0.0 {
            return;
        }

        for spike in &self.spikes {
            let longitude = spike.longitude(self.age_frames);
            let length = spike.length(self.age_frames);
            // The billboard quad straddles the entity centre, with
            // `alpha_center.tga` (alpha-peaks-in-the-middle) mapped so
            // the bright row crosses the anchor. With 20 spikes spread
            // randomly around 360° this reads as a symmetric burst —
            // the original game draws a triangle apex-at-centre per
            // spike, but the aggregate silhouette of 20 random
            // longitudes matches our centred-quad approximation. Alpha
            // blend (the alpha blend flag) keeps the rays soft instead
            // of saturating to white.
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0],
                    self.world_pos[1] + HALO_HEIGHT_OFFSET,
                    self.world_pos[2],
                ],
                size: [SPIKE_THICKNESS, length * 2.0],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: longitude,
                texture: SPIKE_TEXTURE,
                color: [1.0, 1.0, 1.0, spike_alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut BashEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn halo_discs_plus_twenty_spike_billboards_at_spawn() {
        // Sociable: 2 halo GroundDiscs (layered inner/outer for the
        // bright-centre / soft-rim silhouette) + 20 spike Billboards
        // all anchored at the same centre. Confirms the spike count
        // matches the original game's `loopCnt = 20` and that every
        // spike is positioned at the world anchor.
        let mut e = BashEffect::new([0.0; 3]);
        step_frames(&mut e, 8);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());

        let halos: usize = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardDisc { texture, .. } if *texture == HALO_TEXTURE))
            .count();
        let spikes: Vec<&EffectPrimitiveDraw> = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { texture, .. } if *texture == SPIKE_TEXTURE))
            .collect();
        assert_eq!(halos, 2);
        assert_eq!(spikes.len(), SPIKE_COUNT);

        for prim in &spikes {
            if let EffectPrimitiveDraw::Billboard { pos, .. } = prim {
                assert!(
                    pos[0].abs() < 1e-3 && pos[2].abs() < 1e-3,
                    "spike anchored at entity centre, got {:?}",
                    pos,
                );
            }
        }
    }

    #[test]
    fn spikes_elongate_over_lifetime() {
        // Sociable: take the first spike's length at two times and
        // confirm it grows.
        let e = BashEffect::new([0.0; 3]);
        let spike = e.spikes[0];
        let l_early = spike.length(2.0);
        let l_late = spike.length(20.0);
        assert!(l_late > l_early, "spike grows: {l_early} → {l_late}");
    }

    #[test]
    fn dies_after_duration() {
        let mut e = BashEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + 2) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
