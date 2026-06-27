use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const ALPHA_CENTER_TEXTURE: &str = "alpha_center.tga";
pub const TEXTURES: &[&str] = &[ALPHA_CENTER_TEXTURE];

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;
const SWEEP_FRAMES: f32 = 80.0;
const SEGMENT_RAMP_FRAMES: f32 = 18.0;
const FADE_OUT_FRAMES: f32 = 50.0;
const PEAK_ALPHA: f32 = 160.0 / 255.0;

const BASE_RADIUS: f32 = 26.0;
const BAND_RADIUS_STEP: f32 = 0.05;
const HEIGHT_FACTOR: f32 = 1.3;
const SEGMENTS: usize = 32;

const BAND_COLORS: [[f32; 3]; 7] = [
    [255.0 / 255.0, 0.0, 0.0],
    [255.0 / 255.0, 126.0 / 255.0, 0.0],
    [255.0 / 255.0, 255.0 / 255.0, 0.0],
    [0.0, 255.0 / 255.0, 0.0],
    [0.0, 0.0, 255.0 / 255.0],
    [115.0 / 255.0, 50.0 / 255.0, 200.0 / 255.0],
    [255.0 / 255.0, 0.0, 255.0 / 255.0],
];

pub struct RainbowEffect {
    center: [f32; 3],
    age_frames: f32,
}

impl RainbowEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            center: world_pos,
            age_frames: 0.0,
        }
    }

    fn fade_out(&self) -> f32 {
        if self.age_frames < TOTAL_FRAMES - FADE_OUT_FRAMES {
            1.0
        } else {
            (1.0 - (self.age_frames - (TOTAL_FRAMES - FADE_OUT_FRAMES)) / FADE_OUT_FRAMES)
                .clamp(0.0, 1.0)
        }
    }

    fn column_alpha(&self, t_mid: f32) -> f32 {
        let front = (self.age_frames / SWEEP_FRAMES).clamp(0.0, 1.0) * std::f32::consts::PI;
        if t_mid > front {
            return 0.0;
        }
        let activated_at = t_mid / std::f32::consts::PI * SWEEP_FRAMES;
        let since = (self.age_frames - activated_at).max(0.0);
        let ramp = (since / SEGMENT_RAMP_FRAMES).clamp(0.0, 1.0);
        PEAK_ALPHA * ramp * self.fade_out()
    }

    fn facing_yaw(&self, ctx: &EffectRenderCtx) -> f32 {
        let dx = ctx.camera.eye[0] - self.center[0];
        let dz = ctx.camera.eye[2] - self.center[2];
        dx.atan2(dz)
    }

    fn arch_point(&self, t: f32, radius: f32, yaw: f32) -> [f32; 3] {
        let lx = t.cos() * radius;
        let ly = -t.sin() * radius * HEIGHT_FACTOR;
        let (sy, cy) = yaw.sin_cos();
        [
            self.center[0] + lx * cy,
            self.center[1] + ly,
            self.center[2] - lx * sy,
        ]
    }
}

impl Effect for RainbowEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        if self.fade_out() <= 0.0 {
            return;
        }
        let yaw = self.facing_yaw(ctx);
        let step = std::f32::consts::PI / SEGMENTS as f32;
        for i in 0..SEGMENTS {
            let t0 = i as f32 * step;
            let t1 = (i + 1) as f32 * step;
            let alpha = self.column_alpha((t0 + t1) * 0.5);
            if alpha <= 0.0 {
                continue;
            }
            for (band, color) in BAND_COLORS.iter().enumerate() {
                let outer = BASE_RADIUS * (1.0 - band as f32 * BAND_RADIUS_STEP);
                let inner = BASE_RADIUS * (1.0 - (band as f32 + 1.0) * BAND_RADIUS_STEP);
                let corners = [
                    self.arch_point(t0, outer, yaw),
                    self.arch_point(t1, outer, yaw),
                    self.arch_point(t1, inner, yaw),
                    self.arch_point(t0, inner, yaw),
                ];
                out.push(EffectPrimitiveDraw::WorldQuad {
                    corners,
                    uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                    texture: ALPHA_CENTER_TEXTURE,
                    color: [color[0], color[1], color[2], alpha],
                    blend: BlendKind::Additive,
                    no_depth: false,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut RainbowEffect, frames: f32) -> EffectStatus {
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

    fn draws(e: &RainbowEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn distinct_band_colors(prims: &[EffectPrimitiveDraw]) -> Vec<[f32; 3]> {
        let mut colors = Vec::new();
        for p in prims {
            if let EffectPrimitiveDraw::WorldQuad { color, .. } = p {
                let rgb = [color[0], color[1], color[2]];
                if !colors.contains(&rgb) {
                    colors.push(rgb);
                }
            }
        }
        colors
    }

    #[test]
    fn full_arch_has_seven_additive_bands_once_swept() {
        let mut e = RainbowEffect::new([0.0; 3]);
        // Past the sweep + per-column ramp, before the tail fade-out.
        step(&mut e, SWEEP_FRAMES + SEGMENT_RAMP_FRAMES + 2.0);
        let prims = draws(&e);
        assert_eq!(
            prims.len(),
            7 * SEGMENTS,
            "7 bands × all segments now drawn"
        );
        for p in &prims {
            assert!(matches!(
                p,
                EffectPrimitiveDraw::WorldQuad {
                    blend: BlendKind::Additive,
                    ..
                }
            ));
        }
        let colors = distinct_band_colors(&prims);
        assert_eq!(colors.len(), 7, "seven spectrum bands");
        assert!(colors.contains(&[1.0, 0.0, 0.0]), "red band");
        assert!(colors.contains(&[1.0, 0.0, 1.0]), "violet/magenta band");
    }

    #[test]
    fn sweeps_on_progressively_from_one_foot() {
        let mut e = RainbowEffect::new([0.0; 3]);
        // Early: the front has only reached part-way, so fewer columns drawn.
        step(&mut e, SWEEP_FRAMES * 0.3);
        let early = draws(&e).len();
        step(&mut e, SWEEP_FRAMES * 0.5);
        let later = draws(&e).len();
        assert!(early > 0, "the leading edge has started drawing: {early}");
        assert!(
            later > early,
            "more of the arch is drawn as the front sweeps: {early} -> {later}"
        );
        assert!(early < 7 * SEGMENTS, "not the whole arch yet at 30% sweep");
    }

    #[test]
    fn arch_rises_above_the_base_at_its_apex() {
        let e = RainbowEffect::new([0.0, 0.0, 0.0]);
        let apex = e.arch_point(std::f32::consts::FRAC_PI_2, BASE_RADIUS, 0.0);
        assert!(
            apex[1] < -BASE_RADIUS,
            "apex rises above radius: {}",
            apex[1]
        );
    }

    #[test]
    fn fades_out_and_dies() {
        let mut e = RainbowEffect::new([0.0; 3]);
        step(&mut e, TOTAL_FRAMES - FADE_OUT_FRAMES + 1.0);
        let mid = e.fade_out();
        step(&mut e, FADE_OUT_FRAMES * 0.8);
        let late = e.fade_out();
        assert!(late < mid, "arch fades at the tail: {mid} -> {late}");
        assert_eq!(step(&mut e, TOTAL_FRAMES), EffectStatus::Dead);
    }
}
