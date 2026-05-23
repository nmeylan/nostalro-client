//! Shared radial spike-burst — N flash rays
//! (`alpha_center.tga` rays radiating from a centre, rotating slowly and
//! elongating over their lifetime).
//!
//! Bash, HasteUp and Flasher all use this same recipe — 20 rays, each with a
//! random longitude that decelerates and a length that grows over time:
//!
//! ```text
//! for i in 0..20:
//!     ray            = new flash ray
//!     ray.duration   = D
//!     ray.longitude  = random(360)
//!     ray.long_speed = (random(60) + 10) / 10   // 1..7 deg/frame
//!     ray.long_accel = -(long_speed/D) / 1.5    // decelerates
//!     ray.length     = ...                      // length spawn
//!     ray.growth     = ...                      // growth/frame
//!     // optional change-point — HasteUp swaps growth
//!     // halfway through (frame 40 of 80).
//!     // (each ray is then drawn as one billboard ray.)
//! ```
//!
//! Each ray is emitted as a [`Billboard`] textured with `alpha_center.tga`.
//! The billboard quad straddles the entity centre so the alpha-peaks-in-the-
//! middle texture maps to a ray that radiates outward in both directions —
//! 20 random longitudes spread around 360° aggregate to a symmetric burst.
//!
//! [`Billboard`]: super::super::draw::EffectPrimitiveDraw::Billboard

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw};

pub const SPIKE_TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[SPIKE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

/// Parameter set for one spike-burst variant. All fields are in
/// frame-rate-pinned units (60 fps) so the recipe maps 1:1 onto the
/// original flash-ray numbers.
#[derive(Clone, Copy, Debug)]
pub struct SpikeBurstParams {
    pub count: usize,
    pub duration_frames: f32,
    /// Angular-speed random range in **degrees/frame**.
    pub angular_speed_deg_range: (f32, f32),
    /// Random spawn-length range in world units.
    pub length_init_range: (f32, f32),
    /// Random growth range in world units / frame.
    pub growth_range: (f32, f32),
    /// Optional growth change — at frame
    /// `change_at_frame`, the growth speed switches to a value sampled
    /// from `change_growth_range`. Used by HasteUp (frame 40, slower).
    pub change_growth: Option<ChangeGrowth>,
    pub thickness: f32,
    /// Peak alpha (0..1).
    pub max_alpha: f32,
    /// Linear fade-in window (frames). Alpha step = max_alpha / N.
    pub fade_in_frames: f32,
    /// Frame at which the late-life fade-out starts. The original begins it
    /// at `duration - duration/3`, i.e. `2/3 * duration`.
    pub fade_out_start_frame: f32,
    /// Y offset added to the world anchor (native RO — negative Y = up).
    /// The original lifts it ~5 wu above ground.
    pub height_offset: f32,
    pub texture: &'static str,
    pub color_tint: [f32; 3],
    pub blend: BlendKind,
}

#[derive(Clone, Copy, Debug)]
pub struct ChangeGrowth {
    pub at_frame: f32,
    pub growth_range: (f32, f32),
}

impl SpikeBurstParams {
    pub const fn default_fade_out_start(duration_frames: f32) -> f32 {
        duration_frames - duration_frames / 3.0
    }
}

#[derive(Clone, Copy, Debug)]
struct Spike {
    initial_longitude_rad: f32,
    angular_speed_rad_per_frame: f32,
    length_init: f32,
    growth_per_frame: f32,
    /// Pre-sampled change-phase growth (used after `change_at_frame`).
    /// `None` when `params.change_growth` is `None`.
    change_growth_per_frame: Option<f32>,
}

impl Spike {
    fn longitude(&self, frame: f32, duration_frames: f32) -> f32 {
        // Integrate v(N) = v0 + accel * N with accel = -v0/duration/1.5.
        // Position = v0*N + accel*N*(N+1)/2.
        let accel = -self.angular_speed_rad_per_frame / duration_frames / 1.5;
        let travel =
            self.angular_speed_rad_per_frame * frame + accel * frame * (frame + 1.0) / 2.0;
        self.initial_longitude_rad + travel
    }

    fn length(&self, frame: f32, change: Option<ChangeGrowth>) -> f32 {
        match (change, self.change_growth_per_frame) {
            (Some(c), Some(post)) if frame > c.at_frame => {
                self.length_init
                    + self.growth_per_frame * c.at_frame
                    + post * (frame - c.at_frame)
            }
            _ => self.length_init + self.growth_per_frame * frame,
        }
    }
}

/// Live spike-burst instance. Owners call [`SpikeBurst::tick`] every
/// effect-update tick and [`SpikeBurst::collect_draws`] to emit
/// billboards.
pub struct SpikeBurst {
    pub params: SpikeBurstParams,
    spikes: Vec<Spike>,
    age_frames: f32,
}

impl SpikeBurst {
    /// `seed` is mixed into a small LCG so repeat spawns at the same
    /// position get varied spike layouts when the caller bumps it.
    pub fn new(params: SpikeBurstParams, seed: u32) -> Self {
        let mut rng_state = seed ^ 0x9E37_79B9;
        let mut lcg = || {
            rng_state = rng_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            (rng_state >> 8) as f32 / ((1u32 << 24) as f32)
        };

        let mut spikes = Vec::with_capacity(params.count);
        for _ in 0..params.count {
            let longitude_deg = lcg() * 360.0;
            let angular_deg = params.angular_speed_deg_range.0
                + lcg() * (params.angular_speed_deg_range.1 - params.angular_speed_deg_range.0);
            let length_init = params.length_init_range.0
                + lcg() * (params.length_init_range.1 - params.length_init_range.0);
            let growth = params.growth_range.0
                + lcg() * (params.growth_range.1 - params.growth_range.0);
            let change_growth = params
                .change_growth
                .map(|c| c.growth_range.0 + lcg() * (c.growth_range.1 - c.growth_range.0));
            spikes.push(Spike {
                initial_longitude_rad: longitude_deg.to_radians(),
                // Positive angular speed advances the longitude in the
                // original's Y-flipped screen convention. Negate to project
                // to our CCW-positive screen-space `rotation` field.
                angular_speed_rad_per_frame: -angular_deg.to_radians(),
                length_init,
                growth_per_frame: growth,
                change_growth_per_frame: change_growth,
            });
        }

        Self {
            params,
            spikes,
            age_frames: 0.0,
        }
    }

    /// Advance by `delta_seconds`. Returns `true` while the burst is alive.
    pub fn tick(&mut self, delta_seconds: f32) -> bool {
        self.age_frames += delta_seconds * FRAMES_PER_SECOND;
        self.alive()
    }

    pub fn alive(&self) -> bool {
        self.age_frames < self.params.duration_frames
    }

    pub fn age_frames(&self) -> f32 {
        self.age_frames
    }

    pub fn current_alpha(&self) -> f32 {
        fade_in_out(
            self.age_frames,
            self.params.max_alpha,
            self.params.fade_in_frames,
            self.params.fade_out_start_frame,
            self.params.duration_frames,
        )
    }

    /// Emit one [`Billboard`] per spike at the given world anchor.
    ///
    /// [`Billboard`]: super::super::draw::EffectPrimitiveDraw::Billboard
    pub fn collect_draws(&self, out: &mut EffectDrawList, world_pos: [f32; 3]) {
        let alpha = self.current_alpha();
        if alpha <= 0.0 {
            return;
        }
        let pos = [
            world_pos[0],
            world_pos[1] + self.params.height_offset,
            world_pos[2],
        ];
        for spike in &self.spikes {
            let longitude = spike.longitude(self.age_frames, self.params.duration_frames);
            let length = spike.length(self.age_frames, self.params.change_growth);
            if length <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos,
                // Billboard straddles the entity centre — `alpha_center.tga`
                // peaks in the middle so the bright row crosses the anchor.
                size: [self.params.thickness, length * 2.0],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: longitude,
                texture: self.params.texture,
                color: [
                    self.params.color_tint[0],
                    self.params.color_tint[1],
                    self.params.color_tint[2],
                    alpha,
                ],
                blend: self.params.blend,
            });
        }
    }
}

/// Linear fade-in to `peak` over `fade_in` frames, hold, linear fade-out
/// from `fade_out_start` to `total`.
pub fn fade_in_out(
    frame: f32,
    peak: f32,
    fade_in: f32,
    fade_out_start: f32,
    total: f32,
) -> f32 {
    let rise = (frame / fade_in.max(1e-3)).clamp(0.0, 1.0);
    let fall = if frame < fade_out_start {
        1.0
    } else {
        let span = (total - fade_out_start).max(1e-3);
        (1.0 - (frame - fade_out_start) / span).clamp(0.0, 1.0)
    };
    peak * rise * fall
}

/// LCG-friendly seed derived from a world position so two spawns at the
/// same point produce identical bursts (deterministic playback for tests
/// and the effect viewer).
pub fn seed_from_world(world_pos: [f32; 3]) -> u32 {
    world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PARAMS: SpikeBurstParams = SpikeBurstParams {
        count: 20,
        duration_frames: 40.0,
        angular_speed_deg_range: (1.0, 7.0),
        length_init_range: (2.8, 5.6),
        growth_range: (0.28, 0.7),
        change_growth: None,
        thickness: 0.5,
        max_alpha: 0.78,
        fade_in_frames: 10.0,
        fade_out_start_frame: 40.0 - 40.0 / 3.0,
        height_offset: -5.0,
        texture: SPIKE_TEXTURE,
        color_tint: [1.0, 1.0, 1.0],
        blend: BlendKind::Alpha,
    };

    #[test]
    fn emits_one_billboard_per_spike() {
        let mut burst = SpikeBurst::new(TEST_PARAMS, 0);
        burst.tick(10.0 / FRAMES_PER_SECOND);
        let mut list = EffectDrawList::new();
        burst.collect_draws(&mut list, [0.0, 0.0, 0.0]);
        let n = list
            .primitives
            .iter()
            .filter(|p| {
                matches!(p, EffectPrimitiveDraw::Billboard { texture, .. } if *texture == SPIKE_TEXTURE)
            })
            .count();
        assert_eq!(n, TEST_PARAMS.count);
    }

    #[test]
    fn spike_length_grows_over_time_and_respects_change_phase() {
        let mut burst = SpikeBurst::new(
            SpikeBurstParams {
                duration_frames: 80.0,
                change_growth: Some(ChangeGrowth {
                    at_frame: 40.0,
                    growth_range: (0.05, 0.05),
                }),
                growth_range: (0.5, 0.5),
                length_init_range: (0.0, 0.0),
                ..TEST_PARAMS
            },
            0xDEAD_BEEF,
        );
        burst.tick(40.0 / FRAMES_PER_SECOND);
        let mid =
            burst.spikes[0].length(burst.age_frames(), burst.params.change_growth);
        burst.tick(40.0 / FRAMES_PER_SECOND);
        let late =
            burst.spikes[0].length(burst.age_frames(), burst.params.change_growth);
        // 40 * 0.5 = 20 at frame 40; +40 * 0.05 = +2 at frame 80.
        assert!((mid - 20.0).abs() < 1e-3, "mid {mid}");
        assert!((late - 22.0).abs() < 1e-3, "late {late}");
    }

    #[test]
    fn dies_after_duration() {
        let mut burst = SpikeBurst::new(TEST_PARAMS, 0);
        for _ in 0..(TEST_PARAMS.duration_frames as i32 + 2) {
            burst.tick(1.0 / FRAMES_PER_SECOND);
        }
        assert!(!burst.alive());
    }
}
