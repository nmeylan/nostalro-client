use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::STONE_TEXTURE;
use crate::effects::spike_util::{
    FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, speed_limited_travel,
};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const DURATION_FRAMES: f32 = 250.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const STEP_PER_FRAME: f32 = 2.0;
const SPAWN_EVERY_FRAMES: f32 = 3.0;
const SINGLE_SPIKE_DISTANCE: f32 = 2.5;
const SPAWN_DEPTH: f32 = 20.0;

const SPIKE_DURATION_FRAMES: f32 = 40.0;
const FADE_OUT_FRAMES: f32 = 10.0;
const RISE_SPEED: f32 = 3.0;
const RISE_ACCEL: f32 = -RISE_SPEED / SPIKE_DURATION_FRAMES * 2.0;
const RISE_LAST_FRAME: f32 = 6.0;
const HEIGHT: f32 = 10.0;
const ALPHA: f32 = 254.0 / 255.0;

fn lcg_next(state: &mut u32) -> u32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

struct Spike {
    spawn_frame: f32,
    base: [f32; 3],
    apex_dir: [f32; 3],
    tilt_deg: f32,
    heading_deg: f32,
    size: f32,
}

impl Spike {
    fn age_frames(&self, effect_frames: f32) -> f32 {
        effect_frames - self.spawn_frame
    }

    fn position(&self, effect_frames: f32) -> [f32; 3] {
        let d = speed_limited_travel(
            self.age_frames(effect_frames),
            RISE_SPEED,
            RISE_ACCEL,
            RISE_LAST_FRAME,
        );
        [
            self.base[0] + self.apex_dir[0] * d,
            self.base[1] + self.apex_dir[1] * d,
            self.base[2] + self.apex_dir[2] * d,
        ]
    }

    fn alpha(&self, effect_frames: f32) -> f32 {
        let age = self.age_frames(effect_frames);
        let own = fade_tail_alpha(
            age / FRAMES_PER_SECOND,
            SPIKE_DURATION_FRAMES / FRAMES_PER_SECOND,
            ALPHA,
            FADE_OUT_FRAMES,
        );
        let effect_left = DURATION_FRAMES - self.spawn_frame;
        if SPIKE_DURATION_FRAMES > effect_left + 1.0 {
            own.min(ALPHA * (1.0 - age / effect_left).max(0.0))
        } else {
            own
        }
    }
}

pub struct GrimToothEffect {
    origin: [f32; 3],
    direction: [f32; 2],
    single_spike: bool,
    next_spawn_frame: f32,
    age: f32,
    spikes: Vec<Spike>,
    rng_state: u32,
}

impl GrimToothEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
        let distance = (dx * dx + dz * dz).sqrt();
        let direction = if distance > f32::EPSILON {
            [dx / distance, dz / distance]
        } else {
            [0.0, 1.0]
        };
        Self {
            origin: from,
            direction,
            single_spike: distance <= SINGLE_SPIKE_DISTANCE,
            next_spawn_frame: 0.0,
            age: 0.0,
            spikes: Vec::new(),
            rng_state: 0x9E37_79B9 ^ from[0].to_bits() ^ to[2].to_bits().rotate_left(11),
        }
    }

    fn frames(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    fn spawn(&mut self, frame: f32) {
        let along = STEP_PER_FRAME * (frame + 1.0);
        let heading_deg = (lcg_next(&mut self.rng_state) % 360) as f32;
        let tilt_deg = (lcg_next(&mut self.rng_state) % 30 + 75) as f32;
        let size = (lcg_next(&mut self.rng_state) % 40 + 60) as f32 / 100.0;
        self.spikes.push(Spike {
            spawn_frame: frame,
            base: [
                self.origin[0] + self.direction[0] * along,
                self.origin[1] + SPAWN_DEPTH,
                self.origin[2] + self.direction[1] * along,
            ],
            apex_dir: apex_velocity(tilt_deg, heading_deg, 1.0),
            tilt_deg,
            heading_deg,
            size,
        });
    }
}

impl Effect for GrimToothEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = self.frames();
        while self.next_spawn_frame < DURATION_FRAMES && self.next_spawn_frame <= frames + 1e-3 {
            let frame = self.next_spawn_frame;
            self.spawn(frame);
            self.next_spawn_frame = if self.single_spike {
                f32::INFINITY
            } else {
                frame + SPAWN_EVERY_FRAMES
            };
        }

        self.age += ctx.delta;
        let frames = self.frames();
        self.spikes
            .retain(|s| s.age_frames(frames) < SPIKE_DURATION_FRAMES);

        if frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.frames();
        for spike in &self.spikes {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: spike.position(frames),
                size: spike.size,
                height: HEIGHT,
                tilt_x_deg: spike.tilt_deg,
                rotation_y_deg: spike.heading_deg,
                texture: STONE_TEXTURE,
                color: [1.0, 1.0, 1.0, spike.alpha(frames)],
                blend: BlendKind::Alpha,
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

    fn step_frames(e: &mut GrimToothEffect, frames: u32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        for _ in 0..frames {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        status
    }

    fn spike_bases(e: &GrimToothEffect) -> Vec<[f32; 3]> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::QuadHorn { base, .. } => *base,
                other => panic!("expected QuadHorn, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn spike_front_travels_toward_target_and_keeps_going_past_it() {
        let mut e = GrimToothEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 20.0]);

        step_frames(&mut e, 4);
        assert_eq!(e.spikes.len(), 2, "one spike every third frame");
        let early_reach = spike_bases(&e)
            .iter()
            .map(|b| b[2])
            .fold(f32::MIN, f32::max);

        step_frames(&mut e, 30);
        let bases = spike_bases(&e);
        let late_reach = bases.iter().map(|b| b[2]).fold(f32::MIN, f32::max);
        assert!(late_reach > early_reach + 30.0);
        assert!(
            late_reach > 20.0,
            "the front overshoots the target: {late_reach}"
        );
        assert!(
            bases.iter().all(|b| b[0].abs() < 6.0),
            "spikes stay on the line"
        );

        assert_eq!(step_frames(&mut e, 250), EffectStatus::Dead);
    }

    #[test]
    fn caster_on_target_lays_a_single_spike() {
        let mut e = GrimToothEffect::new([0.0, 0.0, 0.0], [1.0, 0.0, 1.0]);
        step_frames(&mut e, 30);
        assert_eq!(e.spikes.len(), 1);
    }

    #[test]
    fn spike_rises_out_of_the_ground_freezes_then_fades() {
        let mut e = GrimToothEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 50.0]);
        step_frames(&mut e, 1);
        let first = &e.spikes[0];
        assert!(
            first.position(0.0)[1] > 0.0,
            "spawns below the ground (+Y is down)"
        );

        let rest = first.position(RISE_LAST_FRAME + 1.0);
        assert!(rest[1] < first.position(0.0)[1] - 15.0);
        assert_eq!(first.position(30.0), rest, "frozen once the rise ends");

        assert!((first.alpha(1.0) - ALPHA).abs() < 1e-4);
        assert!(first.alpha(SPIKE_DURATION_FRAMES - 2.0) < ALPHA / 2.0);
    }
}
