use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, aim_backward};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const FIREBALL_SPRITE: &str = "data/sprite/이팩트/fireball";
pub const SPRITES: &[&str] = &[FIREBALL_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;

const SPAWN_FRAMES: [f32; 4] = [20.0, 24.0, 28.0, 32.0];
const PARTICLE_DURATION_FRAMES: f32 = 20.0;
const PARTICLE_DURATION_S: f32 = PARTICLE_DURATION_FRAMES / FRAMES_PER_SECOND;

const PARTICLE_ANIM_TICKS: f32 = 3.0;
const PARTICLE_FRAME_MS: f32 = 1000.0 / FRAMES_PER_SECOND * PARTICLE_ANIM_TICKS;

const SPRITE_SIZE: f32 = 1.3;
const TARGET_KILL_DISTANCE: f32 = 3.0;

const PARTICLE_COLORS: [[f32; 4]; 4] = [
    [1.0, 1.0, 1.0, 1.0],
    [246.0 / 255.0, 199.0 / 255.0, 76.0 / 255.0, 180.0 / 255.0],
    [246.0 / 255.0, 199.0 / 255.0, 76.0 / 255.0, 130.0 / 255.0],
    [246.0 / 255.0, 199.0 / 255.0, 76.0 / 255.0, 80.0 / 255.0],
];

const LAST_SPAWN_FRAME: f32 = SPAWN_FRAMES[3];
const TOTAL_VISIBLE_FRAMES: f32 = LAST_SPAWN_FRAME + PARTICLE_DURATION_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_VISIBLE_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub const PROJECTILE_FLIGHT: crate::effect_queue::ProjectileFlight =
    crate::effect_queue::ProjectileFlight::FixedFrames(SPAWN_FRAMES[0] + PARTICLE_DURATION_FRAMES);

const STATIC_DURATION_FRAMES: f32 = 60.0;
const STATIC_DURATION_S: f32 = STATIC_DURATION_FRAMES / FRAMES_PER_SECOND;
const STATIC_BASE_SIZE: f32 = 2.0;

#[derive(Clone, Copy)]
struct FireballParticle {
    spawn_time: f32,
    origin: [f32; 3],
    velocity: [f32; 3],
    color: [f32; 4],
}

impl FireballParticle {
    fn age(&self, emitter_age: f32) -> f32 {
        (emitter_age - self.spawn_time).max(0.0)
    }

    fn current_pos(&self, emitter_age: f32) -> [f32; 3] {
        let age = self.age(emitter_age);
        [
            self.origin[0] + self.velocity[0] * age,
            self.origin[1] + self.velocity[1] * age,
            self.origin[2] + self.velocity[2] * age,
        ]
    }

    fn alive(&self, emitter_age: f32) -> bool {
        self.age(emitter_age) < PARTICLE_DURATION_S
    }

    fn reached_target(&self, emitter_age: f32, to: [f32; 3]) -> bool {
        let pos = self.current_pos(emitter_age);
        let dx = pos[0] - to[0];
        let dz = pos[2] - to[2];
        (dx * dx + dz * dz).sqrt() <= TARGET_KILL_DISTANCE
    }
}

pub struct FireballEffect {
    from: [f32; 3],
    to: [f32; 3],
    age: f32,
    particles: Vec<FireballParticle>,
    next_spawn_index: u32,
    is_trail: bool,
}

impl FireballEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let is_trail = (dx * dx + dz * dz).sqrt() > TARGET_KILL_DISTANCE;

        Self {
            from,
            to,
            age: 0.0,
            particles: Vec::with_capacity(4),
            next_spawn_index: 0,
            is_trail,
        }
    }

    fn spawn_particle(&mut self) {
        let idx = self.next_spawn_index as usize;
        let dx = self.to[0] - self.from[0];
        let dz = self.to[2] - self.from[2];
        let dist = (dx * dx + dz * dz).sqrt();

        let velocity = if dist > 0.001 {
            let speed_per_frame = dist / PARTICLE_DURATION_FRAMES;
            let speed_per_s = speed_per_frame * FRAMES_PER_SECOND;
            let ux = dx / dist;
            let uz = dz / dist;
            [ux * speed_per_s, 0.0, uz * speed_per_s]
        } else {
            [0.0, 0.0, 0.0]
        };

        self.particles.push(FireballParticle {
            spawn_time: self.age,
            origin: self.from,
            velocity,
            color: PARTICLE_COLORS[idx],
        });
        self.next_spawn_index += 1;
    }
}

impl Effect for FireballEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;

        if self.is_trail {
            while (self.next_spawn_index as usize) < SPAWN_FRAMES.len() {
                let spawn_time_s = SPAWN_FRAMES[self.next_spawn_index as usize] / FRAMES_PER_SECOND;
                if self.age >= spawn_time_s {
                    self.spawn_particle();
                } else {
                    break;
                }
            }

            let age = self.age;
            let to = self.to;
            self.particles
                .retain(|p| p.alive(age) && !p.reached_target(age, to));

            let all_spawned = self.next_spawn_index as usize >= SPAWN_FRAMES.len();
            if all_spawned && self.particles.is_empty() {
                EffectStatus::Dead
            } else {
                EffectStatus::Running
            }
        } else {
            if self.age >= STATIC_DURATION_S {
                EffectStatus::Dead
            } else {
                EffectStatus::Running
            }
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.is_trail {
            for particle in &self.particles {
                let pos = particle.current_pos(self.age);
                let age_ms = particle.age(self.age) * 1000.0;
                let motion = (age_ms / PARTICLE_FRAME_MS) as usize;
                let render_pos = [pos[0], pos[1] - 1.5, pos[2]];

                out.push(EffectPrimitiveDraw::SpriteParticle {
                    sprite_path: FIREBALL_SPRITE,
                    position: render_pos,
                    action_index: 0,
                    motion_index: motion,
                    size_scale: SPRITE_SIZE,
                    color: particle.color,
                    blend: BlendKind::Additive,
                    aim_target: Some(aim_backward(render_pos, self.to)),
                    no_depth: false,
                });
            }
        } else {
            let t = (self.age / STATIC_DURATION_S).clamp(0.0, 1.0);
            let alpha = if t < 0.7 {
                1.0
            } else {
                (1.0 - (t - 0.7) / 0.3).clamp(0.0, 1.0)
            };
            let scale = STATIC_BASE_SIZE * (1.0 + t * 0.6);
            let pos = [self.from[0], self.from[1] - 1.5, self.from[2]];
            let motion = (self.age * 1000.0 / PARTICLE_FRAME_MS) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: FIREBALL_SPRITE,
                position: pos,
                action_index: 0,
                motion_index: motion,
                size_scale: scale,
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

    fn step(e: &mut FireballEffect, dt: f32) {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &FireballEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn static_fallback_emits_single_sprite_then_dies() {
        let mut e = FireballEffect::new([5.0, 0.0, 7.0], [5.0, 0.0, 7.0]);
        step(&mut e, 0.0);
        assert_eq!(draws(&e).len(), 1);
        let mut status = EffectStatus::Running;
        for _ in 0..120 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn trail_spawns_four_staggered_particles() {
        let mut e = FireballEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        step(&mut e, 19.0 / FRAMES_PER_SECOND);
        assert_eq!(draws(&e).len(), 0, "no particles before frame 20");

        step(&mut e, 2.0 / FRAMES_PER_SECOND);
        assert!(draws(&e).len() >= 1, "at least 1 particle after frame 21");

        step(&mut e, 12.0 / FRAMES_PER_SECOND);
        assert_eq!(e.next_spawn_index, 4, "all 4 spawned by frame 33");
    }

    #[test]
    fn particles_move_toward_target() {
        let mut e = FireballEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        for _ in 0..21 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let z1 = match &draws(&e)[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => panic!("expected SpriteParticle"),
        };

        for _ in 0..5 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let z2 = match &draws(&e)[0] {
            EffectPrimitiveDraw::SpriteParticle { position, .. } => position[2],
            _ => panic!("expected SpriteParticle"),
        };
        assert!(z2 > z1, "particle moved toward +Z target: {z1} -> {z2}");
    }

    #[test]
    fn trail_sprite_aims_away_from_target_for_180_flip() {
        let to = [0.0, 0.0, 60.0];
        let mut e = FireballEffect::new([0.0, 0.0, 0.0], to);
        for _ in 0..21 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        match &draws(&e)[0] {
            EffectPrimitiveDraw::SpriteParticle {
                position,
                aim_target: Some(aim),
                ..
            } => {
                assert!(
                    aim[2] - position[2] < 0.0,
                    "aim must point away from +Z target: pos {position:?} aim {aim:?}"
                );
            }
            _ => panic!("expected aimed SpriteParticle"),
        }
    }

    #[test]
    fn lead_white_trail_golden_decreasing_alpha() {
        let mut e = FireballEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 80.0]);
        for _ in 0..33 {
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
        }
        let prims = draws(&e);
        assert_eq!(prims.len(), 4);

        let colors: Vec<[f32; 4]> = prims
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { color, .. } => *color,
                _ => panic!("expected SpriteParticle"),
            })
            .collect();

        assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0], "lead is white");
        assert!(colors[1][3] > colors[2][3], "trail alpha decreases");
        assert!(colors[2][3] > colors[3][3], "trail alpha decreases");
        for c in &colors[1..] {
            assert!(c[0] > 0.9 && c[1] > 0.7 && c[2] < 0.4, "golden tint");
        }
    }

    #[test]
    fn trail_dies_when_all_particles_expire() {
        let mut e = FireballEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 60.0]);
        let mut status = EffectStatus::Running;
        for _ in 0..200 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
