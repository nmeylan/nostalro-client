use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const RING_BLUE_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[RING_BLUE_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;

const HEIGHT_SCALE: f32 = 0.5;
const CONE_SIDES: u32 = 20;
const GROW_FRAMES: f32 = 90.0;
const MAX_HEIGHT_PULSE: f32 = 2.5;
const WAVE_REL_AMPLITUDE: f32 = 0.35;
const FADE_IN_FRAMES: f32 = 20.0;
const FADE_OUT_START_FRAME: f32 = 110.0;
const FADE_OUT_FRAMES: f32 = 70.0;
const PEAK_ALPHA: f32 = 180.0 / 255.0;
/// 8 additive cones overdraw; pre-attenuate to keep texture detail.
const OVERDRAW_DIVISOR: f32 = 4.0;

const TOTAL_FRAMES: f32 = FADE_OUT_START_FRAME + FADE_OUT_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const TINT: [f32; 3] = [100.0 / 255.0, 100.0 / 255.0, 1.0];

const NUM_ARCS: usize = 4;
const PASS_F1_0: [(f32, f32, f32, f32); NUM_ARCS] = [
    (4.5, 85.0, 27.0, 0.0),
    (5.0, 80.0, 28.0, 90.0),
    (5.5, 70.0, 29.0, 180.0),
    (5.0, 75.0, 30.0, 270.0),
];
const PASS_F1_1: [(f32, f32, f32, f32); NUM_ARCS] = [
    (5.5, 50.0, 30.0, 90.0),
    (6.0, 55.0, 29.0, 180.0),
    (6.5, 65.0, 28.0, 270.0),
    (6.0, 60.0, 27.0, 0.0),
];

#[derive(Clone, Copy)]
struct Arc {
    distance: f32,
    rise_deg: f32,
    base_max_height: f32,
    rot_start_deg: f32,
}

pub struct FlowerCastEffect {
    world_pos: [f32; 3],
    arcs: Vec<Arc>,
    process: f32,
}

impl FlowerCastEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let arcs = PASS_F1_0
            .iter()
            .chain(PASS_F1_1.iter())
            .map(
                |&(distance, rise_deg, base_max_height, rot_start_deg)| Arc {
                    distance,
                    rise_deg,
                    base_max_height,
                    rot_start_deg,
                },
            )
            .collect();
        Self {
            world_pos,
            arcs,
            process: 0.0,
        }
    }

    fn grow(&self) -> f32 {
        (self.process.min(GROW_FRAMES)).to_radians().sin()
    }

    fn alpha(&self) -> f32 {
        let a = if self.process < FADE_IN_FRAMES {
            PEAK_ALPHA * (self.process / FADE_IN_FRAMES)
        } else if self.process < FADE_OUT_START_FRAME {
            PEAK_ALPHA
        } else {
            PEAK_ALPHA
                * (1.0 - (self.process - FADE_OUT_START_FRAME) / FADE_OUT_FRAMES).clamp(0.0, 1.0)
        };
        a / OVERDRAW_DIVISOR
    }
}

impl Effect for FlowerCastEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.process += ctx.delta * FRAMES_PER_SECOND;
        if self.process >= TOTAL_FRAMES {
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
        let grow = self.grow();
        let pulse = ((self.process % GROW_FRAMES) * 4.0).to_radians().sin() * MAX_HEIGHT_PULSE;
        for a in &self.arcs {
            let max_h = (a.base_max_height + pulse) * HEIGHT_SCALE;
            let (sin_rise, cos_rise) = a.rise_deg.to_radians().sin_cos();
            let height = sin_rise * max_h * grow;
            let bottom = a.distance;
            let top = a.distance + cos_rise * max_h * grow;
            let wave_amplitude = WAVE_REL_AMPLITUDE * max_h * grow;
            out.push(EffectPrimitiveDraw::Frustum {
                base_alpha: 1.0,
                base: self.world_pos,
                bottom_size: bottom,
                top_size: top,
                height,
                sides: CONE_SIDES,
                arc_angle_deg: 360.0,
                rotation: a.rot_start_deg.to_radians(),
                uv_repeat: 1.0,
                uv_scroll: [0.0, 0.0],
                wave_amplitude,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::SaintBell,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: RING_BLUE_TEXTURE,
                color: [TINT[0], TINT[1], TINT[2], alpha],
                blend: BlendKind::Additive,
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

    fn draws(e: &FlowerCastEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(e: &mut FlowerCastEffect, frames: i32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..frames {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        s
    }

    fn tallest(prims: &[EffectPrimitiveDraw]) -> f32 {
        prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { height, .. } => Some(*height),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn emits_eight_blue_frustum_cones() {
        let mut e = FlowerCastEffect::new([0.0; 3]);
        step(&mut e, 5);
        let prims = draws(&e);
        let cones = prims
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    EffectPrimitiveDraw::Frustum { texture, blend: BlendKind::Additive, .. }
                        if *texture == RING_BLUE_TEXTURE
                )
            })
            .count();
        assert_eq!(cones, 8, "two passes of four arcs");
    }

    #[test]
    fn starts_small_and_expands_uniformly() {
        let mut e = FlowerCastEffect::new([0.0; 3]);
        step(&mut e, 2);
        let early = tallest(&draws(&e));
        step(&mut e, 43); // ~frame 45, mid-rise
        let mid = tallest(&draws(&e));
        step(&mut e, 44); // ~frame 89, near full
        let full = tallest(&draws(&e));
        assert!(
            early < mid && mid < full,
            "height grows: {early} -> {mid} -> {full}"
        );
        assert!(early < 0.2 * full, "starts small relative to full height");
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = FlowerCastEffect::new([0.0; 3]);
        assert_eq!(step(&mut e, TOTAL_FRAMES as i32 - 1), EffectStatus::Running);
        assert_eq!(step(&mut e, 2), EffectStatus::Dead);
    }
}
