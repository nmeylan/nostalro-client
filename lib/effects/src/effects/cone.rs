use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const PARTICLE1_SPRITE: &str = ragnarok_resources::sprite::effect::PARTICLE1;
pub const SPRITES: &[&str] = &[PARTICLE1_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SPAWN_PERIOD_FRAMES: f32 = 130.0;
const PARENT_DURATION_FRAMES: f32 = 200.0;
const PARTICLE_DURATION_FRAMES: f32 = 300.0;

const RADIUS_INIT: f32 = 0.1;
const RADIUS_SPEED: f32 = 0.2;
const LONG_INIT_DEG: f32 = 2.0;
const LONG_SPEED_INIT_DEG: f32 = 15.0;
const LONG_ACCEL_DEG: f32 = -0.01;
const GRAV_Y_INIT: f32 = -30.0;
const GRAV_SPEED_INIT: f32 = 0.1;
const GRAV_ACCEL: f32 = 0.005;
const SIZE: f32 = 1.2;

const POS_SCALE: f32 = 0.3;

const PARTICLE_ANIM_TICKS: f32 = 4.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;
const FADEOUT_AT: f32 = PARTICLE_DURATION_FRAMES - PARTICLE_DURATION_FRAMES / 10.0;

const TOTAL_FRAMES: f32 = SPAWN_PERIOD_FRAMES + PARTICLE_DURATION_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

#[derive(Clone, Copy)]
struct Particle {
    age_frames: f32,
    longitude_deg: f32,
    long_speed_deg: f32,
    radius: f32,
    grav_y: f32,
    grav_speed: f32,
}

impl Particle {
    fn new() -> Self {
        Self {
            age_frames: 0.0,
            longitude_deg: LONG_INIT_DEG,
            long_speed_deg: LONG_SPEED_INIT_DEG,
            radius: RADIUS_INIT,
            grav_y: GRAV_Y_INIT,
            grav_speed: GRAV_SPEED_INIT,
        }
    }

    fn step(&mut self, dt_frames: f32) {
        self.grav_speed += GRAV_ACCEL * dt_frames;
        self.grav_y += self.grav_speed * dt_frames;
        self.long_speed_deg += LONG_ACCEL_DEG * dt_frames;
        self.longitude_deg += self.long_speed_deg * dt_frames;
        self.radius += RADIUS_SPEED * dt_frames;
        self.age_frames += dt_frames;
    }

    fn alive(&self) -> bool {
        self.age_frames < PARTICLE_DURATION_FRAMES
    }

    fn alpha(&self) -> f32 {
        if self.age_frames < FADEOUT_AT {
            1.0
        } else {
            let span = (PARTICLE_DURATION_FRAMES - FADEOUT_AT).max(1e-3);
            (1.0 - (self.age_frames - FADEOUT_AT) / span).clamp(0.0, 1.0)
        }
    }

    fn position(&self, anchor: [f32; 3]) -> [f32; 3] {
        let rad = self.longitude_deg.to_radians();
        let (sn, cs) = rad.sin_cos();
        [
            anchor[0] + self.radius * sn * POS_SCALE,
            anchor[1] + self.grav_y * POS_SCALE,
            anchor[2] + self.radius * cs * POS_SCALE,
        ]
    }
}

pub struct ConeEffect {
    world_pos: [f32; 3],
    particles: Vec<Particle>,
    age_frames: f32,
    last_spawn_frame: i32,
}

impl ConeEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            particles: Vec::new(),
            age_frames: 0.0,
            last_spawn_frame: -1,
        }
    }
}

impl Effect for ConeEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;

        let current_frame = self.age_frames.floor() as i32;
        let next = self.last_spawn_frame + 1;
        for f in next..=current_frame {
            if f >= 0
                && f as f32 <= PARENT_DURATION_FRAMES
                && (f as f32) % SPAWN_PERIOD_FRAMES == 0.0
            {
                self.particles.push(Particle::new());
            }
        }
        self.last_spawn_frame = current_frame;

        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());
        self.age_frames += dt_frames;

        if self.age_frames >= TOTAL_FRAMES && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            let alpha = p.alpha();
            if alpha <= 0.0 {
                continue;
            }
            let motion = (p.age_frames * (1000.0 / FRAMES_PER_SECOND) / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE1_SPRITE,
                position: p.position(self.world_pos),
                action_index: 0,
                motion_index: motion,
                size_scale: SIZE,
                color: [1.0, 1.0, 1.0, alpha],
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

    fn step(e: &mut ConeEffect, frames: i32) {
        for _ in 0..frames {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    #[test]
    fn spawns_particle_at_frame_zero_then_again_at_period() {
        let mut e = ConeEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 1);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1);
        assert!(matches!(
            &list.primitives[0],
            EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == PARTICLE1_SPRITE
        ));

        step(&mut e, 131);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(
            list.primitives.len(),
            2,
            "second particle spawned at frame 130"
        );
    }

    #[test]
    fn particle_spirals_outward_and_dies() {
        let mut e = ConeEffect::new([0.0, 0.0, 0.0]);
        step(&mut e, 5);
        let r0 = e.particles[0].radius;
        step(&mut e, 30);
        let r1 = e.particles[0].radius;
        assert!(r1 > r0, "orbit radius grows: {r0} -> {r1}");

        let mut status = EffectStatus::Running;
        for _ in 0..(TOTAL_FRAMES as i32 + 5) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
