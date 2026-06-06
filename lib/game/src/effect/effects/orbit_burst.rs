//! Sparkle bursts — shared `particle1.spr` orbiting / spiralling particle
//! pattern used by `EF_SPHERE` (id 72) and
//! `EF_REMOVETRAP` (id 100). Both spawn `particle1` sprites that orbit a
//! vertical axis while their radius and height evolve; they differ only in
//! the spawn schedule and per-particle constants.
//!
//! Observed behaviour:
//! * Sphere — one orbit particle every
//!   10 frames; radius `0.1 + 0.15·f` reversing to `−0.15` at frame 95;
//!   longitude advances 8°/frame (accel `−0.01`); spawned high (30 units up) and
//!   sinking (0.15/frame); size 1.2; 300-frame per-particle life.
//! * RemoveTrap — 12 particles at
//!   frame 0 and 12 at frame 7; radius grows 0.2/frame, random longitude (no
//!   orbit); rises (2.0/frame) then falls (accel +0.08); size
//!   0.8; 50-frame life, fade from frame 35.
//!
//! The orbit-particle integration mirrors `hasteup.rs`'s `OrbitParticle`;
//! this module generalises it (radius growth + mid-life reversal + gravity)
//! and drives two configs.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::seed_from_world;

pub const PARTICLE_SPRITE: &str = "data/sprite/이팩트/particle1";
pub const SPRITES: &[&str] = &[PARTICLE_SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const ANIM_FRAMES_PER_MOTION: f32 = 4.0;

/// Per-variant constants. Distances are in original-game units; `world_scale`
/// ports them uniformly to our smaller world (a character is ~5–8 wu).
#[derive(Clone, Copy, Debug)]
pub struct OrbitBurstParams {
    pub world_scale: f32,
    pub particle_size: f32,
    pub particle_life_frames: f32,
    pub fade_out_at_frames: f32,
    pub radius_init: f32,
    pub radius_speed: f32,
    /// Frame at which `radius_speed` flips sign (Sphere implodes at f95).
    pub radius_reverse_at: Option<f32>,
    pub long_speed_deg: f32,
    pub long_accel_deg: f32,
    /// Initial vertical offset (native RO −Y = up).
    pub y_init: f32,
    pub y_speed: f32,
    pub y_accel: f32,
    pub blend: BlendKind,
}

// ---- EF_SPHERE (72) ---------------------------------------------------------

pub const SPHERE: OrbitBurstParams = OrbitBurstParams {
    world_scale: 0.55,
    particle_size: 1.2,
    particle_life_frames: 300.0,
    fade_out_at_frames: 300.0 - 30.0,
    radius_init: 0.1,
    radius_speed: 0.15,
    radius_reverse_at: Some(95.0),
    long_speed_deg: 8.0,
    long_accel_deg: -0.01,
    y_init: -30.0,
    y_speed: 0.15,
    y_accel: 0.0,
    blend: BlendKind::Additive,
};
/// Longitude starts at 2° for every Sphere particle; the 10-frame spawn stagger
/// (× `long_speed`) is what spreads them around the orbit.
const SPHERE_SPAWN_INTERVAL_FRAMES: f32 = 10.0;
const SPHERE_SPAWN_END_FRAME: f32 = 250.0;
const SPHERE_LONGITUDE_INIT_DEG: f32 = 2.0;
pub const SPHERE_TOTAL_DURATION_MS: u32 =
    ((SPHERE_SPAWN_END_FRAME + SPHERE.particle_life_frames) / FRAMES_PER_SECOND * 1000.0) as u32;

// ---- EF_REMOVETRAP (100) ----------------------------------------------------

pub const REMOVETRAP: OrbitBurstParams = OrbitBurstParams {
    world_scale: 0.55,
    particle_size: 0.8,
    particle_life_frames: 50.0,
    fade_out_at_frames: 35.0,
    radius_init: 0.1,
    radius_speed: 0.35,
    radius_reverse_at: None,
    long_speed_deg: 0.0,
    long_accel_deg: 0.0,
    y_init: 0.0,
    y_speed: -2.0,
    y_accel: 0.08,
    blend: BlendKind::Additive,
};
const REMOVETRAP_BURST_FRAMES: [f32; 2] = [0.0, 7.0];
const REMOVETRAP_PER_BURST: usize = 12;
pub const REMOVETRAP_TOTAL_DURATION_MS: u32 =
    ((REMOVETRAP_BURST_FRAMES[1] + REMOVETRAP.particle_life_frames) / FRAMES_PER_SECOND * 1000.0)
        as u32;

#[derive(Clone, Copy, Debug)]
struct OrbitParticle {
    longitude_deg: f32,
    long_speed_deg: f32,
    radius: f32,
    radius_speed: f32,
    y_offset: f32,
    y_speed: f32,
    age_frames: f32,
    reversed: bool,
}

impl OrbitParticle {
    fn new(p: &OrbitBurstParams, longitude_deg: f32) -> Self {
        Self {
            longitude_deg,
            long_speed_deg: p.long_speed_deg,
            radius: p.radius_init,
            radius_speed: p.radius_speed,
            y_offset: p.y_init,
            y_speed: p.y_speed,
            age_frames: 0.0,
            reversed: false,
        }
    }

    fn step(&mut self, p: &OrbitBurstParams, dt: f32) {
        self.long_speed_deg += p.long_accel_deg * dt;
        self.longitude_deg += self.long_speed_deg * dt;
        if !self.reversed {
            if let Some(at) = p.radius_reverse_at {
                if self.age_frames >= at {
                    self.radius_speed = -self.radius_speed.abs();
                    self.reversed = true;
                }
            }
        }
        self.radius += self.radius_speed * dt;
        self.y_speed += p.y_accel * dt;
        self.y_offset += self.y_speed * dt;
        self.age_frames += dt;
    }

    fn alive(&self, p: &OrbitBurstParams) -> bool {
        self.age_frames < p.particle_life_frames
    }

    fn alpha(&self, p: &OrbitBurstParams) -> f32 {
        if self.age_frames < p.fade_out_at_frames {
            1.0
        } else {
            let span = (p.particle_life_frames - p.fade_out_at_frames).max(1e-3);
            (1.0 - (self.age_frames - p.fade_out_at_frames) / span).clamp(0.0, 1.0)
        }
    }

    fn position(&self, p: &OrbitBurstParams, anchor: [f32; 3]) -> [f32; 3] {
        let (s, c) = self.longitude_deg.to_radians().sin_cos();
        let r = self.radius * p.world_scale;
        [
            anchor[0] + r * s,
            anchor[1] + self.y_offset * p.world_scale,
            anchor[2] + r * c,
        ]
    }

    fn collect(&self, p: &OrbitBurstParams, anchor: [f32; 3], out: &mut EffectDrawList) {
        let a = self.alpha(p);
        if a <= 0.0 {
            return;
        }
        let motion = (self.age_frames / ANIM_FRAMES_PER_MOTION) as usize;
        out.push(EffectPrimitiveDraw::SpriteParticle {
            sprite_path: PARTICLE_SPRITE,
            position: self.position(p, anchor),
            action_index: 0,
            motion_index: motion,
            size_scale: p.particle_size,
            color: [1.0, 1.0, 1.0, a],
            blend: p.blend,
            aim_target: None,
        });
    }
}

/// Cheap deterministic angle generator so a fixed spawn point is stable
/// (LCG over the world-pos seed + particle index).
fn random_longitude(seed: u32, index: usize) -> f32 {
    let mut x = seed.wrapping_add((index as u32).wrapping_mul(2654435761));
    x ^= x >> 13;
    x = x.wrapping_mul(1274126177);
    x ^= x >> 16;
    (x % 360) as f32
}

// ---- EF_SPHERE effect -------------------------------------------------------

pub struct SphereEffect {
    world_pos: [f32; 3],
    particles: Vec<OrbitParticle>,
    age_frames: f32,
    next_spawn_frame: f32,
}

impl SphereEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            particles: vec![OrbitParticle::new(&SPHERE, SPHERE_LONGITUDE_INIT_DEG)],
            age_frames: 0.0,
            next_spawn_frame: SPHERE_SPAWN_INTERVAL_FRAMES,
        }
    }
}

impl Effect for SphereEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt;
        while self.age_frames >= self.next_spawn_frame && self.next_spawn_frame <= SPHERE_SPAWN_END_FRAME {
            self.particles.push(OrbitParticle::new(&SPHERE, SPHERE_LONGITUDE_INIT_DEG));
            self.next_spawn_frame += SPHERE_SPAWN_INTERVAL_FRAMES;
        }
        for p in &mut self.particles {
            p.step(&SPHERE, dt);
        }
        self.particles.retain(|p| p.alive(&SPHERE));
        if self.age_frames >= SPHERE_SPAWN_END_FRAME && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            p.collect(&SPHERE, self.world_pos, out);
        }
    }
}

// ---- EF_REMOVETRAP effect ---------------------------------------------------

pub struct RemoveTrapEffect {
    world_pos: [f32; 3],
    particles: Vec<OrbitParticle>,
    age_frames: f32,
    bursts_fired: usize,
    seed: u32,
}

impl RemoveTrapEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut e = Self {
            world_pos,
            particles: Vec::new(),
            age_frames: 0.0,
            bursts_fired: 0,
            seed: seed_from_world(world_pos),
        };
        e.fire_burst();
        e
    }

    fn fire_burst(&mut self) {
        let base = self.bursts_fired * REMOVETRAP_PER_BURST;
        for i in 0..REMOVETRAP_PER_BURST {
            let lon = random_longitude(self.seed, base + i);
            self.particles.push(OrbitParticle::new(&REMOVETRAP, lon));
        }
        self.bursts_fired += 1;
    }
}

impl Effect for RemoveTrapEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt = ctx.delta * FRAMES_PER_SECOND;
        self.age_frames += dt;
        while self.bursts_fired < REMOVETRAP_BURST_FRAMES.len()
            && self.age_frames >= REMOVETRAP_BURST_FRAMES[self.bursts_fired]
        {
            self.fire_burst();
        }
        for p in &mut self.particles {
            p.step(&REMOVETRAP, dt);
        }
        self.particles.retain(|p| p.alive(&REMOVETRAP));
        if self.bursts_fired >= REMOVETRAP_BURST_FRAMES.len() && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for p in &self.particles {
            p.collect(&REMOVETRAP, self.world_pos, out);
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
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn count_particles(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == PARTICLE_SPRITE))
            .count()
    }

    #[test]
    fn sphere_spawns_more_particles_over_time() {
        let mut e = SphereEffect::new([0.0; 3]);
        e.update(&ctx(5.0 / FRAMES_PER_SECOND));
        let early = e.particles.len();
        // ~45 frames later → ~4 more 10-frame spawns.
        for _ in 0..45 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert!(e.particles.len() > early, "more particles after spawns {early} → {}", e.particles.len());
    }

    #[test]
    fn sphere_radius_grows_then_reverses() {
        let mut e = SphereEffect::new([0.0; 3]);
        for _ in 0..90 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let r_before = e.particles[0].radius;
        for _ in 0..40 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let r_after = e.particles[0].radius;
        assert!(e.particles[0].reversed, "radius reversed past frame 95");
        assert!(r_after < r_before, "radius shrinks after reversal {r_before} → {r_after}");
    }

    #[test]
    fn removetrap_fires_two_bursts_of_twelve() {
        let mut e = RemoveTrapEffect::new([3.0, 0.0, 4.0]);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(count_particles(&list.primitives), REMOVETRAP_PER_BURST, "first burst");
        // Past frame 7 the second burst has fired.
        for _ in 0..8 {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        let mut list2 = EffectDrawList::new();
        e.collect_draws(&mut list2, &render_ctx());
        assert_eq!(count_particles(&list2.primitives), REMOVETRAP_PER_BURST * 2, "both bursts");
    }

    #[test]
    fn removetrap_dies_after_particles_expire() {
        let mut e = RemoveTrapEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..70 {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
