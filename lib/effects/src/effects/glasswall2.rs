use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::RADIAL_EMITTER_DIVISION;

pub const TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

pub const STR_OVERLAY: &str = "SafetyWall";

const FRAMES_PER_SECOND: f32 = 60.0;

pub const TOTAL_DURATION_MS: u32 = 99990;

const SLOT_DISTANCES: [f32; 3] = [3.6, 3.3, 3.0];
const SLOT_MAX_HEIGHT: [f32; 3] = [34.0, 37.0, 40.0];
const SLOT_ROT_START_DEG: [f32; 3] = [40.0, 170.0, 290.0];

const HEIGHT_SCALE: f32 = 0.42;
const RISE_FRAMES: f32 = 24.0;
const PEAK_ALPHA: f32 = 150.0 / 255.0;
const TINT: [f32; 3] = [1.0, 0.42, 0.85];

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const RISE_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_2;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;

pub struct Glasswall2Effect {
    world_pos: [f32; 3],
    age_frames: f32,
}

impl Glasswall2Effect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
        }
    }

    fn rise(&self) -> f32 {
        (self.age_frames / RISE_FRAMES).clamp(0.0, 1.0)
    }
}

impl Effect for Glasswall2Effect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let rise = self.rise();
        let alpha = PEAK_ALPHA * rise;
        if alpha <= 0.0 {
            return;
        }
        for slot in 0..3 {
            let h = SLOT_MAX_HEIGHT[slot] * rise;
            let heights = [h; RADIAL_EMITTER_DIVISION];
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: SLOT_DISTANCES[slot],
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: SLOT_ROT_START_DEG[slot].to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights,
                texture: TEXTURE,
                color: [TINT[0], TINT[1], TINT[2], alpha],
                blend: BlendKind::Additive,
            });
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(STR_OVERLAY)
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

    fn step_and_draw(e: &mut Glasswall2Effect, frames: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_concentric_rising_rings_with_str_overlay() {
        let mut e = Glasswall2Effect::new([10.0, 0.0, 20.0]);
        assert_eq!(e.str_overlay(), Some(STR_OVERLAY));
        let prims = step_and_draw(&mut e, RISE_FRAMES);
        let rings: Vec<_> = prims
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing {
                    distance,
                    rise_angle_rad,
                    texture,
                    ..
                } => Some((*distance, *rise_angle_rad, *texture)),
                _ => None,
            })
            .collect();
        assert_eq!(rings.len(), 3, "three stacked rings");
        for (_, rise, tex) in &rings {
            assert!((rise - RISE_ANGLE_RAD).abs() < 1e-4, "rings are upright");
            assert_eq!(*tex, TEXTURE);
        }
        let mut dists: Vec<f32> = rings.iter().map(|(d, ..)| *d).collect();
        dists.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((dists[0] - 3.0).abs() < 1e-4 && (dists[2] - 3.6).abs() < 1e-4);
    }

    #[test]
    fn column_rises_and_ramps_in_then_persists() {
        let mut e = Glasswall2Effect::new([0.0; 3]);
        let early = step_and_draw(&mut e, 6.0);
        let (h_early, a_early) = match &early[0] {
            EffectPrimitiveDraw::RadialRing { heights, color, .. } => (heights[0], color[3]),
            _ => unreachable!(),
        };
        let held = step_and_draw(&mut e, RISE_FRAMES); // well past the ramp
        let (h_full, a_full) = match &held[0] {
            EffectPrimitiveDraw::RadialRing { heights, color, .. } => (heights[0], color[3]),
            _ => unreachable!(),
        };
        assert!(h_full > h_early, "column grows taller");
        assert!(a_full > a_early, "alpha ramps in");
        assert!((a_full - PEAK_ALPHA).abs() < 1e-4, "alpha holds at peak");
    }

    #[test]
    fn persists_past_the_normal_effect_lifetime() {
        let mut e = Glasswall2Effect::new([0.0; 3]);
        let s = e.update(&EffectUpdateCtx {
            delta: 10.0,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Running));
    }
}
