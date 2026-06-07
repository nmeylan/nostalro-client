//! `EF_COLORPAPER` (id 347) — falling confetti, the original game's wedding
//! effect.
//!
//! 50 bursts of 4 emitters → 200 confetti chips total. Per chip the
//! behaviour is:
//! it spawns high above the caster
//! (`y = -(110..160)`, `x,z ∈ [-50,50]`), falls at 0.25–0.51/frame,
//! tumbles (rotation +2°/frame), random RGB, fades in over 60 frames, fades
//! out near the ground (`y > -10`), then respawns up to 3 times before dying.
//!
//! Each chip draws as **one** tumbling square quad (4 corners
//! 90° apart at the chip radius, tilted around world-X by the tumble angle) —
//! it flickers edge-on as it tumbles. Alpha blend; the per-chip
//! RGB is randomised per chip.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TEXTURES: &[&str] = &["white02.bmp"];
const TEXTURE: &str = "white02.bmp";

const CHIP_COUNT: usize = 200;

/// Horizontal spread (±50) and drop height (110–160) are independent dimensions
/// for a confetti field — the cloud is far wider than it is tall. Scale them
/// separately: a wide ±15-unit scatter dropping from ~14–21 units overhead.
const HORIZ_SCALE: f32 = 0.45;
const POS_SCALE: f32 = 0.13;
/// Chip half-size is an independent dimension from the fall height — confetti
/// chips stay small specks regardless of how high they fall from.
const CHIP_SCALE: f32 = 1.5;

const MAX_RESPAWNS: u8 = 3;
const FADE_IN_FRAMES: i32 = 60;
/// Chips fade once within 10 (scaled) units of the caster's ground plane.
const GROUND_FADE_Y: f32 = -10.0 * POS_SCALE;

const ALPHA_FADE_IN: f32 = 5.0 / 255.0;
const ALPHA_FADE_OUT: f32 = 10.0 / 255.0;
const ALPHA_CAP: f32 = 250.0 / 255.0;

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

struct Chip {
    pos: [f32; 3],
    fall_speed: f32,
    radius: f32,
    rot_deg: f32,
    tilt_deg: f32,
    color: [f32; 3],
    alpha: f32,
    process: i32,
    respawns: u8,
    rng: Rng,
    dead: bool,
}

impl Chip {
    fn spawn_pos(rng: &mut Rng, anchor: [f32; 3]) -> [f32; 3] {
        [
            anchor[0] + (rng.range(0.0, 101.0) - 50.0) * HORIZ_SCALE,
            anchor[1] - (rng.range(0.0, 50.0) + 110.0) * POS_SCALE, // native −Y up
            anchor[2] + (rng.range(0.0, 101.0) - 50.0) * HORIZ_SCALE,
        ]
    }

    fn step(&mut self, anchor: [f32; 3]) {
        if self.dead {
            return;
        }
        self.process += 1;
        self.tilt_deg = (self.tilt_deg + 2.0) % 360.0;
        self.pos[1] += self.fall_speed; // falling toward the ground (+y)

        if self.pos[1] > GROUND_FADE_Y + anchor[1] {
            self.alpha -= ALPHA_FADE_OUT;
            if self.alpha <= 0.0 {
                self.alpha = 0.0;
                self.respawns += 1;
                if self.respawns < MAX_RESPAWNS {
                    self.pos = Self::spawn_pos(&mut self.rng, anchor);
                    self.process = 0;
                } else if self.pos[1] > anchor[1] {
                    self.dead = true;
                }
            }
        }
        if self.process < FADE_IN_FRAMES {
            self.alpha = (self.alpha + ALPHA_FADE_IN).min(ALPHA_CAP);
        }
    }
}

pub struct ColorpaperEffect {
    anchor: [f32; 3],
    chips: Vec<Chip>,
}

impl ColorpaperEffect {
    pub fn new(anchor: [f32; 3]) -> Self {
        let seed = anchor[0].to_bits() ^ anchor[2].to_bits() ^ 0xC01F_3777;
        let mut master = Rng(seed | 1);
        let chips = (0..CHIP_COUNT)
            .map(|_| {
                let mut rng = Rng(master.next_u32() | 1);
                let pos = Chip::spawn_pos(&mut rng, anchor);
                let fall_speed = (rng.range(0.0, 26.0) * 0.01 + 0.25) * POS_SCALE;
                let radius = (rng.range(0.0, 31.0) * 0.01 + 0.3) * CHIP_SCALE;
                Chip {
                    pos,
                    fall_speed,
                    radius,
                    rot_deg: rng.range(0.0, 360.0),
                    tilt_deg: rng.range(0.0, 4.0) * 45.0, // ec*45
                    color: [rng.range(0.2, 1.0), rng.range(0.2, 1.0), rng.range(0.2, 1.0)],
                    alpha: 0.0,
                    process: 0,
                    respawns: 0,
                    rng,
                    dead: false,
                }
            })
            .collect();
        Self { anchor, chips }
    }
}

impl Effect for ColorpaperEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = (ctx.delta * FRAMES_PER_SECOND).round() as i32;
        for _ in 0..frames.max(0) {
            for chip in &mut self.chips {
                chip.step(self.anchor);
            }
        }
        if self.chips.iter().all(|c| c.dead) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for chip in &self.chips {
            if chip.dead || chip.alpha <= 0.0 {
                continue;
            }
            let tilt = chip.tilt_deg.to_radians();
            let (tc, ts) = (tilt.cos(), tilt.sin());
            let r = chip.radius;
            // 4 corners 90° apart, tilted around world-X as the chip tumbles.
            let corner = |deg_off: f32| {
                let a = (chip.rot_deg + deg_off).to_radians();
                let (ca, sa) = (a.cos(), a.sin());
                [
                    chip.pos[0] + r * ca,
                    chip.pos[1] + r * sa * tc,
                    chip.pos[2] + r * sa * ts,
                ]
            };
            out.push(EffectPrimitiveDraw::WorldQuad {
                corners: [corner(0.0), corner(90.0), corner(180.0), corner(270.0)],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                texture: TEXTURE,
                color: [chip.color[0], chip.color[1], chip.color[2], chip.alpha],
                blend: BlendKind::Alpha,
            });
        }
    }
}

/// Three fall cycles of ~300 frames each ≈ 15 s. Chips self-terminate; this is
/// the holder's upper bound.
pub const TOTAL_DURATION_MS: u32 = 15000;

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 256.0, screen_h: 256.0, elapsed: 0.0 }
    }

    fn tick(e: &mut ColorpaperEffect, frames: u32) -> EffectStatus {
        let mut st = EffectStatus::Running;
        for _ in 0..frames {
            st = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
        st
    }

    fn quads(e: &ColorpaperEffect) -> Vec<([[f32; 3]; 4], [f32; 4])> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::WorldQuad { corners, color, blend: BlendKind::Alpha, .. } => {
                    (*corners, *color)
                }
                other => panic!("expected alpha WorldQuad, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn confetti_spawns_high_with_varied_color() {
        let mut e = ColorpaperEffect::new([0.0, 0.0, 0.0]);
        tick(&mut e, 3.0 as u32);
        let q = quads(&e);
        assert!(q.len() > 100, "a cloud of chips ({})", q.len());
        // Chips start above the caster (native −Y up → negative y).
        let mean_y: f32 = q.iter().map(|(c, _)| c[0][1]).sum::<f32>() / q.len() as f32;
        assert!(mean_y < 0.0, "cloud starts overhead: {mean_y}");
        // Colors vary across chips.
        let c0 = q[0].1;
        assert!(q.iter().any(|(_, c)| (c[0] - c0[0]).abs() > 0.1), "varied RGB");
    }

    #[test]
    fn chips_fall_and_tumble() {
        let mut e = ColorpaperEffect::new([0.0; 3]);
        tick(&mut e, 20);
        let y_early: f32 = quads(&e).iter().map(|(c, _)| c[0][1]).sum();
        let early_corner = quads(&e)[0].0;
        tick(&mut e, 30);
        let y_late: f32 = quads(&e).iter().map(|(c, _)| c[0][1]).sum();
        assert!(y_late > y_early, "chips fall (native +y down): {y_early} -> {y_late}");
        // Tumble changes the quad's z extent over time.
        let late_corner = quads(&e)[0].0;
        assert!(early_corner != late_corner, "chip tumbles");
    }

    #[test]
    fn effect_eventually_dies() {
        let mut e = ColorpaperEffect::new([0.0; 3]);
        // Long enough for all chips to exhaust their 3 fall cycles.
        let st = tick(&mut e, 2000);
        assert_eq!(st, EffectStatus::Dead);
    }
}
