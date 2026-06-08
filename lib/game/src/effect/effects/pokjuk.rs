//! `EF_POKJUK` (id 297) — a firecracker: colored sparks burst overhead and
//! drift away, fading.
//!
//! The original game launches four particles,
//! each staggered (`process = -300 - random(50)`). A particle
//! is invisible while it rises (alpha 0), then **bursts** (alpha 250)
//! and drifts along a random direction: `y += sin(elevation)·distance`
//! plus a horizontal slide along the heading, `distance *= 0.98` and alpha
//! drains ~1–2/frame while rotation `+= 5°` tumbles the quad. A palette index
//! 0..4 tints the spark blue / red / green / yellow / magenta; the quad is a
//! camera-facing `pok1/2/3.tga` billboard.
//!
//! Only the burst is ever drawn, so this models the four bursts directly
//! (skipping the invisible rise) at a point above the caster, staggered. The
//! long dormancy/loop (a persistent festival firecracker) is
//! compressed to a single visible burst sequence. No reference gif — validated
//! against observed original-game behaviour.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const NUM_SPARKS: usize = 4;
/// Staggered launch + the alpha drain (~250 → 0 at ~1.5/frame) ≈ 170 frames.
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub const TEXTURES: &[&str] = &["pok1.tga", "pok2.tga", "pok3.tga"];
const SPARK_TEXTURES: [&str; 3] = ["pok1.tga", "pok2.tga", "pok3.tga"];

/// Spark palette (blue / red / green / yellow / magenta).
const COLORS: [[f32; 3]; 5] = [
    [70.0 / 255.0, 70.0 / 255.0, 1.0],
    [1.0, 70.0 / 255.0, 70.0 / 255.0],
    [70.0 / 255.0, 1.0, 70.0 / 255.0],
    [1.0, 1.0, 90.0 / 255.0],
    [1.0, 70.0 / 255.0, 1.0],
];

/// Burst origin above and to the side of the caster (the risen rocket point;
/// ~15 to the side and ~38 units up). Native `-Y = up`.
const ORIGIN_X: f32 = -7.0;
const ORIGIN_UP: f32 = 14.0;
const LAUNCH_STAGGER_FRAMES: f32 = 8.0;
const SPARK_SIZE: f32 = 2.0;
const START_ALPHA: f32 = 250.0 / 255.0;
const ALPHA_DRAIN_PER_FRAME: f32 = 1.5 / 255.0;
const DRIFT_UP_PER_FRAME: f32 = 0.02;
const SHRINK_PER_FRAME: f32 = 0.98;
const TUMBLE_DEG_PER_FRAME: f32 = 5.0;

struct Spark {
    color: [f32; 3],
    texture: &'static str,
    launch_delay: f32,
    /// Vertical/horizontal split of the eject.
    elevation: f32,
    /// Horizontal heading.
    heading: f32,
    distance: f32,
    pos: [f32; 3],
    rotation: f32,
    alpha: f32,
    bursting: bool,
}

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

pub struct PokjukEffect {
    sparks: Vec<Spark>,
    frame: f32,
}

impl PokjukEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let [cx, cy, cz] = world_pos;
        let seed = (cx * 41.0 + cz * 97.0) as i64 as u32 ^ 0x2468_ACE0;
        let mut rng = Rng(seed | 1);
        let origin = [cx + ORIGIN_X, cy - ORIGIN_UP, cz];
        let sparks = (0..NUM_SPARKS)
            .map(|i| Spark {
                color: COLORS[(rng.next_u32() % 5) as usize],
                texture: SPARK_TEXTURES[(rng.next_u32() % 3) as usize],
                launch_delay: i as f32 * LAUNCH_STAGGER_FRAMES,
                elevation: rng.range(0.0, 360.0),
                heading: rng.range(0.0, 360.0),
                distance: rng.range(0.4, 0.8),
                pos: origin,
                rotation: rng.range(0.0, 360.0),
                alpha: 0.0,
                bursting: false,
            })
            .collect();
        Self { sparks, frame: 0.0 }
    }
}

impl Effect for PokjukEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.frame += frames;
        for s in &mut self.sparks {
            if self.frame < s.launch_delay {
                continue;
            }
            if !s.bursting {
                s.bursting = true;
                s.alpha = START_ALPHA;
            }
            let elev = s.elevation.to_radians();
            let head = s.heading.to_radians();
            let radial = elev.cos() * s.distance;
            // Vertical eject (native -Y up) + small upward drift.
            s.pos[1] -= (elev.sin() * s.distance + DRIFT_UP_PER_FRAME) * frames;
            s.pos[0] += head.cos() * radial * frames;
            s.pos[2] += head.sin() * radial * frames;
            s.distance *= SHRINK_PER_FRAME.powf(frames);
            s.rotation = (s.rotation + TUMBLE_DEG_PER_FRAME * frames) % 360.0;
            s.alpha = (s.alpha - ALPHA_DRAIN_PER_FRAME * frames).max(0.0);
        }
        if self.frame >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for s in &self.sparks {
            if !s.bursting || s.alpha <= 0.0 {
                continue;
            }
            let [r, g, b] = s.color;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: s.pos,
                size: [SPARK_SIZE, SPARK_SIZE],
                uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
                rotation: s.rotation.to_radians(),
                texture: s.texture,
                color: [r, g, b, s.alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn tick(e: &mut PokjukEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        st
    }

    fn draws(e: &PokjukEffect) -> Vec<EffectPrimitiveDraw> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives
    }

    #[test]
    fn sparks_launch_staggered() {
        let mut e = PokjukEffect::new([0.0; 3]);
        tick(&mut e, 2);
        let early = draws(&e).len();
        tick(&mut e, NUM_SPARKS as u32 * LAUNCH_STAGGER_FRAMES as u32 + 2);
        let all = draws(&e).len();
        assert!(early < NUM_SPARKS, "not all sparks burst at frame 0 ({early})");
        assert_eq!(all, NUM_SPARKS, "all sparks bursting once staggered in");
    }

    #[test]
    fn spark_drifts_and_alpha_drains() {
        let mut e = PokjukEffect::new([0.0; 3]);
        tick(&mut e, 5);
        let (p0, a0) = first(&e);
        tick(&mut e, 40);
        let (p1, a1) = first(&e);
        let moved = (p0[0] - p1[0]).abs() + (p0[1] - p1[1]).abs() + (p0[2] - p1[2]).abs();
        assert!(moved > 1e-4, "spark drifts");
        assert!(a1 < a0, "alpha drains ({a0} → {a1})");
    }

    #[test]
    fn sparks_carry_varied_colors_and_textures() {
        let e = PokjukEffect::new([5.0, 0.0, 9.0]);
        let textures: std::collections::BTreeSet<&str> = e.sparks.iter().map(|s| s.texture).collect();
        assert!(textures.iter().all(|t| TEXTURES.contains(t)));
        // Colours come from the 5-entry palette.
        assert!(e.sparks.iter().all(|s| COLORS.contains(&s.color)));
    }

    #[test]
    fn self_terminates() {
        let mut e = PokjukEffect::new([0.0; 3]);
        assert_eq!(tick(&mut e, TOTAL_FRAMES as u32 + 2), EffectStatus::Dead);
    }

    fn first(e: &PokjukEffect) -> ([f32; 3], f32) {
        match &draws(e)[0] {
            EffectPrimitiveDraw::Billboard { pos, color, .. } => (*pos, color[3]),
            _ => panic!(),
        }
    }
}
