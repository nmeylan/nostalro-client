//! `EF_FIRESPLASHHIT` (#50) — the expanding ring of fire of a fire-property
//! hit.
//!
//! A single screen-space `FireRing.tga` quad is drawn over the target's
//! projected position: it starts small, grows `+5.7`/frame, spins
//! (-18°/frame, with a decelerating spin) and fades out over the back half
//! of its 30-frame life. The texture *is* a circular ring of fire, so one
//! screen-space quad reproduces it directly — no geometry annulus, no tiled
//! flame puffs.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use super::spike_burst::fade_in_out;

/// The ring-of-fire texture itself draws the circle (fire on transparent).
pub const FIRE_RING_TEXTURE: &str = "FireRing.tga";
pub const TEXTURES: &[&str] = &[FIRE_RING_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

/// The original sizes are screen pixels; our `BillboardFlash` is sized in
/// world units (a character is ~5-8). Downscaled uniformly so the frame-15
/// ring is roughly one-and-a-half characters wide.
const WORLD_SCALE: f32 = 0.06;
/// Ring lifted to the target's body centre (native RO — negative Y = up);
/// stands in for the original -20 px screen lift.
const BODY_LIFT: f32 = 3.0;

// Ring sizing/lifetime literals (screen-pixel half-extents → full quad is
// `2 * half`).
const HALF_INIT: f32 = 10.0;
const HALF_GROWTH_PER_FRAME: f32 = 5.7;
const DURATION_FRAMES: f32 = 30.0;
const FADE_OUT_START: f32 = 15.0;
const FADE_IN_FRAMES: f32 = 2.0;
const MAX_ALPHA: f32 = 1.0;
/// Spin: start angle `random(360)`, speed -18°/frame,
/// accel `-(roll_speed / duration) / 2.5` → the spin decelerates.
const ROLL_SPEED_INIT: f32 = -18.0;
const ROLL_ACCEL: f32 = -(ROLL_SPEED_INIT / DURATION_FRAMES) / 2.5;

pub struct FireSplashHitEffect {
    world_pos: [f32; 3],
    age: f32,
    /// Initial spin angle, seeded per spawn so repeated hits don't line up.
    roll0_deg: f32,
}

impl FireSplashHitEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let bits = world_pos[0].to_bits() ^ world_pos[2].to_bits().rotate_left(13);
        let roll0_deg = (bits % 360) as f32;
        Self { world_pos, age: 0.0, roll0_deg }
    }

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }

    /// Closed-form integral of `roll += rollSpeed; rollSpeed += rollAccel`
    /// over `n` frames: `roll0 + speed*n + accel*n*(n-1)/2`, in radians.
    fn roll_rad(&self, frame: f32) -> f32 {
        let deg = self.roll0_deg
            + ROLL_SPEED_INIT * frame
            + ROLL_ACCEL * frame * (frame - 1.0) * 0.5;
        deg.to_radians()
    }
}

impl Effect for FireSplashHitEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.frame() > DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.frame();
        let alpha = fade_in_out(frame, MAX_ALPHA, FADE_IN_FRAMES, FADE_OUT_START, DURATION_FRAMES);
        if alpha <= 0.0 {
            return;
        }
        let full = 2.0 * (HALF_INIT + HALF_GROWTH_PER_FRAME * frame) * WORLD_SCALE;
        let pos = [self.world_pos[0], self.world_pos[1] - BODY_LIFT, self.world_pos[2]];
        out.push(EffectPrimitiveDraw::BillboardFlash {
            pos,
            size: [full, full],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            rotation: self.roll_rad(frame),
            texture: FIRE_RING_TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Alpha,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx { camera: Default::default(), screen_w: 800.0, screen_h: 600.0, elapsed: 0.0 }
    }

    fn run_to(c: &mut FireSplashHitEffect, target_frame: f32) {
        let delta = (target_frame - c.frame()) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta, ..Default::default() });
        }
    }

    /// Returns `(size, rotation, alpha)` of the single fire-ring quad.
    fn ring(c: &FireSplashHitEffect) -> Option<(f32, f32, f32)> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives.into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::BillboardFlash { size, rotation, color, texture, blend, .. } => {
                assert_eq!(texture, FIRE_RING_TEXTURE);
                assert_eq!(blend, BlendKind::Alpha);
                Some((size[0], rotation, color[3]))
            }
            _ => None,
        })
    }

    #[test]
    fn ring_expands_and_spins() {
        let mut c = FireSplashHitEffect::new([3.0, 0.0, 7.0]);
        run_to(&mut c, 5.0);
        let (r1, rot1, _) = ring(&c).expect("ring");
        run_to(&mut c, 12.0);
        let (r2, rot2, _) = ring(&c).unwrap();
        assert!(r2 > r1, "ring expands ({r1} → {r2})");
        assert!(rot2 != rot1, "ring spins ({rot1} → {rot2})");
    }

    #[test]
    fn alpha_holds_then_fades() {
        let mut c = FireSplashHitEffect::new([0.0; 3]);
        run_to(&mut c, FADE_OUT_START);
        let hold = ring(&c).unwrap().2;
        run_to(&mut c, DURATION_FRAMES - 1.0);
        let late = ring(&c).unwrap().2;
        assert!(late < hold, "alpha fades after frame {FADE_OUT_START} ({hold} → {late})");
    }

    #[test]
    fn dies_after_duration() {
        let mut c = FireSplashHitEffect::new([0.0; 3]);
        run_to(&mut c, DURATION_FRAMES + 2.0);
        assert_eq!(c.update(&EffectUpdateCtx::default()), EffectStatus::Dead);
    }
}
