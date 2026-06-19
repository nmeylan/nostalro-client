//! `EF_HEAVENSDRIVE` — Wizard Heaven's Drive (id 142).
//!
//! Launches a
//! 5×5 grid of 25 `stone.bmp` blades at frame 0, one-shot, and
//! triggers a screen quake. Each blade rises along its own apex axis,
//! reverses, freezes, then sinks back into the ground at the end of its
//! life. Per-blade params are randomised:
//!   * grid cell offset `(-10 + j*5, _, -10 + i*5)` (orig units),
//!   * size `(random(50)+300)/100` → 3.0..3.5,
//!   * height `9 + (random(60)+1)/10` → 9.1..15.1,
//!   * heading `random(360)`, tilt `80 + random(20)`.
//!
//! Per-blade motion:
//!   * `speed = 1.0, accel = 0.01` (rise) from frame 0,
//!   * frame 11: `speed = -1.2, accel = 0` (reverse),
//!   * frame 14 (speed limit): `speed = accel = 0` (freeze),
//!   * frame `duration-30` (return down): `speed = -1.0, accel = -0.01` (sink).
//!
//! Sizes/heights/distances are scaled to our world units (~0.4× like
//! `earthspike`, the sibling stone-spike QuadHorn effect) and tuned against the
//! reference gif. The screen quake is skipped (no effect-layer hook).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::STONE_TEXTURE;
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const GRID: i32 = 5;
const SPIKE_COUNT: usize = (GRID * GRID) as usize;
/// Orig grid step is 5 units; scaled to our world.
const GRID_STEP: f32 = 2.0;

/// Orig→ours scale factors (cf. `earthspike`: orig size 3..5 → 1.2..1.8,
/// height 18/20 → 9/6.7).
const SIZE_SCALE: f32 = 0.4;
const HEIGHT_SCALE: f32 = 0.45;
/// Apex travel is integrated in orig units (speed≈1/frame) then scaled.
const DIST_SCALE: f32 = 0.4;

// Per-blade motion constants.
const SPEED_INIT: f32 = 1.0;
const ACCEL_INIT: f32 = 0.01;
const CHANGE_POINT_FRAME: f32 = 11.0;
const CHANGE_SPEED: f32 = -1.2;
const CHANGE_ACCEL: f32 = 0.0;
const SPEEDLIMIT_FRAME: f32 = 14.0;
const RETURNDOWN_SPEED: f32 = -1.0;
const RETURNDOWN_ACCEL: f32 = -0.01;

/// Effect duration is 250 + 50 ≈ 300 frames; the return-down sink
/// starts 30 frames before the end.
const DURATION_FRAMES: f32 = 300.0;
const RETURNDOWN_BEFORE_END: f32 = 30.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

struct Spike {
    base: [f32; 3],
    axis: [f32; 3],
    tilt_deg: f32,
    heading_deg: f32,
    size: f32,
    height: f32,
    /// Apex travel distance in original units (scaled by `DIST_SCALE` on draw).
    distance: f32,
    speed: f32,
    accel: f32,
}

impl Spike {
    /// One frame of the 4-phase QuadHorn integrator. `frame` is the
    /// blade's age in 60 fps frames at the *start* of the step.
    fn step(&mut self, frame: f32, dt_frames: f32) {
        if frame <= CHANGE_POINT_FRAME && frame + dt_frames > CHANGE_POINT_FRAME {
            self.speed = CHANGE_SPEED;
            self.accel = CHANGE_ACCEL;
        }
        self.speed += self.accel * dt_frames;
        self.distance += self.speed * dt_frames;
        if frame <= SPEEDLIMIT_FRAME && frame + dt_frames > SPEEDLIMIT_FRAME {
            self.speed = 0.0;
            self.accel = 0.0;
        }
        let sink_frame = DURATION_FRAMES - RETURNDOWN_BEFORE_END;
        if frame <= sink_frame && frame + dt_frames > sink_frame {
            self.speed = RETURNDOWN_SPEED;
            self.accel = RETURNDOWN_ACCEL;
        }
    }

    fn position(&self) -> [f32; 3] {
        let d = self.distance * DIST_SCALE;
        [
            self.base[0] + self.axis[0] * d,
            self.base[1] + self.axis[1] * d,
            self.base[2] + self.axis[2] * d,
        ]
    }
}

pub struct HeavensDriveEffect {
    spikes: Vec<Spike>,
    age_frames: f32,
}

impl HeavensDriveEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        // Deterministic LCG seeded from the spawn point (cf. spike_burst) so
        // the grid keeps the original's varied look while tests stay stable.
        let mut rng_state = seed_from_world(world_pos) ^ 0x9E37_79B9;
        let mut lcg = || {
            rng_state = rng_state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (rng_state >> 8) as f32 / ((1u32 << 24) as f32)
        };

        let mut spikes = Vec::with_capacity(SPIKE_COUNT);
        for i in 0..GRID {
            for j in 0..GRID {
                let off_x = (-(GRID - 1) as f32 / 2.0 + j as f32) * GRID_STEP;
                let off_z = (-(GRID - 1) as f32 / 2.0 + i as f32) * GRID_STEP;
                let size = (3.0 + lcg() * 0.5) * SIZE_SCALE;
                let height = (9.0 + lcg() * 6.0) * HEIGHT_SCALE;
                let heading = lcg() * 360.0;
                let tilt = 80.0 + lcg() * 20.0;
                spikes.push(Spike {
                    base: [world_pos[0] + off_x, world_pos[1], world_pos[2] + off_z],
                    axis: apex_velocity(tilt, heading, 1.0),
                    tilt_deg: tilt,
                    heading_deg: heading,
                    size,
                    height,
                    distance: 0.0,
                    speed: SPEED_INIT,
                    accel: ACCEL_INIT,
                });
            }
        }
        Self { spikes, age_frames: 0.0 }
    }
}

impl Effect for HeavensDriveEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        for s in &mut self.spikes {
            s.step(self.age_frames, dt_frames);
        }
        self.age_frames += dt_frames;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: s.position(),
                size: s.size,
                height: s.height,
                tilt_x_deg: s.tilt_deg,
                rotation_y_deg: s.heading_deg,
                texture: STONE_TEXTURE,
                // Opaque brown stone (cf. earthspike / grimtooth): the sinking
                // blades skip alpha fading, so hold full
                // alpha and let the geometry sink carry the exit.
                color: [1.0, 1.0, 1.0, 1.0],
                blend: BlendKind::Alpha,
            });
        }
    }
}

fn seed_from_world(world_pos: [f32; 3]) -> u32 {
    let mut h = 0x811C_9DC5u32;
    for c in world_pos {
        h ^= (c.to_bits()).rotate_left(5);
        h = h.wrapping_mul(16_777_619);
    }
    h
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

    fn draws(e: &HeavensDriveEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn sample_y(e: &HeavensDriveEffect) -> f32 {
        match &draws(e)[0] {
            EffectPrimitiveDraw::QuadHorn { base, .. } => base[1],
            _ => unreachable!(),
        }
    }

    #[test]
    fn emits_grid_of_twenty_five_stone_blades_then_dies() {
        // Sociable: 5×5 grid of QuadHorn stone blades at frame 0, all on a
        // square footprint around the anchor; the effect ends after its window.
        let mut e = HeavensDriveEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None });
        let prims = draws(&e);
        assert_eq!(prims.len(), 25);
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn { base, texture, .. } = p else {
                panic!("expected QuadHorn");
            };
            assert_eq!(*texture, STONE_TEXTURE);
            let span = (GRID - 1) as f32 / 2.0 * GRID_STEP + 0.001;
            assert!(base[0].abs() <= span && base[2].abs() <= span, "blade inside grid footprint");
        }

        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + 5) {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn blade_rises_then_sinks_back_down() {
        // Sociable: a blade rises (Y more negative in native RO) through the
        // speed window, holds, then sinks back below its risen height once
        // the return-down phase kicks in near the end.
        let mut e = HeavensDriveEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None });
        let y_start = sample_y(&e);

        // Step to just past the freeze frame (14).
        for _ in 0..16 {
            e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        let y_risen = sample_y(&e);
        assert!(y_risen < y_start, "blade rose during speed window: {y_start} -> {y_risen}");

        // Run out the rest of the life; the sink pulls it back down (Y up
        // toward / past start).
        while e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None })
            != EffectStatus::Dead
        {}
        // Re-create and sample just before death to read the sunk position.
        let mut e2 = HeavensDriveEffect::new([0.0, 0.0, 0.0]);
        let mut prev = sample_y(&e2);
        for _ in 0..(DURATION_FRAMES as i32 - 1) {
            e2.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
            prev = sample_y(&e2);
        }
        assert!(prev > y_risen, "blade sank back down at end of life: {y_risen} -> {prev}");
    }
}
