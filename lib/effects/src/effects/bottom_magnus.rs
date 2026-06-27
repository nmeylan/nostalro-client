//! Bottom_Magnus family — square vertical pillars (Magnus Exorcismus / Fogwall).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

#[derive(Clone, Copy, Debug)]
pub struct BottomMagnusParams {
    pub texture: &'static str,
    pub half_extent: f32,
    pub height: f32,
    pub tint_rgb: [f32; 3],
    pub alpha_top: f32,
    pub alpha_bottom: f32,
    pub blend: BlendKind,
}

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_IN_SECS: f32 = FADE_IN_FRAMES / FRAMES_PER_SECOND;
const HEIGHT_RAMP_FRAMES: f32 = 90.0;
const STEADY_FRAC: f32 = 0.65;
const PULSE_FRAC: f32 = 0.35;

pub(crate) fn animated_height(max_height: f32, age: f32, phase_deg: f32) -> f32 {
    let frame = age * FRAMES_PER_SECOND;
    let angle = (phase_deg + frame).rem_euclid(360.0);
    let mut h = max_height * angle.to_radians().sin() * PULSE_FRAC + max_height * STEADY_FRAC;
    if frame < HEIGHT_RAMP_FRAMES {
        h *= frame.to_radians().sin();
    }
    h
}

pub const MAGNUS: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_red.tga",
    half_extent: 5.0,
    height: 20.0,
    tint_rgb: [1.0, 1.0, 1.0],
    alpha_top: 0.0,
    alpha_bottom: 0.7,
    blend: BlendKind::Additive,
};

pub const FOGWALL: BottomMagnusParams = BottomMagnusParams {
    texture: "ring_white.tga",
    half_extent: 2.5,
    height: 32.0,
    tint_rgb: [80.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0],
    alpha_top: 0.7,
    alpha_bottom: 0.7,
    blend: BlendKind::Additive,
};

pub const TEXTURES: &[&str] = &["ring_red.tga", "ring_white.tga"];

pub struct BottomMagnusEffect {
    world_pos: [f32; 3],
    params: BottomMagnusParams,
    age: f32,
    phase_deg: f32,
}

impl BottomMagnusEffect {
    pub fn new(world_pos: [f32; 3], params: BottomMagnusParams) -> Self {
        let key = (world_pos[0].to_bits() ^ world_pos[2].to_bits()) as f32 * 1.6180339;
        Self {
            world_pos,
            params,
            age: 0.0,
            phase_deg: key.rem_euclid(360.0),
        }
    }
}

impl Effect for BottomMagnusEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let fade = (self.age / FADE_IN_SECS).clamp(0.0, 1.0);
        let height = animated_height(self.params.height, self.age, self.phase_deg);
        let [r, g, b] = self.params.tint_rgb;
        out.push(EffectPrimitiveDraw::Cylinder {
            base: self.world_pos,
            bottom_size: self.params.half_extent,
            top_size: self.params.half_extent,
            height,
            sides: 4,
            rotation: 0.0,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            uv_scroll: [0.0, 0.0],
            texture: self.params.texture,
            color: [r, g, b, self.params.alpha_top * fade],
            alpha_bottom: self.params.alpha_bottom * fade,
            blend: self.params.blend,
        });
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

    fn step(effect: &mut BottomMagnusEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomMagnusEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn magnus_emits_four_sided_additive_pillar_fading_to_transparent_top() {
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], MAGNUS);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder {
                sides,
                bottom_size,
                top_size,
                blend,
                color,
                alpha_bottom,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!((bottom_size - 5.0).abs() < f32::EPSILON);
                assert!((top_size - 5.0).abs() < f32::EPSILON);
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "ring_red.tga");
                assert!((color[0] - 1.0).abs() < 1e-4);
                assert!(*alpha_bottom > color[3]);
                assert!(color[3] < 1e-4, "top fades to transparent: {}", color[3]);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }

    #[test]
    fn pillar_rises_from_ground_then_breathes_below_peak() {
        let max = MAGNUS.height;
        let early = animated_height(max, 5.0 / FRAMES_PER_SECOND, 0.0);
        assert!(early < 0.3 * max, "still rising from the ground: {early}");

        let (mut lo, mut hi, mut sum, mut n) = (f32::MAX, 0.0_f32, 0.0, 0);
        for f in 90..=450 {
            let h = animated_height(max, f as f32 / FRAMES_PER_SECOND, 0.0);
            lo = lo.min(h);
            hi = hi.max(h);
            sum += h;
            n += 1;
        }
        assert!(
            lo >= 0.30 * max - 0.5 && hi <= max + 0.5,
            "bounds: {lo}..{hi}"
        );
        let avg = sum / n as f32;
        assert!(
            (avg - 0.65 * max).abs() < 0.05 * max,
            "average breath ~0.65·max, got {avg}"
        );
    }

    #[test]
    fn fogwall_emits_additive_dark_pillar() {
        let mut e = BottomMagnusEffect::new([0.0, 0.0, 0.0], FOGWALL);
        step(&mut e, FADE_IN_SECS);
        match &draws(&e)[0] {
            EffectPrimitiveDraw::Cylinder {
                sides,
                height,
                blend,
                color,
                texture,
                ..
            } => {
                assert_eq!(*sides, 4);
                assert!(*height <= FOGWALL.height + 0.5);
                assert_eq!(*blend, BlendKind::Additive);
                assert_eq!(*texture, "ring_white.tga");
                assert!((color[0] - 80.0 / 255.0).abs() < 1e-4);
                assert!((color[2] - 80.0 / 255.0).abs() < 1e-4);
            }
            other => panic!("expected Cylinder, got {other:?}"),
        }
    }
}
