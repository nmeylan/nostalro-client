//! `EF_BOTTOM` (id 114) / `EF_BOTTOM2` (id 137) — Bard/Dancer ground song
//! boxes built from upright textured walls.
//!
//!
//! Both ids each launch four vertical textured quads forming
//! a square "well" around the caster: for `i ∈ {+1,-1}`, one wall offset
//! `±width` along the facing axis (longitude = facing) and one along the
//! perpendicular axis (longitude = facing + 90). The walls rise from the
//! ground plane and pulse up and down — observed in the original game as a
//! slow oscillation rather than a per-vertex ripple: the height speed simply
//! flips sign every 50 frames, so the walls
//! pulse up and down. Reproduced here as a scalar height pulse, re-emitting a
//! `Texture3D` quad each frame.
//!
//!   * Bottom : `magic_violet.tga`, width 5.0, height 15 (+0.25/f), fades in
//!     0→180 over 6 frames.
//!   * Bottom2: `magic_green.tga`,  width 2.5, height 13 (−0.25/f), starts opaque.
//!
//! The original duration is 9999 (the song loops while active); our one-shot
//! lifecycle caps it at the table's default and fades out near the end
//! (fade-out begins 35 frames before the end).

use std::f32::consts::FRAC_PI_2;

use crate::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const VIOLET_TEXTURE: &str = "magic_violet.tga";
pub const GREEN_TEXTURE: &str = "magic_green.tga";
pub const TEXTURES: &[&str] = &[VIOLET_TEXTURE, GREEN_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const WALLS: usize = 4;
const WAVER_PERIOD_FRAMES: f32 = 50.0;
const MAX_ALPHA: f32 = 180.0 / 255.0;
/// Alpha rises by `max_alpha / 6` per frame → fades in over 6 frames.
const FADE_IN_FRAMES: f32 = 6.0;
const FADE_OUT_BEFORE_END: f32 = 35.0;

const DURATION_FRAMES: f32 = 3500.0 / 1000.0 * FRAMES_PER_SECOND;
pub const TOTAL_DURATION_MS: u32 = 3500;

#[derive(Clone, Copy)]
struct Variant {
    width: f32,
    start_height: f32,
    height_speed: f32,
    start_alpha: f32,
    texture: &'static str,
}

const BOTTOM: Variant = Variant {
    width: 5.0,
    start_height: 15.0,
    height_speed: 0.25,
    start_alpha: 0.0,
    texture: VIOLET_TEXTURE,
};

const BOTTOM2: Variant = Variant {
    width: 2.5,
    start_height: 13.0,
    height_speed: -0.25,
    start_alpha: MAX_ALPHA,
    texture: GREEN_TEXTURE,
};

pub struct BottomBoxEffect {
    world_pos: [f32; 3],
    facing: f32,
    variant: Variant,
    height: f32,
    height_speed: f32,
    age_frames: f32,
}

impl BottomBoxEffect {
    fn new(world_pos: [f32; 3], variant: Variant) -> Self {
        Self {
            world_pos,
            facing: 0.0,
            variant,
            height: variant.start_height,
            height_speed: variant.height_speed,
            age_frames: 0.0,
        }
    }

    pub fn bottom(world_pos: [f32; 3]) -> Self {
        Self::new(world_pos, BOTTOM)
    }

    pub fn bottom2(world_pos: [f32; 3]) -> Self {
        Self::new(world_pos, BOTTOM2)
    }

    fn alpha(&self) -> f32 {
        let fade_out_start = DURATION_FRAMES - FADE_OUT_BEFORE_END;
        if self.age_frames >= fade_out_start {
            let span = (DURATION_FRAMES - fade_out_start).max(1e-3);
            return MAX_ALPHA * (1.0 - (self.age_frames - fade_out_start) / span).clamp(0.0, 1.0);
        }
        if self.variant.start_alpha >= MAX_ALPHA {
            return MAX_ALPHA;
        }
        (self.age_frames / FADE_IN_FRAMES).clamp(0.0, 1.0) * MAX_ALPHA
    }
}

impl Effect for BottomBoxEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let dt_frames = ctx.delta * FRAMES_PER_SECOND;
        // Oscillation: flip the height speed every 50 frames.
        let before = (self.age_frames / WAVER_PERIOD_FRAMES).floor();
        let after = ((self.age_frames + dt_frames) / WAVER_PERIOD_FRAMES).floor();
        if after > before {
            self.height_speed = -self.height_speed;
        }
        self.height = (self.height + self.height_speed * dt_frames).max(0.0);
        self.age_frames += dt_frames;
        if self.age_frames >= DURATION_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let color = [1.0, 1.0, 1.0, alpha];
        let half_h = self.height / 2.0;
        // Native RO -Y = up: base sits on the ground (anchor.y), top extends up.
        let center_y = self.world_pos[1] - half_h;
        let w = self.variant.width;
        for k in 0..WALLS {
            let theta = self.facing + k as f32 * FRAC_PI_2;
            let (s, c) = theta.sin_cos();
            let center = [
                self.world_pos[0] + w * c,
                center_y,
                self.world_pos[2] + w * s,
            ];
            out.push(EffectPrimitiveDraw::Texture3D {
                center,
                size: [w, half_h],
                // Wall width axis is perpendicular to its outward offset.
                plane: QuadPlane::VerticalYaw(theta + FRAC_PI_2),
                uv: [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                texture: self.variant.texture,
                color,
                blend: BlendKind::Alpha,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut BottomBoxEffect, frames: i32) {
        for _ in 0..frames {
            e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None, caster_yaw: None });
        }
    }

    fn quad_count(e: &BottomBoxEffect) -> usize {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Texture3D { .. }))
            .count()
    }

    #[test]
    fn bottom_emits_four_walls_and_grows_then_waveringly_reverses() {
        // Sociable: Bottom builds a 4-wall box; height grows for the first 50
        // frames then the oscillation flip reverses the trend.
        let mut e = BottomBoxEffect::bottom([0.0, 0.0, 0.0]);
        step(&mut e, 6); // past fade-in so quads are visible
        assert_eq!(quad_count(&e), 4);

        let h0 = e.height;
        step(&mut e, 20); // age ~26, still growing
        let h_before_peak = e.height;
        assert!(h_before_peak > h0, "height grows before the flip: {h0} -> {h_before_peak}");

        step(&mut e, 17); // age ~43, near the 50-frame peak
        let h_peak = e.height;
        step(&mut e, 15); // age ~58, past the oscillation flip
        let h_after = e.height;
        assert!(h_after < h_peak, "waveringly reversed the height trend: {h_peak} -> {h_after}");
    }

    #[test]
    fn bottom2_shrinks_while_bottom_grows() {
        // Sociable: same family, opposite initial height trend.
        let mut up = BottomBoxEffect::bottom([0.0, 0.0, 0.0]);
        let mut down = BottomBoxEffect::bottom2([0.0, 0.0, 0.0]);
        let up0 = up.height;
        let down0 = down.height;
        step(&mut up, 10);
        step(&mut down, 10);
        assert!(up.height > up0, "Bottom grows");
        assert!(down.height < down0, "Bottom2 shrinks");
    }
}
