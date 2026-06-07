//! `EF_WATERBALL2` (id 117) — a twisting water column.
//!
//! Reference: the
//! original game gif `100-150/117.gif` — a tall braided blue/green column that
//! swirls upward then sinks. A single swirling particle strand
//! (`data/sprite/이팩트/waterball`, 12 segments,
//! size 0.75) whose head rises while a swirl spins it around a
//! vertical axis; the 12 trailing history points draw the braid.
//!
//! The math integrates a gravity/latitude-speed head with roll; the net
//! visible result is the vertical helix the gif shows, so we drive the head
//! directly as a rise-then-sink (gravity speed) with a constant-radius swirl
//! (lateral speed). The strand renders at the impact point and never depends on a
//! caster→target distance, so it always shows regardless of the anchor.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const SPRITE: &str = "data/sprite/이팩트/waterball";
pub const SPRITES: &[&str] = &[SPRITE];

const FRAMES_PER_SECOND: f32 = 60.0;
const DURATION_FRAMES: u32 = 40;
const NUM_SEGMENT: usize = 12;
const ANIM_SPEED: u32 = 4;
const RENDER_SIZE: f32 = 0.75;

/// Peak rise of the head above the base (native RO −Y = up), ~1.5 characters.
const RISE_AMP: f32 = 9.0;
/// Radius of the swirl helix.
const SWIRL_RADIUS: f32 = 1.3;
/// Helix spin — ~5–6 twists over the column's life like the braided gif.
const SPIN_DEG_PER_FRAME: f32 = 52.0;

pub const TOTAL_DURATION_MS: u32 =
    ((DURATION_FRAMES + NUM_SEGMENT as u32) as f32 / FRAMES_PER_SECOND * 1000.0) as u32;

/// Head offset from the base at a given (head) frame.
fn head_offset(frame: f32) -> [f32; 3] {
    let t = (frame / DURATION_FRAMES as f32).clamp(0.0, 1.0);
    // Rise up then sink back (native RO: up = −Y).
    let y = -RISE_AMP * (std::f32::consts::PI * t).sin();
    let a = (frame * SPIN_DEG_PER_FRAME).to_radians();
    let (s, c) = a.sin_cos();
    [SWIRL_RADIUS * c, y, SWIRL_RADIUS * s]
}

pub struct WaterBall2Effect {
    base: [f32; 3],
    /// History of head offsets (index 0 = newest).
    segments: [[f32; 3]; NUM_SEGMENT],
    head_frame: u32,
    effect_frame: u32,
}

impl WaterBall2Effect {
    pub fn new(_from: [f32; 3], to: [f32; 3]) -> Self {
        // Forms at the impact point.
        Self {
            base: to,
            segments: [head_offset(0.0); NUM_SEGMENT],
            head_frame: 0,
            effect_frame: 0,
        }
    }

    /// Trailing segments currently drawn: fills up, then drains from the tail.
    fn rendered_count(&self) -> usize {
        let f = self.effect_frame as usize;
        if f <= DURATION_FRAMES as usize {
            (f + 1).min(NUM_SEGMENT)
        } else {
            NUM_SEGMENT.saturating_sub(f - DURATION_FRAMES as usize)
        }
    }

    fn step(&mut self) {
        if self.head_frame < DURATION_FRAMES {
            self.head_frame += 1;
            for i in (1..NUM_SEGMENT).rev() {
                self.segments[i] = self.segments[i - 1];
            }
            self.segments[0] = head_offset(self.head_frame as f32);
        }
        self.effect_frame += 1;
    }

    fn world(&self, off: [f32; 3]) -> [f32; 3] {
        [self.base[0] + off[0], self.base[1] + off[1], self.base[2] + off[2]]
    }
}

impl Effect for WaterBall2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let target = ((self.effect_frame as f32) + ctx.delta * FRAMES_PER_SECOND) as u32;
        while self.effect_frame < target {
            self.step();
        }
        if self.rendered_count() == 0 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let motion = (self.head_frame / ANIM_SPEED) as usize;
        let fn_seg = NUM_SEGMENT as f32;
        for i in 0..self.rendered_count() {
            let fi = i as f32;
            let alpha = 1.0 - fi / fn_seg;
            if alpha <= 0.0 {
                continue;
            }
            let size = RENDER_SIZE * (1.0 - fi / (2.0 * fn_seg));
            out.push(EffectPrimitiveDraw::SpriteParticle {
                sprite_path: SPRITE,
                position: self.world(self.segments[i]),
                action_index: 0,
                motion_index: motion,
                size_scale: size,
                color: [1.0, 1.0, 1.0, alpha],
                blend: BlendKind::Additive,
                aim_target: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx { delta: dt, camera_target: None, caster_yaw: None }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn step(e: &mut WaterBall2Effect, n: i32) {
        for _ in 0..n {
            e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
    }

    fn segs(e: &WaterBall2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .into_iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::SpriteParticle { sprite_path, .. } if *sprite_path == SPRITE))
            .collect()
    }

    #[test]
    fn fills_to_twelve_segments_at_impact_point() {
        let mut e = WaterBall2Effect::new([0.0; 3], [4.0, 0.0, 6.0]);
        step(&mut e, 15);
        let s = segs(&e);
        assert_eq!(s.len(), NUM_SEGMENT);
        // Renders around the given impact point, not at the origin.
        if let EffectPrimitiveDraw::SpriteParticle { position, .. } = s[NUM_SEGMENT - 1] {
            assert!((position[0] - 4.0).abs() < SWIRL_RADIUS + 1e-3);
            assert!((position[2] - 6.0).abs() < SWIRL_RADIUS + 1e-3);
        }
    }

    #[test]
    fn head_rises_then_sinks() {
        let mut e = WaterBall2Effect::new([0.0; 3], [0.0; 3]);
        step(&mut e, 5);
        let y_early = e.segments[0][1];
        step(&mut e, 15); // ~frame 20 = peak
        let y_peak = e.segments[0][1];
        step(&mut e, 18); // ~frame 38 = sinking back
        let y_late = e.segments[0][1];
        assert!(y_peak < y_early, "rises (Y more negative) {y_early} → {y_peak}");
        assert!(y_late > y_peak, "sinks back {y_peak} → {y_late}");
    }

    #[test]
    fn dies_after_strand_drains() {
        let mut e = WaterBall2Effect::new([0.0; 3], [0.0; 3]);
        let mut status = EffectStatus::Running;
        for _ in 0..(DURATION_FRAMES as i32 + NUM_SEGMENT as i32 + 2) {
            status = e.update(&ctx(1.0 / FRAMES_PER_SECOND));
        }
        assert_eq!(status, EffectStatus::Dead);
    }
}
