//! `EF_WATERBALL2` (id 117) — a twisting water column.
//!
//! Reference: the
//! original game gif `100-150/117.gif` — a tall braided blue/green column that
//! swirls upward then sinks. A single swirling particle strand
//! (`data/sprite/이팩트/waterball`, 12 segments,
//! size 0.75) whose head rises while a swirl spins it around a
//! vertical axis; the 12 trailing history points draw the braid.
//!
//! The original integrates a rising/swirling head with roll while the
//! head also travels horizontally at `radius / duration` along the
//! caster→target line. The net visible result is a braided column
//! that flies from the caster to the target while swirling and arcing, so we drive
//! the head as a horizontal caster→target travel (mirroring `waterball.rs`) plus a
//! rise-then-sink and a constant-radius swirl. Spawned
//! without trail data (`from == to`) it forms in place and swirls without moving.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

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

/// Swirl + rise/sink offset from the travelling base at a given (head) frame.
fn head_offset(frame: f32) -> [f32; 3] {
    let t = (frame / DURATION_FRAMES as f32).clamp(0.0, 1.0);
    // Rise up then sink back (native RO: up = −Y).
    let y = -RISE_AMP * (std::f32::consts::PI * t).sin();
    let a = (frame * SPIN_DEG_PER_FRAME).to_radians();
    let (s, c) = a.sin_cos();
    [SWIRL_RADIUS * c, y, SWIRL_RADIUS * s]
}

pub struct WaterBall2Effect {
    from: [f32; 3],
    /// Per-frame horizontal velocity carrying the head from caster to target.
    vel: [f32; 3],
    /// History of absolute world head positions (index 0 = newest), so the
    /// braid trails behind the moving head instead of riding a fixed base.
    segments: [[f32; 3]; NUM_SEGMENT],
    head_frame: u32,
    effect_frame: u32,
}

impl WaterBall2Effect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let dist = (dx * dx + dz * dz).sqrt();
        // Speed = radius / duration: cover the full horizontal
        // span across the strand's `DURATION_FRAMES` flight.
        let vel = if dist > 0.001 {
            [dx / DURATION_FRAMES as f32, 0.0, dz / DURATION_FRAMES as f32]
        } else {
            [0.0; 3]
        };
        let mut effect = Self {
            from,
            vel,
            segments: [[0.0; 3]; NUM_SEGMENT],
            head_frame: 0,
            effect_frame: 0,
        };
        effect.segments = [effect.head_world(0); NUM_SEGMENT];
        effect
    }

    /// Absolute world position of the head at a given frame: caster→target
    /// horizontal travel plus the swirl/rise-sink offset.
    fn head_world(&self, frame: u32) -> [f32; 3] {
        let f = frame as f32;
        let off = head_offset(f);
        [
            self.from[0] + self.vel[0] * f + off[0],
            self.from[1] + off[1],
            self.from[2] + self.vel[2] * f + off[2],
        ]
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
            self.segments[0] = self.head_world(self.head_frame);
        }
        self.effect_frame += 1;
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
                position: self.segments[i],
                action_index: 0,
                motion_index: motion,
                size_scale: size,
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

    fn head_xz(e: &WaterBall2Effect) -> [f32; 2] {
        [e.segments[0][0], e.segments[0][2]]
    }

    #[test]
    fn head_travels_horizontally_from_caster_toward_target() {
        let mut e = WaterBall2Effect::new([0.0; 3], [40.0, 0.0, 60.0]);
        step(&mut e, 5);
        let early = head_xz(&e);
        step(&mut e, 20);
        let late = head_xz(&e);
        // Head advances along the caster→target line (swirl radius is tiny
        // next to the travelled span, so the trend is unambiguous).
        assert!(late[0] > early[0], "advances along +X: {early:?} → {late:?}");
        assert!(late[1] > early[1], "advances along +Z: {early:?} → {late:?}");
        assert_eq!(segs(&e).len(), NUM_SEGMENT, "strand fills to 12 segments");
    }

    #[test]
    fn static_when_no_trail_data() {
        let mut e = WaterBall2Effect::new([3.0, 0.0, 7.0], [3.0, 0.0, 7.0]);
        step(&mut e, 10);
        // With from == to the head only swirls around the spawn point.
        let [x, z] = head_xz(&e);
        assert!((x - 3.0).abs() <= SWIRL_RADIUS + 1e-3);
        assert!((z - 7.0).abs() <= SWIRL_RADIUS + 1e-3);
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
