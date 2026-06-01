//! Rising-sparkle status effects — Hptime/Sptime,
//! Hated/Hated2, SmaReady, Sprinklesand (ids 331, 332, 543, 572, 546, 310).
//!
//! HealTime / ParticleTime / Sprinklesand repeatedly launch bursts of 4
//! sparkle billboards
//! spawned every 4 frames over a window, scattered around the actor, drifting
//! upward, spinning, fading in over 10 frames then
//! out. A colour index selects the tint. The textures are hardcoded
//! (`pok1.tga`; `thunder_center.bmp` for the
//! Sprinklesand burst).
//!
//! Firstaid uses a different primitive and is deferred.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const SPAWN_PERIOD: u32 = 4;
/// Particles spawn `y_offset` above the actor's feet (native RO `-Y = up`,
/// 6 units up).
const Y_OFFSET: f32 = -6.0;

fn rgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]
}

#[derive(Clone, Copy)]
pub struct ParticleUpParams {
    pub texture: &'static str,
    pub tint_rgb: (u8, u8, u8),
    /// Frame window in which bursts spawn.
    pub spawn_start: u32,
    pub spawn_end: u32,
    /// Sub-emitters per burst; each emits 4 particles (×4 below).
    pub prims_per_spawn: usize,
    /// Horizontal scatter half-range around the actor.
    pub spread: f32,
    pub base_dist: f32,
    pub dist_rand: f32,
    pub rise_base: f32,
    pub rise_rand: f32,
    /// Sprinklesand staggers starts with `process=-10+rand(10)`.
    pub stagger_start: bool,
}

const fn p(texture: &'static str, tint_rgb: (u8, u8, u8)) -> ParticleUpParams {
    ParticleUpParams {
        texture,
        tint_rgb,
        spawn_start: 0,
        spawn_end: 20,
        prims_per_spawn: 1,
        spread: 3.0,
        base_dist: 2.5,
        dist_rand: 1.5,
        rise_base: 0.2,
        rise_rand: 0.2,
        stagger_start: false,
    }
}

// HealTime — short window (0-20), tighter scatter.
pub const HPTIME: ParticleUpParams = p("pok1.tga", (220, 250, 220)); // pale green (HP)
pub const SPTIME: ParticleUpParams = p("pok1.tga", (150, 150, 250)); // blue (SP)
// Hated — a wide field of many small twinkling sparkles (firefly look),
// spawned over a long window. Small `pok1.tga` stars, wide horizontal scatter.
pub const HATED: ParticleUpParams = ParticleUpParams {
    spawn_end: 80,
    prims_per_spawn: 2, // denser field
    spread: 5.0,
    base_dist: 1.2,
    dist_rand: 0.8,
    rise_base: 0.1,
    rise_rand: 0.3,
    ..p("pok1.tga", (150, 150, 250))
};
pub const HATED2: ParticleUpParams = ParticleUpParams { tint_rgb: (250, 100, 100), ..HATED }; // red
pub const SMAREADY: ParticleUpParams = ParticleUpParams {
    spawn_start: 40,
    spawn_end: 120,
    ..HATED
};
// Sprinklesand — one 20-prim burst, small sparkles, staggered.
pub const SPRINKLESAND: ParticleUpParams = ParticleUpParams {
    spawn_end: 0,
    prims_per_spawn: 20,
    spread: 4.0,
    base_dist: 0.8,
    dist_rand: 1.2,
    rise_base: 0.2,
    rise_rand: 0.4,
    stagger_start: true,
    ..p("thunder_center.bmp", (250, 250, 150)) // yellow
};

// Sma3 (556) — one blue thunder-ball burst rising
// from the actor (blue palette, `thunder_ball_0002.bmp`).
pub const SMA3: ParticleUpParams = ParticleUpParams {
    spawn_end: 0,
    spread: 4.0,
    base_dist: 1.2,
    dist_rand: 0.8,
    rise_base: 0.1,
    rise_rand: 0.2,
    ..p("thunder_ball_0002.bmp", (120, 120, 255))
};
/// Single burst that fades in over 10 frames then out at `-3/255`/frame.
pub const SMA3_TOTAL_DURATION_MS: u32 = 1100;

pub const TEXTURES: &[&str] = &["pok1.tga", "thunder_center.bmp", "thunder_ball_0002.bmp"];

struct Rng(u32);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

struct Particle {
    pos: [f32; 3],
    size: f32,
    rise: f32,
    rotation: f32,
    process: i32,
    alpha: f32,
}

pub struct ParticleUpEffect {
    params: ParticleUpParams,
    center: [f32; 3],
    rng: Rng,
    particles: Vec<Particle>,
    frame: u32,
    frame_accum: f32,
}

impl ParticleUpEffect {
    pub fn new(anchor: [f32; 3], params: ParticleUpParams) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ 0x9A1C_0FF1;
        Self {
            params,
            center: anchor,
            rng: Rng(seed | 1),
            particles: Vec::new(),
            frame: 0,
            frame_accum: 0.0,
        }
    }

    fn spawn_burst(&mut self) {
        for _ in 0..self.params.prims_per_spawn * 4 {
            let process = if self.params.stagger_start {
                -10 + (self.rng.next_u32() % 10) as i32
            } else {
                0
            };
            self.particles.push(Particle {
                pos: [
                    self.center[0] + self.rng.range(-self.params.spread, self.params.spread),
                    self.center[1] + Y_OFFSET,
                    self.center[2] + self.rng.range(-self.params.spread, self.params.spread),
                ],
                size: self.params.base_dist + self.rng.range(0.0, self.params.dist_rand),
                rise: self.params.rise_base + self.rng.range(0.0, self.params.rise_rand),
                rotation: self.rng.range(0.0, std::f32::consts::TAU),
                process,
                alpha: 0.0,
            });
        }
    }

    fn step_frame(&mut self) {
        if self.frame >= self.params.spawn_start
            && self.frame <= self.params.spawn_end
            && (self.frame - self.params.spawn_start) % SPAWN_PERIOD == 0
        {
            self.spawn_burst();
        }
        for pt in &mut self.particles {
            pt.process += 1;
            if pt.process > 0 {
                pt.pos[1] -= pt.rise; // native -Y = up
                pt.rotation -= 5.0_f32.to_radians();
                if pt.process <= 10 {
                    pt.alpha = (pt.alpha + 15.0 / 255.0).min(150.0 / 255.0);
                } else {
                    pt.alpha -= 3.0 / 255.0;
                }
            }
        }
        self.particles.retain(|pt| !(pt.process > 10 && pt.alpha <= 0.0));
        self.frame += 1;
    }
}

impl Effect for ParticleUpEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        // Done once spawning has ended and every particle has faded.
        if self.frame > self.params.spawn_end && self.particles.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let (r, g, b) = self.params.tint_rgb;
        let tint = rgb(r, g, b);
        for pt in &self.particles {
            if pt.alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: pt.pos,
                size: [pt.size, pt.size],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: pt.rotation,
                texture: self.params.texture,
                color: [tint[0], tint[1], tint[2], pt.alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut ParticleUpEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None });
        }
        st
    }

    fn billboards(e: &ParticleUpEffect) -> Vec<([f32; 3], [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { pos, color, blend: BlendKind::Additive, .. } => (*pos, *color),
                _ => panic!("expected additive Billboard sparkles"),
            })
            .collect()
    }

    #[test]
    fn spawns_bursts_and_tints_each_variant() {
        let mut e = ParticleUpEffect::new([0.0; 3], HPTIME);
        tick(&mut e, 12); // past first burst's fade-in
        let bb = billboards(&e);
        assert!(!bb.is_empty(), "sparkles spawned");
        // green channel dominates blue for the HP variant.
        let (_, c) = bb[0];
        assert!(c[1] > c[2], "HP sparkles are greenish: {c:?}");
    }

    #[test]
    fn particles_rise_and_effect_eventually_dies() {
        let mut e = ParticleUpEffect::new([0.0, 0.0, 0.0], SPTIME);
        tick(&mut e, 6);
        let y_early = billboards(&e).iter().map(|(p, _)| p[1]).sum::<f32>()
            / billboards(&e).len().max(1) as f32;
        tick(&mut e, 8);
        let bb = billboards(&e);
        if !bb.is_empty() {
            let y_late = bb.iter().map(|(p, _)| p[1]).sum::<f32>() / bb.len() as f32;
            assert!(y_late < y_early, "particles drift up (native -Y): {y_early} -> {y_late}");
        }
        assert_eq!(tick(&mut e, 400), EffectStatus::Dead);
    }
}
