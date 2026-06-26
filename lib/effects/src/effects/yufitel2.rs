//! `EF_YUFITEL2` — Jupiter Thunder hit, ball variant (enum id 452).
//!
//! At the hit point we spawn two additive billboards, matching the
//! original game's on-screen look:
//!
//! * a **sustained animated thunder ball** (frame 10 onward, ~300-frame life):
//!   a 5-texture cycle `twirl_soft / thunder_ball_b / twirl_soft /
//!   thunder_ball_c / twirl_soft` at one texture per tick, 7.5×7.5, additive —
//!   reads as the soft glowing core.
//! * a **periodic spark burst** (`pokjuk_d.bmp`): re-spawned every 20 ticks,
//!   each living ~10 ticks — the colourful sparkle that flashes around the
//!   core in the reference.
//!
//! Unlike `Yufitel` (93) this is a stationary hit effect (no caster→target
//! travel), so it anchors at a single point.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const BALL_TEXTURES: &[&str] = &[
    "twirl_soft.bmp",
    "thunder_ball_b.bmp",
    "twirl_soft.bmp",
    "thunder_ball_c.bmp",
    "twirl_soft.bmp",
];
const BURST_TEXTURE: &str = "pokjuk_d.bmp";
pub const TEXTURES: &[&str] = &[
    "twirl_soft.bmp",
    "thunder_ball_b.bmp",
    "thunder_ball_c.bmp",
    BURST_TEXTURE,
];

const FPS: f32 = 60.0;
/// Whole effect lives 250 frames (matches the reference duration).
const TOTAL_FRAMES: f32 = 250.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

/// The animated core appears 10 ticks after spawn (the first burst flashes
/// alone before it).
const BALL_START_FRAME: f32 = 10.0;
const BALL_SIZE: f32 = 7.5;
const Y_OFFSET: f32 = -5.0;

/// One texture step per tick.
const BALL_FRAMES_PER_STEP: f32 = 1.0;

const BALL_FADE_IN_FRAMES: f32 = 10.0;
const BALL_FADE_OUT_FRAMES: f32 = 30.0;

/// A fresh `pokjuk_d` spark is launched every 20 ticks and lives 10.
const BURST_PERIOD_FRAMES: f32 = 20.0;
const BURST_LIFE_FRAMES: f32 = 10.0;
const BURST_SIZE: f32 = 8.0;

const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

pub struct Yufitel2Effect {
    pos: [f32; 3],
    age_frames: f32,
}

impl Yufitel2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            pos: [world_pos[0], world_pos[1] + Y_OFFSET, world_pos[2]],
            age_frames: 0.0,
        }
    }

    fn ball_alpha(&self) -> f32 {
        let t = self.age_frames - BALL_START_FRAME;
        if t < 0.0 {
            return 0.0;
        }
        let fade_out_start = TOTAL_FRAMES - BALL_FADE_OUT_FRAMES;
        if self.age_frames >= fade_out_start {
            ((TOTAL_FRAMES - self.age_frames) / BALL_FADE_OUT_FRAMES).clamp(0.0, 1.0)
        } else {
            (t / BALL_FADE_IN_FRAMES).clamp(0.0, 1.0)
        }
    }

    fn ball_texture(&self) -> &'static str {
        let t = (self.age_frames - BALL_START_FRAME).max(0.0);
        let step = (t / BALL_FRAMES_PER_STEP) as usize;
        BALL_TEXTURES[step % BALL_TEXTURES.len()]
    }

    /// Alpha of the current spark burst, or 0 when none is active. Bursts
    /// launch at frames 0, 20, 40, … and fade linearly over their 10-frame
    /// life.
    fn burst_alpha(&self) -> f32 {
        let phase = self.age_frames % BURST_PERIOD_FRAMES;
        if phase < BURST_LIFE_FRAMES {
            1.0 - phase / BURST_LIFE_FRAMES
        } else {
            0.0
        }
    }
}

impl Effect for Yufitel2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let burst_alpha = self.burst_alpha();
        if burst_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::Billboard {
                pos: self.pos,
                size: [BURST_SIZE, BURST_SIZE],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: BURST_TEXTURE,
                color: [1.0, 1.0, 1.0, burst_alpha],
                blend: BlendKind::Additive,
            });
        }

        let ball_alpha = self.ball_alpha();
        if ball_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::Billboard {
                pos: self.pos,
                size: [BALL_SIZE, BALL_SIZE],
                uv: UNIT_UV,
                rotation: 0.0,
                texture: self.ball_texture(),
                color: [1.0, 1.0, 1.0, ball_alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut Yufitel2Effect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn draws(e: &Yufitel2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn textures(prims: &[EffectPrimitiveDraw]) -> Vec<&'static str> {
        prims
            .iter()
            .map(|p| match p {
                EffectPrimitiveDraw::Billboard { texture, .. } => *texture,
                other => panic!("expected Billboard, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn first_burst_flashes_before_the_core_appears() {
        // Sociable: at frame 5 only the spark burst is up (the animated core
        // starts at frame 10); both blend additively.
        let mut e = Yufitel2Effect::new([0.0, 0.0, 0.0]);
        step(&mut e, 5.0);
        let prims = draws(&e);
        assert_eq!(textures(&prims), vec![BURST_TEXTURE]);
        assert!(matches!(
            prims[0],
            EffectPrimitiveDraw::Billboard {
                blend: BlendKind::Additive,
                ..
            }
        ));
    }

    #[test]
    fn core_animates_through_its_texture_cycle_after_frame_10() {
        // Once past frame 10 the core is present and steps one texture per
        // tick; sampling consecutive ticks walks the 5-frame cycle.
        let mut e = Yufitel2Effect::new([0.0, 0.0, 0.0]);
        step(&mut e, BALL_START_FRAME + 0.5);
        let mut seen = Vec::new();
        for _ in 0..BALL_TEXTURES.len() {
            // The core is the last draw (burst, when present, comes first).
            let prims = draws(&e);
            seen.push(*textures(&prims).last().unwrap());
            step(&mut e, 1.0);
        }
        assert_eq!(seen, BALL_TEXTURES);
    }

    #[test]
    fn burst_recurs_on_its_period_and_fades_within_its_life() {
        let mut e = Yufitel2Effect::new([0.0, 0.0, 0.0]);
        // Just after a period boundary a fresh burst is bright…
        step(&mut e, BURST_PERIOD_FRAMES + 0.5);
        let fresh = e.burst_alpha();
        // …and past its 10-frame life it has gone.
        step(&mut e, BURST_LIFE_FRAMES);
        let gone = e.burst_alpha();
        assert!(fresh > 0.5, "fresh burst bright: {fresh}");
        assert!(gone <= 0.0, "burst expired: {gone}");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = Yufitel2Effect::new([0.0, 0.0, 0.0]);
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
