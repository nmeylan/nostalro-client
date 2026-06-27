use super::spike_burst::{self, ChangeGrowth, SpikeBurst, SpikeBurstParams, seed_from_world};
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &[spike_burst::SPIKE_TEXTURE];
pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SPIKE_DURATION_FRAMES: f32 = 80.0;
const PARENT_DURATION_FRAMES: f32 = 300.0;
const PARTICLE_DURATION_FRAMES: f32 = 100.0;

pub const SPIKES: SpikeBurstParams = SpikeBurstParams {
    count: 20,
    duration_frames: SPIKE_DURATION_FRAMES,
    angular_speed_deg_range: (1.0, 7.0),
    length_init_range: (0.0, 0.0),
    growth_range: (3.85 / 6.0, 5.31 / 6.0),
    change_growth: Some(ChangeGrowth {
        at_frame: 40.0,
        growth_range: (1.5 / 6.0, 2.4 / 6.0),
    }),
    thickness: 0.35,
    max_alpha: 90.0 / 255.0,
    fade_in_frames: 10.0,
    fade_out_start_frame: SpikeBurstParams::default_fade_out_start(SPIKE_DURATION_FRAMES),
    height_offset: -5.0,
    texture: spike_burst::SPIKE_TEXTURE,
    color_tint: [1.0, 1.0, 1.0],
    blend: BlendKind::Alpha,
};

const ORBIT_RADIUS: f32 = 7.0;
const PARTICLE_SIZE: f32 = 1.2;
const PARTICLE_INITIAL_Y_OFFSET: f32 = -5.0;
const PARTICLE_Y_SPEED_PER_FRAME: f32 = -0.1;
const PARTICLE_Y_ACCEL_PER_FRAME: f32 = -0.002;
const PARTICLE_LONG_SPEED_INIT_DEG: f32 = 0.3;
const PARTICLE_LONG_ACCEL_DEG_PER_FRAME: f32 = 0.2;
const PARTICLE_FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - PARTICLE_DURATION_FRAMES / 10.0;

const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

pub const TOTAL_DURATION_MS: u32 = ((PARENT_DURATION_FRAMES) / FRAMES_PER_SECOND * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
struct OrbitParticle {
    initial_longitude_deg: f32,
    age_frames: f32,
    y_offset: f32,
    y_velocity_per_frame: f32,
}

impl OrbitParticle {
    fn step(&mut self, dt_frames: f32) {
        self.y_velocity_per_frame += PARTICLE_Y_ACCEL_PER_FRAME * dt_frames;
        self.y_offset += self.y_velocity_per_frame * dt_frames;
        self.age_frames += dt_frames;
    }

    fn longitude_deg(&self) -> f32 {
        let n = self.age_frames;
        self.initial_longitude_deg
            + n * PARTICLE_LONG_SPEED_INIT_DEG
            + PARTICLE_LONG_ACCEL_DEG_PER_FRAME * n * (n + 1.0) / 2.0
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < PARTICLE_FADEOUT_AT {
            1.0
        } else {
            let span = (PARTICLE_DURATION_FRAMES - PARTICLE_FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - PARTICLE_FADEOUT_AT) / span).clamp(0.0, 1.0)
        }
    }

    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn position(&self, anchor: [f32; 3]) -> [f32; 3] {
        let rad = self.longitude_deg().to_radians();
        let (sn, cs) = rad.sin_cos();
        [
            anchor[0] + ORBIT_RADIUS * sn,
            anchor[1] + self.y_offset,
            anchor[2] + ORBIT_RADIUS * cs,
        ]
    }
}

pub struct HasteUpEffect {
    world_pos: [f32; 3],
    spikes: SpikeBurst,
    particles: Vec<OrbitParticle>,
    age_frames: f32,
}

impl HasteUpEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let particles = (0..4)
            .map(|i| OrbitParticle {
                initial_longitude_deg: i as f32 * 90.0,
                age_frames: 0.0,
                y_offset: PARTICLE_INITIAL_Y_OFFSET,
                y_velocity_per_frame: PARTICLE_Y_SPEED_PER_FRAME,
            })
            .collect();
        Self {
            world_pos,
            spikes: SpikeBurst::new(SPIKES, seed_from_world(world_pos)),
            particles,
            age_frames: 0.0,
        }
    }
}

impl Effect for HasteUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;
        self.spikes.tick(ctx.delta);
        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames >= PARENT_DURATION_FRAMES
            && !self.spikes.alive()
            && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        self.spikes.collect_draws(out, self.world_pos);
        for p in &self.particles {
            let a = p.alpha();
            if a <= 0.0 {
                continue;
            }
            let motion = (p.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE_SPRITE,
                position: p.position(self.world_pos),
                action_index: 0,
                motion_index: motion,
                size_scale: PARTICLE_SIZE,
                color: [1.0, 1.0, 1.0, a],
                blend: BlendKind::Additive,
                aim_target: None,
                no_depth: false,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_frames(e: &mut HasteUpEffect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn emits_twenty_spikes_plus_four_orbit_particles_at_spawn() {
        let mut e = HasteUpEffect::new([0.0; 3]);
        step_frames(&mut e, 5);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let spikes = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::BillboardFlash { texture, .. } if *texture == spike_burst::SPIKE_TEXTURE))
            .count();
        let particles = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == PARTICLE_SPRITE))
            .count();
        assert_eq!(spikes, 20);
        assert_eq!(particles, 4);
    }

    #[test]
    fn orbit_particles_rotate_and_drift_upward() {
        let mut e = HasteUpEffect::new([0.0; 3]);
        step_frames(&mut e, 10);
        let early_lon = e.particles[0].longitude_deg();
        let early_y = e.particles[0].y_offset;
        step_frames(&mut e, 30);
        let late_lon = e.particles[0].longitude_deg();
        let late_y = e.particles[0].y_offset;
        assert!(
            late_lon > early_lon,
            "longitude advances {early_lon} → {late_lon}"
        );
        assert!(
            late_y < early_y,
            "drifts up (Y decreases) {early_y} → {late_y}"
        );
    }

    #[test]
    fn dies_after_parent_duration() {
        let mut e = HasteUpEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(PARENT_DURATION_FRAMES as i32 + 5) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
