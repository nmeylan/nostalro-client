use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SIGHT_SPRITE: &str = ragnarok_resources::sprite::effect::SIGHT;
pub const SHADOW_SPRITE: &str = ragnarok_resources::sprite::SHADOW;
pub const PARTICLE2_SPRITE: &str = ragnarok_resources::sprite::effect::PARTICLE2;

pub const SPRITES: &[&str] = &[SIGHT_SPRITE, SHADOW_SPRITE, PARTICLE2_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const ORBIT_RADIUS: f32 = 15.0;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub parent_duration_frames: f32,
    pub spawn_period_frames: u32,
    pub angle_deg_per_frame: f32,
    pub angle_divisor: f32,
    pub upper: ParticleParams,
    pub lower: Option<ParticleParams>,
}

#[derive(Clone, Copy, Debug)]
pub struct ParticleParams {
    pub sprite: &'static str,
    pub duration_frames: f32,
    /// Y offset above ground at spawn (negative = up; native RO uses -Y = up).
    pub y_offset: f32,
    pub y_accel_per_frame: f32,
    pub size: f32,
    pub size_speed_per_frame: f32,
    pub alpha_init: f32,
    pub alpha_speed_per_frame: f32,
    /// `f32::MAX` disables the late-life fadeout.
    pub fadeout_start_frame: f32,
    pub anim_ticks: f32,
}

pub const SIGHT: Params = Params {
    parent_duration_frames: 162.0,
    spawn_period_frames: 2,
    angle_deg_per_frame: -5.0,
    angle_divisor: 1.0,
    upper: ParticleParams {
        sprite: SIGHT_SPRITE,
        duration_frames: 20.0,
        y_offset: -20.0,
        y_accel_per_frame: 0.0,
        size: 2.5,
        size_speed_per_frame: -0.1,
        alpha_init: 150.0 / 255.0,
        alpha_speed_per_frame: -3.0 / 255.0,
        fadeout_start_frame: f32::MAX,
        anim_ticks: 4.0,
    },
    lower: Some(ParticleParams {
        sprite: SHADOW_SPRITE,
        duration_frames: 20.0,
        y_offset: 0.0,
        y_accel_per_frame: 0.0,
        size: 1.5,
        size_speed_per_frame: -0.06,
        alpha_init: 120.0 / 255.0,
        alpha_speed_per_frame: -1.0 / 255.0,
        fadeout_start_frame: f32::MAX,
        anim_ticks: 4.0,
    }),
};

pub const RUWACH: Params = Params {
    parent_duration_frames: PERSISTENT_FRAMES,
    spawn_period_frames: 3,
    angle_deg_per_frame: -5.0,
    angle_divisor: 1.3,
    upper: ParticleParams {
        sprite: PARTICLE2_SPRITE,
        duration_frames: 25.0,
        y_offset: -10.0,
        y_accel_per_frame: 0.0,
        size: 3.0,
        size_speed_per_frame: -0.1,
        alpha_init: 250.0 / 255.0,
        alpha_speed_per_frame: -3.0 / 255.0,
        fadeout_start_frame: 19.0,
        anim_ticks: 4.0,
    },
    lower: Some(ParticleParams {
        sprite: SHADOW_SPRITE,
        duration_frames: 25.0,
        y_offset: 0.0,
        y_accel_per_frame: 0.0,
        size: 1.5,
        size_speed_per_frame: -0.06,
        alpha_init: 150.0 / 255.0,
        alpha_speed_per_frame: -2.5 / 255.0,
        fadeout_start_frame: 19.0,
        anim_ticks: 4.0,
    }),
};

pub const SIGHT2: Params = Params {
    parent_duration_frames: PERSISTENT_FRAMES,
    spawn_period_frames: 2,
    angle_deg_per_frame: -5.0,
    angle_divisor: 1.0,
    upper: ParticleParams {
        sprite: SIGHT_SPRITE,
        duration_frames: 20.0,
        y_offset: -20.0,
        y_accel_per_frame: 0.1,
        size: 2.5,
        size_speed_per_frame: -0.1,
        alpha_init: 50.0 / 255.0,
        alpha_speed_per_frame: -3.0 / 255.0,
        fadeout_start_frame: f32::MAX,
        anim_ticks: 3.0,
    },
    lower: None,
};

const PERSISTENT_FRAMES: f32 = 9_000_000.0;

pub const fn total_duration_ms(p: &Params) -> u32 {
    let upper = p.upper.duration_frames;
    let lower = match p.lower {
        Some(l) => l.duration_frames,
        None => 0.0,
    };
    let particle_max = if upper > lower { upper } else { lower };
    ((p.parent_duration_frames + particle_max) / FRAMES_PER_SECOND * 1000.0) as u32
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    sprite: &'static str,
    anchor: [f32; 3],
    offset: [f32; 3],
    y_velocity_per_frame: f32,
    y_accel_per_frame: f32,
    size: f32,
    size_speed_per_frame: f32,
    alpha: f32,
    alpha_speed_per_frame: f32,
    fadeout_start_frame: f32,
    anim_ticks: f32,
    age_frames: f32,
    lifetime_frames: f32,
}

impl Particle {
    fn alive(&self) -> bool {
        self.age_frames < self.lifetime_frames && self.alpha > 0.0 && self.size > 0.0
    }

    fn step(&mut self, dt_frames: f32) {
        self.y_velocity_per_frame += self.y_accel_per_frame * dt_frames;
        self.offset[1] += self.y_velocity_per_frame * dt_frames;
        self.size = (self.size + self.size_speed_per_frame * dt_frames).max(0.0);
        self.alpha = (self.alpha + self.alpha_speed_per_frame * dt_frames).clamp(0.0, 1.0);
        if self.age_frames >= self.fadeout_start_frame {
            let cliff_span = (self.lifetime_frames - self.fadeout_start_frame).max(1e-3);
            let t = ((self.age_frames - self.fadeout_start_frame) / cliff_span).clamp(0.0, 1.0);
            self.alpha *= 1.0 - t;
        }
        self.age_frames += dt_frames;
    }

    fn position(&self) -> [f32; 3] {
        [
            self.anchor[0] + self.offset[0],
            self.anchor[1] + self.offset[1],
            self.anchor[2] + self.offset[2],
        ]
    }
}

pub struct OrbitEffect {
    world_pos: [f32; 3],
    params: Params,
    particles: Vec<Particle>,
    age_frames: f32,
    total_duration_s: f32,
    last_spawn_frame: i32,
}

impl OrbitEffect {
    pub fn new(world_pos: [f32; 3], params: Params) -> Self {
        Self {
            world_pos,
            params,
            particles: Vec::new(),
            age_frames: 0.0,
            total_duration_s: total_duration_ms(&params) as f32 / 1000.0,
            last_spawn_frame: -1,
        }
    }

    fn orbit_at(&self, frame: f32) -> (f32, f32) {
        let degree = (frame / self.params.angle_divisor) * self.params.angle_deg_per_frame;
        let rad = degree.to_radians();
        (rad.sin(), rad.cos())
    }

    fn spawn_pair(&mut self, frame: i32) {
        let (sn, cs) = self.orbit_at(frame as f32);
        let particle_from = |pp: &ParticleParams| Particle {
            sprite: pp.sprite,
            anchor: self.world_pos,
            offset: [ORBIT_RADIUS * sn, pp.y_offset, -ORBIT_RADIUS * cs],
            y_velocity_per_frame: 0.0,
            y_accel_per_frame: pp.y_accel_per_frame,
            size: pp.size,
            size_speed_per_frame: pp.size_speed_per_frame,
            alpha: pp.alpha_init,
            alpha_speed_per_frame: pp.alpha_speed_per_frame,
            fadeout_start_frame: pp.fadeout_start_frame,
            anim_ticks: pp.anim_ticks,
            age_frames: 0.0,
            lifetime_frames: pp.duration_frames,
        };
        self.particles.push(particle_from(&self.params.upper));
        if let Some(lower) = &self.params.lower {
            self.particles.push(particle_from(lower));
        }
    }
}

impl Effect for OrbitEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt_frames;

        let current_frame = self.age_frames.floor() as i32;
        if self.age_frames <= self.params.parent_duration_frames {
            let next_frame = self.last_spawn_frame + 1;
            for f in next_frame..=current_frame {
                if f >= 0
                    && f as f32 <= self.params.parent_duration_frames
                    && (f as u32) % self.params.spawn_period_frames == 0
                {
                    self.spawn_pair(f);
                }
            }
            self.last_spawn_frame = current_frame;
        }

        for p in &mut self.particles {
            p.step(dt_frames);
        }
        self.particles.retain(|p| p.alive());

        if self.age_frames / FRAMES_PER_SECOND >= self.total_duration_s && self.particles.is_empty()
        {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            if p.alpha <= 0.0 || p.size <= 0.0 {
                continue;
            }
            let motion = (p.age_frames / p.anim_ticks) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: p.sprite,
                position: p.position(),
                action_index: 0,
                motion_index: motion,
                size_scale: p.size,
                color: [1.0, 1.0, 1.0, p.alpha],
                blend: BlendKind::Alpha,
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

    fn step_one_frame(e: &mut OrbitEffect) {
        e.update(&ctx(1.0 / FRAMES_PER_SECOND));
    }

    #[test]
    fn sight_emits_one_pair_per_period_frame_in_orbit() {
        let mut e = OrbitEffect::new([10.0, 5.0, 20.0], SIGHT);
        step_one_frame(&mut e);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 2, "exactly two particles per spawn");
        let sprites: Vec<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } => Some(*sprite_path),
                _ => None,
            })
            .collect();
        assert!(sprites.contains(&SIGHT_SPRITE), "upper uses sight sprite");
        assert!(sprites.contains(&SHADOW_SPRITE), "lower uses shadow sprite");

        for prim in &list.primitives {
            let EffectPrimitiveDraw::SpriteParticle { position, .. } = prim else {
                unreachable!()
            };
            let dx = position[0] - 10.0;
            let dz = position[2] - 20.0;
            let r = (dx * dx + dz * dz).sqrt();
            assert!(
                (r - ORBIT_RADIUS).abs() < 0.5,
                "particle on radius-15 orbit: r={r}",
            );
        }
    }

    #[test]
    fn ruwach_uses_particle2_sprite_and_period_3() {
        let mut e = OrbitEffect::new([0.0; 3], RUWACH);
        for _ in 0..4 {
            step_one_frame(&mut e);
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(
            list.primitives.len() >= 4,
            "two pairs spawned across frames 0 and 3, got {}",
            list.primitives.len(),
        );
        let used: std::collections::HashSet<&str> = list
            .primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } => Some(*sprite_path),
                _ => None,
            })
            .collect();
        assert!(used.contains(PARTICLE2_SPRITE));
        assert!(used.contains(SHADOW_SPRITE));
        assert!(!used.contains(SIGHT_SPRITE));
    }

    #[test]
    fn particles_fade_and_die_then_effect_ends() {
        let mut e = OrbitEffect::new([0.0; 3], SIGHT);
        let total_frames = SIGHT.parent_duration_frames + SIGHT.upper.duration_frames + 5.0;
        let mut status = EffectStatus::Running;
        for _ in 0..(total_frames as i32) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn sight2_spawns_single_particle_per_period_no_shadow() {
        let mut e = OrbitEffect::new([10.0, 5.0, 20.0], SIGHT2);
        let status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 1, "single particle, no shadow pair");
        let EffectPrimitiveDraw::SpriteParticle {
            sprite_path,
            position,
            ..
        } = &list.primitives[0]
        else {
            panic!("expected SpriteParticle");
        };
        assert_eq!(*sprite_path, SIGHT_SPRITE);
        let dx = position[0] - 10.0;
        let dz = position[2] - 20.0;
        assert!(((dx * dx + dz * dz).sqrt() - ORBIT_RADIUS).abs() < 0.5);
        assert_eq!(status, EffectStatus::Running, "persistent — keeps running");
    }
}
