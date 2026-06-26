//! SMA wind-spiral family — `EF_SMA` (551), `EF_SMA2` (552), `EF_STIN3`
//! (555) and `EF_SMA3` (556).
//!
//! A rising spiral ribbon plus a travelling emitter that streams those ribbons
//! along the caster→target heading:
//!
//! * **Sma2** (552): a single ribbon — three live strands (a fourth is
//!   disabled), each a 315° spiral that rises from a
//!   base ring to a tongue-shaped crest (max height 13/11/9, rise angle
//!   55/50/45°, distance 3.4/3.6/3.8). The crest grows over ~90 frames and
//!   each strand spins at `+(n+3)°/frame`, so the three bands separate into
//!   the swirl the column is built from. Additive blue,
//!   tint `(90,90,255)`.
//! * **Sma** (551): a textureless travelling emitter that moves along the
//!   caster→target heading and fires one `Sma2` ribbon every 8 frames.
//!   Cast on self → the emitter barely moves and the ribbons stack into the
//!   rising column.
//! * **Stin3** (555): the same emitter, but spawns an `Sma3`
//!   particle burst every frame → a dense rising mote column.
//! * **Sma3** (556): rising thunder-ball motes — handled by the
//!   shared `particle_up` effect (see factory); the in-stream variant for 555
//!   lives here.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

/// Number of ring nodes per spiral strand.
const E_DIVISION: usize = 21;
/// The spiral sweeps 315°.
const SWEEP_DEG: f32 = 315.0;

const RING_TEXTURE: &str = "ring_blue.tga";
/// `Sma3` alternates two thunder-ball textures.
const PARTICLE_TEXTURES: [&str; 2] = ["thunder_ball_0002.bmp", "thunder_ball_0003.bmp"];

pub const TEXTURES: &[&str] = &[RING_TEXTURE, PARTICLE_TEXTURES[0], PARTICLE_TEXTURES[1]];

/// Tint `(90,90,255)`, additive.
const BAND_TINT: [f32; 3] = [90.0 / 255.0, 90.0 / 255.0, 1.0];
const PARTICLE_TINT: [f32; 3] = [120.0 / 255.0, 120.0 / 255.0, 1.0];

/// Each `Sma2` ribbon lives this long (rise to full crest + hold + fade).
const BAND_DURATION_FRAMES: u32 = 120;
/// `Sma` emits a ribbon every 8 frames; `Stin3` a particle burst every frame.
const BAND_EMIT_PERIOD: u32 = 8;
/// How long the travelling emitter keeps spawning.
const EMIT_FRAMES: u32 = 100;

pub const SMA2_TOTAL_DURATION_MS: u32 =
    ((BAND_DURATION_FRAMES as f32 / FRAMES_PER_SECOND) * 1000.0) as u32;
pub const SMA_TOTAL_DURATION_MS: u32 =
    (((EMIT_FRAMES + BAND_DURATION_FRAMES) as f32 / FRAMES_PER_SECOND) * 1000.0) as u32;
pub const STIN3_TOTAL_DURATION_MS: u32 =
    (((EMIT_FRAMES + 60) as f32 / FRAMES_PER_SECOND) * 1000.0) as u32;

struct Rng(u32);
impl Rng {
    fn from_seed(seed: u32) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9).wrapping_add(1))
    }
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * (self.next_u32() as f32 / u32::MAX as f32)
    }
}

/// One of the three spiral strands of an `SMA2` ribbon.
#[derive(Clone, Copy)]
struct Strand {
    max_height: f32,
    rise_deg: f32,
    distance: f32,
    /// Spiral rotation, advances `+(n+3)°/frame`.
    rot_deg: f32,
    spin_rate: f32,
}

/// A rising 315° spiral ribbon. Owns its three strands and an
/// alpha/lifetime envelope; rendered as a strip of additive `WorldQuad`s.
pub struct Sma2Band {
    center: [f32; 3],
    strands: [Strand; 3],
    process: u32,
    alpha: f32,
    duration: u32,
}

impl Sma2Band {
    /// `tight` selects the smaller-radius variant the emitter alternates to
    /// every 16 frames.
    fn new(center: [f32; 3], tight: bool, duration: u32) -> Self {
        let dist = |loose: f32, narrow: f32| if tight { narrow } else { loose };
        Self {
            center,
            strands: [
                Strand {
                    max_height: 13.0,
                    rise_deg: 55.0,
                    distance: dist(3.4, 2.4),
                    rot_deg: 0.0,
                    spin_rate: 3.0,
                },
                Strand {
                    max_height: 11.0,
                    rise_deg: 50.0,
                    distance: dist(3.6, 2.6),
                    rot_deg: 90.0,
                    spin_rate: 4.0,
                },
                Strand {
                    max_height: 9.0,
                    rise_deg: 45.0,
                    distance: dist(3.8, 2.8),
                    rot_deg: 180.0,
                    spin_rate: 5.0,
                },
            ],
            process: 0,
            alpha: 0.0,
            duration,
        }
    }

    fn step(&mut self) {
        self.process += 1;
        for s in &mut self.strands {
            s.rot_deg = (s.rot_deg + s.spin_rate) % 360.0;
        }
        if self.process >= self.duration.saturating_sub(40) {
            self.alpha = (self.alpha - 5.0).max(0.0);
        } else if self.process < 40 {
            self.alpha = (self.alpha + 5.0).min(180.0);
        }
    }

    fn dead(&self) -> bool {
        self.process >= self.duration && self.alpha <= 0.0
    }

    /// Walk the 21 ring nodes, each at `order + rotation`, base on the ring
    /// and crest raised by the rise angle; emit a quad per segment.
    fn draw(&self, out: &mut EffectDrawList) {
        if self.alpha <= 0.0 {
            return;
        }
        let a = (self.alpha / 255.0).clamp(0.0, 1.0);
        // Crest envelope: the middle nodes are tallest, tapering at both ends
        // (sin-limit = 90 + (i-middle)*9), ramped in over the first 90 frames.
        let middle = (E_DIVISION as i32 - 1) / 2;
        let ramp = ((self.process.min(90) as f32).to_radians()).sin();
        let step_deg = SWEEP_DEG / (E_DIVISION as f32 - 1.0);
        for s in &self.strands {
            let (rc, rs) = (s.rise_deg.to_radians().cos(), s.rise_deg.to_radians().sin());
            let mut prev: Option<([f32; 3], [f32; 3])> = None;
            for order in 0..E_DIVISION {
                let angle = (order as f32 * step_deg + s.rot_deg).to_radians();
                let (ca, sa) = (angle.cos(), angle.sin());
                let base = [
                    self.center[0] + ca * s.distance,
                    self.center[1],
                    self.center[2] + sa * s.distance,
                ];
                let sin_limit = (90.0 + (order as i32 - middle) as f32 * 9.0).to_radians();
                let height = s.max_height * sin_limit.sin() * ramp;
                let rx = rc * height;
                let tip = [base[0] + ca * rx, base[1] - rs * height, base[2] + sa * rx]; // native −Y up
                if let Some((pbase, ptip)) = prev {
                    let tx1 = (order as f32 - 1.0) / E_DIVISION as f32;
                    let tx2 = order as f32 / E_DIVISION as f32;
                    out.push(EffectPrimitiveDraw::WorldQuad {
                        // (prev base, cur base, cur tip, prev tip)
                        corners: [pbase, base, tip, ptip],
                        uv: [[tx1, 1.0], [tx2, 1.0], [tx2, 0.0], [tx1, 0.0]],
                        texture: RING_TEXTURE,
                        color: [BAND_TINT[0], BAND_TINT[1], BAND_TINT[2], a],
                        blend: BlendKind::Additive,
                        no_depth: false,
                    });
                }
                prev = Some((base, tip));
            }
        }
    }
}

/// `EF_SMA2` — one standalone rising spiral ribbon.
pub struct Sma2Effect {
    band: Sma2Band,
    frame_accum: f32,
}

impl Sma2Effect {
    pub fn new(anchor: [f32; 3]) -> Self {
        Self {
            band: Sma2Band::new(anchor, false, BAND_DURATION_FRAMES),
            frame_accum: 0.0,
        }
    }
}

impl Effect for Sma2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.band.step();
        }
        if self.band.dead() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        self.band.draw(out);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SmaKind {
    /// Emit `Sma2` ribbons (551).
    Bands,
    /// Emit `Sma3` particle bursts (555).
    Particles,
}

struct Particle {
    pos: [f32; 3],
    size: f32,
    rise: f32,
    rotation: f32,
    process: i32,
    alpha: f32,
    texture: &'static str,
}

/// A travelling emitter that streams either spiral ribbons (Sma)
/// or rising particle bursts (Stin3) as it moves along the heading.
pub struct SmaEffect {
    kind: SmaKind,
    emitter: [f32; 3],
    step: [f32; 3],
    frame: u32,
    frame_accum: f32,
    bands: Vec<Sma2Band>,
    particles: Vec<Particle>,
    rng: Rng,
}

impl SmaEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], kind: SmaKind) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let len = (dx * dx + dz * dz).sqrt();
        // The emitter advances 0.8/frame along the heading.
        let step = if len > 0.0 {
            [0.8 * dx / len, 0.0, 0.8 * dz / len]
        } else {
            [0.0; 3]
        };
        let seed = from[0].to_bits() ^ to[2].to_bits() ^ 0x51A_3C9D;
        Self {
            kind,
            // Emitter raised to chest height (y - 10, seeded once).
            emitter: [from[0], from[1] - 10.0, from[2]],
            step,
            frame: 0,
            frame_accum: 0.0,
            bands: Vec::new(),
            particles: Vec::new(),
            rng: Rng::from_seed(seed),
        }
    }

    fn spawn_particle_burst(&mut self) {
        // Base recipe seeds 4 motes/frame; the gif column is dense, so the
        // emitter doubles that as it travels.
        for k in 0..8 {
            self.particles.push(Particle {
                pos: [
                    self.emitter[0] + self.rng.range(-4.0, 4.0),
                    self.emitter[1] + 6.0, // near the actor's feet (`-Y` up)
                    self.emitter[2] + self.rng.range(-4.0, 4.0),
                ],
                size: self.rng.range(1.5, 2.5),
                rise: self.rng.range(0.1, 0.25),
                rotation: self.rng.range(0.0, std::f32::consts::TAU),
                process: 0,
                alpha: 0.0,
                texture: PARTICLE_TEXTURES[k % 2],
            });
        }
    }

    fn step_frame(&mut self) {
        let emitting = self.frame < EMIT_FRAMES;
        match self.kind {
            SmaKind::Bands => {
                if emitting && self.frame % BAND_EMIT_PERIOD == 0 {
                    let tight = (self.frame / BAND_EMIT_PERIOD) % 2 == 1;
                    self.bands
                        .push(Sma2Band::new(self.emitter, tight, BAND_DURATION_FRAMES));
                }
            }
            SmaKind::Particles => {
                if emitting {
                    self.spawn_particle_burst();
                }
            }
        }
        self.emitter[0] += self.step[0];
        self.emitter[2] += self.step[2];

        for b in &mut self.bands {
            b.step();
        }
        self.bands.retain(|b| !b.dead());

        for pt in &mut self.particles {
            pt.process += 1;
            pt.pos[1] -= pt.rise; // native −Y = up
            pt.rotation -= 5.0_f32.to_radians();
            if pt.process <= 10 {
                pt.alpha = (pt.alpha + 15.0 / 255.0).min(180.0 / 255.0);
            } else {
                pt.alpha -= 3.0 / 255.0;
            }
        }
        self.particles
            .retain(|pt| !(pt.process > 10 && pt.alpha <= 0.0));
        self.frame += 1;
    }
}

impl Effect for SmaEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.frame_accum -= 1.0;
            self.step_frame();
        }
        let done = self.frame >= EMIT_FRAMES && self.bands.is_empty() && self.particles.is_empty();
        if done {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for b in &self.bands {
            b.draw(out);
        }
        for pt in &self.particles {
            if pt.alpha <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::Billboard {
                pos: pt.pos,
                size: [pt.size, pt.size],
                // Renderer billboard corner order is TL, TR, BL, BR.
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: pt.rotation,
                texture: pt.texture,
                color: [
                    PARTICLE_TINT[0],
                    PARTICLE_TINT[1],
                    PARTICLE_TINT[2],
                    pt.alpha,
                ],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: 1.0 / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn collect(e: &dyn Effect) -> EffectDrawList {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        list
    }

    #[test]
    fn sma2_band_is_additive_multi_quad() {
        let mut e = Sma2Effect::new([0.0; 3]);
        for _ in 0..60 {
            e.update(&ctx());
        }
        let list = collect(&e);
        // 3 strands × (E_DIVISION-1) segments.
        assert!(
            list.primitives.len() > 30,
            "spiral strip: {}",
            list.primitives.len()
        );
        match &list.primitives[0] {
            EffectPrimitiveDraw::WorldQuad { blend, color, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!(color[2] >= color[0], "blue-ish band: {color:?}");
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }

    #[test]
    fn sma_emits_bands_over_time_then_dies() {
        let mut e = SmaEffect::new([0.0; 3], [0.0; 3], SmaKind::Bands);
        for _ in 0..10 {
            e.update(&ctx());
        }
        let early = e.bands.len();
        for _ in 0..30 {
            e.update(&ctx());
        }
        assert!(
            e.bands.len() > early,
            "more bands accumulate: {early} -> {}",
            e.bands.len()
        );
        // Runs out long after emission ends.
        let mut st = EffectStatus::Running;
        for _ in 0..400 {
            st = e.update(&ctx());
        }
        assert_eq!(st, EffectStatus::Dead);
    }

    #[test]
    fn stin3_streams_rising_particles() {
        let mut e = SmaEffect::new([0.0; 3], [0.0; 3], SmaKind::Particles);
        for _ in 0..6 {
            e.update(&ctx());
        }
        let list = collect(&e);
        assert!(
            list.primitives
                .iter()
                .all(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. })),
            "particles render as billboards",
        );
        assert!(!list.primitives.is_empty(), "particles spawned");
    }
}
