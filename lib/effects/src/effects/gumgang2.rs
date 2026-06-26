//! `EF_GUMGANG2` — Fury Cast Animation. Petals of yellow flame open
//! outward from a central ring, like a flower blooming.
//!
//! Reference: `ro-effects/effects/imgs/250-300/261.gif`.
//!
//! The yellow-flame cones bloom outward like petals. Each cone
//! integrates a
//! `rise_angle` down from 80° to 40°: as the angle falls, the cone's top
//! rim flares outward (`cos(rise) * max_height`) while its vertical reach
//! shrinks (`sin(rise) * max_height`). Concentric emitters at distances
//! 1/2/3/4 wu (ec ∈ 0..3) each spin at +3°/frame on the slow branch.
//! The `ring_yellow.tga` texture, mapped through the
//! `Cylinder` primitive's 0.25-per-segment UV cadence, paints four
//! vertical petal stripes per cone — matching the gif's streaked-petal
//! silhouette without the per-segment wave that would reproduce the
//! LandProtector/Gumgang3 wreath look.
//!
//! Recipe: 4 concentric flared cones (`bottom_size = distance`,
//! `top_size = distance + cos(rise) * max_h`), `rise_angle` decaying over
//! 90 frames, alpha fade-in (12 frames) → hold → fade-out (last 60 frames).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::{RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot};

pub const TEXTURE: &str = "ring_yellow.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// 180 frames = 3000 ms — matches `effect/table.rs` `EffectId::Gumgang2`.
/// The reference gif is shorter (~120 frames); the tail covers fade-out.
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const SIDES: u32 = 24;

const DISTANCE_BASE: f32 = 1.0;
const DISTANCE_STEP: f32 = 0.6;
/// The slow branch grows `distance` at +0.1/frame; we
/// halve that so the outermost ring doesn't sweep past the gif silhouette.
const DISTANCE_GROWTH_PER_FRAME: f32 = 0.05;

const MAX_HEIGHT: f32 = 7.0;

/// `rise_angle` decays from `RISE_INITIAL_DEG` to `RISE_FINAL_DEG` over
/// `RISE_DECAY_FRAMES`. As it falls, `top_size` flares outward and
/// `height` shrinks — petals open.
const RISE_INITIAL_DEG: f32 = 80.0;
const RISE_FINAL_DEG: f32 = 30.0;
const RISE_DECAY_FRAMES: f32 = 90.0;

const ALPHA_PEAK: f32 = 0.8;
const FADE_IN_FRAMES: f32 = 12.0;
const FADE_OUT_FRAMES: f32 = 60.0;

const ROT_DEG_PER_FRAME: f32 = 3.0;

pub struct Gumgang2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
    emitter: RadialEmitter,
}

impl Gumgang2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        for (ec, slot) in slots.iter_mut().enumerate() {
            *slot = RadialEmitterSlot::spawn(
                DISTANCE_BASE + ec as f32 * DISTANCE_STEP,
                RISE_INITIAL_DEG,
                MAX_HEIGHT,
            );
        }
        Self {
            world_pos,
            age_frames: 0.0,
            emitter: RadialEmitter::from_slots(slots),
        }
    }
}

fn alpha_at(frame: f32) -> f32 {
    if frame < 0.0 || frame >= TOTAL_FRAMES {
        return 0.0;
    }
    let fade_out_start = TOTAL_FRAMES - FADE_OUT_FRAMES;
    let fade_in = (frame / FADE_IN_FRAMES).clamp(0.0, 1.0);
    let fade_out = if frame <= fade_out_start {
        1.0
    } else {
        ((TOTAL_FRAMES - frame) / FADE_OUT_FRAMES).max(0.0)
    };
    ALPHA_PEAK * fade_in * fade_out
}

fn rise_angle_deg(frame: f32) -> f32 {
    let t = (frame / RISE_DECAY_FRAMES).clamp(0.0, 1.0);
    RISE_INITIAL_DEG + (RISE_FINAL_DEG - RISE_INITIAL_DEG) * t
}

impl Effect for Gumgang2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let prev_frames = self.age_frames;
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let delta_frames = self.age_frames - prev_frames;

        let new_rise = rise_angle_deg(self.age_frames);
        self.emitter.tick();
        for slot in self.emitter.slots.iter_mut().filter(|s| s.alive) {
            slot.distance += DISTANCE_GROWTH_PER_FRAME * delta_frames;
            slot.rise_angle_deg = new_rise;
            slot.rot_start_deg += ROT_DEG_PER_FRAME * delta_frames;
        }

        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = alpha_at(self.age_frames);
        if alpha <= 0.0 {
            return;
        }

        for (ec, slot) in self.emitter.active() {
            let (sin_rise, cos_rise) = slot.rise_angle_deg.to_radians().sin_cos();
            let max_outward = cos_rise * slot.max_height;
            let max_upward = sin_rise * slot.max_height;
            let rotation_rad = (ec as f32 * 90.0 + slot.rot_start_deg).to_radians();

            out.push(EffectPrimitiveDraw::Cylinder {
                base: self.world_pos,
                bottom_size: slot.distance,
                top_size: slot.distance + max_outward,
                height: max_upward,
                sides: SIDES,
                rotation: rotation_rad,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                uv_scroll: [0.0, 0.0],
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
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

    fn step(e: &mut Gumgang2Effect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &Gumgang2Effect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn cone(prim: &EffectPrimitiveDraw) -> (f32, f32, f32, &'static str) {
        match prim {
            EffectPrimitiveDraw::Cylinder {
                bottom_size,
                top_size,
                height,
                texture,
                ..
            } => (*bottom_size, *top_size, *height, *texture),
            _ => panic!("expected Cylinder, got {:?}", prim),
        }
    }

    #[test]
    fn emits_concentric_flared_cones() {
        // Sociable: at peak the effect emits RADIAL_EMITTER_SLOTS flared cones
        // (top_size > bottom_size), at the expected base radii, all
        // textured with ring_yellow.tga.
        let mut e = Gumgang2Effect::new([5.0, 0.0, -3.0]);
        step(&mut e, FADE_IN_FRAMES + 1.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), RADIAL_EMITTER_SLOTS);
        for (i, prim) in prims.iter().enumerate() {
            let (bottom, top, height, tex) = cone(prim);
            let expected_base = DISTANCE_BASE
                + i as f32 * DISTANCE_STEP
                + DISTANCE_GROWTH_PER_FRAME * (FADE_IN_FRAMES + 1.0);
            assert!(
                (bottom - expected_base).abs() < 1e-3,
                "ring {i} bottom={bottom}, want {expected_base}"
            );
            assert!(
                top > bottom,
                "ring {i} should flare: top={top}, bottom={bottom}"
            );
            assert!(height > 0.0, "ring {i} should have height");
            assert_eq!(tex, TEXTURE);
        }
    }

    #[test]
    fn petals_open_over_time() {
        // Early frame: cones near-vertical (small flare, tall).
        // Late frame (post-decay): cones flared wide, shorter.
        let mut e = Gumgang2Effect::new([0.0; 3]);
        step(&mut e, 4.0);
        let (b_early, t_early, h_early, _) = cone(&draws(&e)[0]);
        let flare_early = t_early - b_early;

        step(&mut e, RISE_DECAY_FRAMES);
        let (b_late, t_late, h_late, _) = cone(&draws(&e)[0]);
        let flare_late = t_late - b_late;

        assert!(
            flare_late > flare_early,
            "petals must open over time: flare {flare_early} -> {flare_late}"
        );
        assert!(
            h_late < h_early,
            "vertical reach shrinks as petals open: {h_early} -> {h_late}"
        );
        assert!(
            b_late > b_early,
            "ring expands outward: {b_early} -> {b_late}"
        );
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = Gumgang2Effect::new([0.0; 3]);
        let s = step(&mut e, TOTAL_FRAMES + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
