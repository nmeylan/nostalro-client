//! `EF_HITDARK` (id 180) and `EF_DARKATTACK` (id 184) — a dark impact: a brief
//! blue rim ring plus a spray of dark-grey debris. Both ids share the
//! same effect; id 184 only
//! differs by a zero parent duration (its debris still outlives that).
//!
//! Reference gif `150-200/180.gif` (dark ring with a bright rim + dark spray).
//! Layout:
//!   * 1× cylinder (`ring_blue.tga`) laid on its side
//!     and aimed along the attacker's facing — a short flared tube that reads
//!     as a vertical rim ring. 10-frame life, fade from frame 5.
//!   * 2× forward debris + 2× gravity-affected backward debris
//!     (`particle1.spr`, RGB ≈ 20/255 dark): random speed `0.6..1.6`,
//!     decelerating; the gravity pair also falls. 6..30-frame lives.
//!
//! The attacker's facing isn't plumbed to a point-anchored effect, so the
//! ring/spray use a fixed heading (`+Z`); the gif frames the ring roughly
//! face-on, so this matches the captured silhouette.

use super::spike_burst::seed_from_world;
use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_TEXTURE: &str = "ring_blue.tga";
pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const TEXTURES: &[&str] = &[RING_TEXTURE];
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const WORLD_SCALE: f32 = 0.3;
const HEADING_RAD: f32 = 0.0;
const Y_OFFSET: f32 = -10.0 * WORLD_SCALE;
const ANIM_FRAMES_PER_MOTION: f32 = 4.0;

// Blue rim ring (cylinder laid on its side).
const RING_DURATION_FRAMES: f32 = 10.0;
const RING_FADE_OUT_AT: f32 = 5.0;
const RING_OUTER: f32 = 10.0 * WORLD_SCALE;
const RING_INNER: f32 = 5.0 * WORLD_SCALE;
const RING_HEIGHT: f32 = 3.5 * WORLD_SCALE;
const RING_MAX_ALPHA: f32 = 1.0;

// Dark debris.
const DARK_RGB: f32 = 20.0 / 255.0;
const PARTICLE_MAX_ALPHA: f32 = 0.8;
const MAX_PARTICLE_LIFE_FRAMES: f32 = 30.0;

pub const TOTAL_DURATION_MS: u32 = (MAX_PARTICLE_LIFE_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / ((1u32 << 24) as f32)
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next() * (hi - lo)
    }
}

#[derive(Clone, Copy)]
struct DarkParticle {
    pos: [f32; 3],
    dir: [f32; 3],
    speed: f32,
    accel: f32,
    vy: f32,
    gravity_accel: f32,
    size: f32,
    age_frames: f32,
    life_frames: f32,
}

impl DarkParticle {
    fn step(&mut self, dt: f32) {
        self.speed = (self.speed + self.accel * dt).max(0.0);
        for k in 0..3 {
            self.pos[k] += self.dir[k] * self.speed * dt;
        }
        self.vy += self.gravity_accel * dt;
        self.pos[1] += self.vy * dt;
        self.age_frames += dt;
    }
    fn alive(&self) -> bool {
        self.age_frames < self.life_frames
    }
    fn alpha(&self) -> f32 {
        (1.0 - self.age_frames / self.life_frames).clamp(0.0, 1.0) * PARTICLE_MAX_ALPHA
    }
}

/// Build one debris particle. `forward` selects the spray hemisphere;
/// `gravity` adds a downward pull (the backward gravity pair).
fn spawn_particle(rng: &mut Lcg, origin: [f32; 3], forward: bool, gravity: bool) -> DarkParticle {
    // Azimuth: forward cone around the heading, backward cone opposite.
    let base = if forward {
        HEADING_RAD
    } else {
        HEADING_RAD + std::f32::consts::PI
    };
    let azimuth = base + rng.range(-40.0, 40.0).to_radians();
    // Elevation biased upward (`−90+40+random(100)` → −10..+90°).
    let elevation = rng.range(-10.0, 50.0).to_radians();
    let (sa, ca) = azimuth.sin_cos();
    let (se, ce) = elevation.sin_cos();
    // Native RO: −Y = up, so upward elevation is negative Y.
    let dir = [ce * sa, -se, ce * ca];
    let life = rng.range(6.0, MAX_PARTICLE_LIFE_FRAMES);
    let speed = rng.range(0.6, 1.6) * WORLD_SCALE;
    DarkParticle {
        pos: origin,
        dir,
        speed,
        accel: -speed / (2.0 * life),
        vy: 0.0,
        gravity_accel: if gravity {
            rng.range(0.3, 1.2) * WORLD_SCALE / life
        } else {
            0.0
        },
        size: rng.range(0.6, 1.6),
        age_frames: 0.0,
        life_frames: life,
    }
}

pub struct HitDarkEffect {
    world_pos: [f32; 3],
    particles: Vec<DarkParticle>,
    age_frames: f32,
}

impl HitDarkEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let origin = [world_pos[0], world_pos[1] + Y_OFFSET, world_pos[2]];
        let mut rng = Lcg(seed_from_world(world_pos) ^ 0x5151_5151);
        let mut particles = Vec::with_capacity(4);
        for _ in 0..2 {
            particles.push(spawn_particle(&mut rng, origin, true, false));
        }
        for _ in 0..2 {
            particles.push(spawn_particle(&mut rng, origin, false, true));
        }
        Self {
            world_pos,
            particles,
            age_frames: 0.0,
        }
    }

    fn ring_alpha(&self) -> f32 {
        let f = self.age_frames;
        if f >= RING_DURATION_FRAMES {
            return 0.0;
        }
        if f <= RING_FADE_OUT_AT {
            RING_MAX_ALPHA
        } else {
            RING_MAX_ALPHA
                * ((RING_DURATION_FRAMES - f) / (RING_DURATION_FRAMES - RING_FADE_OUT_AT)).max(0.0)
        }
    }
}

impl Effect for HitDarkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt;
        for p in &mut self.particles {
            p.step(dt);
        }
        self.particles.retain(|p| p.alive());
        if self.particles.is_empty() && self.age_frames >= RING_DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let ring_a = self.ring_alpha();
        if ring_a > 0.0 {
            out.push(EffectPrimitiveDraw::Cylinder {
                base: [
                    self.world_pos[0],
                    self.world_pos[1] + Y_OFFSET,
                    self.world_pos[2],
                ],
                bottom_size: RING_OUTER,
                top_size: RING_INNER,
                height: RING_HEIGHT,
                sides: 16,
                rotation: 0.0,
                tilt_x_rad: -std::f32::consts::FRAC_PI_2,
                rotation_y_rad: HEADING_RAD,
                uv_scroll: [0.0, 0.0],
                texture: RING_TEXTURE,
                color: [1.0, 1.0, 1.0, ring_a],
                blend: BlendKind::Additive,
            });
        }
        for p in &self.particles {
            let a = p.alpha();
            if a <= 0.0 {
                continue;
            }
            let motion = (p.age_frames / ANIM_FRAMES_PER_MOTION) as usize;
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: PARTICLE_SPRITE,
                position: p.pos,
                action_index: 0,
                motion_index: motion,
                size_scale: p.size,
                color: [DARK_RGB, DARK_RGB, DARK_RGB, a],
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

    #[test]
    fn emits_ring_plus_four_dark_particles_at_spawn() {
        let mut e = HitDarkEffect::new([5.0, 0.0, 6.0]);
        e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let rings = list.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { texture, .. } if *texture == RING_TEXTURE)).count();
        let parts = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { .. }))
            .count();
        assert_eq!(rings, 1);
        assert_eq!(parts, 4);
    }

    #[test]
    fn ring_fades_out_before_particles() {
        let mut e = HitDarkEffect::new([0.0; 3]);
        // Past the ring's 10-frame life the ring is gone but debris remains.
        for _ in 0..12 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(e.ring_alpha(), 0.0);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let rings = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
            .count();
        assert_eq!(rings, 0, "ring gone");
    }

    #[test]
    fn dies_after_debris_expires() {
        let mut e = HitDarkEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(MAX_PARTICLE_LIFE_FRAMES as i32 + 2) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
