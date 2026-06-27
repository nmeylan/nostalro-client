//! `EF_MAPPILLAR` (231) / `EF_MAPPILLAR2` (247) / `EF_MAPPILLAR3` (259) / `EF_MAPPILLAR4` (260).

use std::f32::consts::{PI, TAU};

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::RADIAL_EMITTER_DIVISION;

const FRAMES_PER_SECOND: f32 = 60.0;

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;

const MAX_HEIGHT: f32 = 120.0;
const HEIGHT_SCALE: f32 = 0.45;
const RISE_ANGLE_RAD: f32 = 89.0 * PI / 180.0;
const RISE_FRAMES: f32 = 54.0;
const SLOT_START_FRAME: [f32; 4] = [30.0, 20.0, 10.0, 0.0];
const SLOT_ROT_START_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];

pub const TEXTURES: &[&str] = &["ring_blue.tga", "magic_green.tga", "ring_red.tga"];

#[derive(Clone, Copy, Debug)]
pub struct MappillarParams {
    pub texture: &'static str,
    pub color_rgb: [f32; 3],
    pub distance_base: f32,
    pub alpha_max: f32,
}

pub const MAPPILLAR: MappillarParams = MappillarParams {
    texture: "ring_blue.tga",
    color_rgb: [0.30, 0.40, 1.00],
    distance_base: 2.0,
    alpha_max: 50.0 / 255.0,
};

pub const MAPPILLAR2: MappillarParams = MappillarParams {
    texture: "ring_blue.tga",
    color_rgb: [0.30, 0.40, 1.00],
    distance_base: 11.0,
    alpha_max: 70.0 / 255.0,
};

pub const MAPPILLAR3: MappillarParams = MappillarParams {
    texture: "magic_green.tga",
    color_rgb: [0.30, 1.00, 0.30],
    distance_base: 2.0,
    alpha_max: 50.0 / 255.0,
};

pub const MAPPILLAR4: MappillarParams = MappillarParams {
    texture: "ring_red.tga",
    color_rgb: [1.00, 0.25, 0.25],
    distance_base: 2.0,
    alpha_max: 50.0 / 255.0,
};

pub struct MappillarEffect {
    params: MappillarParams,
    world_pos: [f32; 3],
    age_frames: f32,
}

impl MappillarEffect {
    pub fn new(world_pos: [f32; 3], params: MappillarParams) -> Self {
        Self {
            params,
            world_pos,
            age_frames: 0.0,
        }
    }

    fn slot_height(&self, slot_idx: usize) -> f32 {
        let local = self.age_frames - SLOT_START_FRAME[slot_idx];
        if local <= 0.0 {
            0.0
        } else if local < RISE_FRAMES {
            MAX_HEIGHT * (local / RISE_FRAMES * std::f32::consts::FRAC_PI_2).sin()
        } else {
            MAX_HEIGHT
        }
    }
}

impl Effect for MappillarEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        for slot in 0..4 {
            let height = self.slot_height(slot);
            if height <= 0.0 {
                continue;
            }

            let rot_deg = SLOT_ROT_START_DEG[slot] + self.age_frames;
            let rot_rad = rot_deg * PI / 180.0;
            let distance = self.params.distance_base + slot as f32 * 0.5;

            let heights = [height; RADIAL_EMITTER_DIVISION];
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance,
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: rot_rad,
                full_arc_rad: TAU,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights,
                texture: self.params.texture,
                color: [r, g, b, self.params.alpha_max],
                blend: BlendKind::Additive,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dt: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn advance(e: &mut MappillarEffect, frames: f32) {
        e.update(&ctx(frames / FRAMES_PER_SECOND));
    }

    fn radial_rings(e: &MappillarEffect) -> Vec<(f32, f32, f32, f32)> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter_map(|p| match p {
                EffectPrimitiveDraw::RadialRing {
                    distance,
                    rot_start_rad,
                    heights,
                    color,
                    ..
                } => Some((*distance, *rot_start_rad, heights[0], color[3])),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn slots_emerge_in_staggered_order() {
        let mut e = MappillarEffect::new([0.0; 3], MAPPILLAR);
        advance(&mut e, 5.0);
        let rings = radial_rings(&e);
        assert_eq!(rings.len(), 1, "only slot 3 visible at frame 5");
        assert!((rings[0].0 - 3.5).abs() < 1e-4, "slot 3 at distance 3.5");

        // Frame 35: all four start frames (≤30) have passed → all visible.
        advance(&mut e, 30.0);
        assert_eq!(radial_rings(&e).len(), 4, "all 4 slots visible at frame 35");
    }

    #[test]
    fn full_height_held_after_grow_window() {
        let mut e = MappillarEffect::new([0.0; 3], MAPPILLAR);
        advance(&mut e, 100.0);
        let rings = radial_rings(&e);
        assert_eq!(rings.len(), 4);
        for (_, _, h, _) in &rings {
            assert!(
                (*h - MAX_HEIGHT).abs() < 1.0,
                "slot at full height ({h}), expected ~{MAX_HEIGHT}"
            );
        }
    }

    #[test]
    fn rotation_advances_and_slots_are_90_deg_apart() {
        let mut e = MappillarEffect::new([0.0; 3], MAPPILLAR);
        advance(&mut e, 100.0);
        let rings = radial_rings(&e);
        assert_eq!(rings.len(), 4);

        // rot = (slot*90 + age_frames) deg. At frame 100:
        //   slot 0 → 100°, slot 1 → 190°, slot 2 → 280°, slot 3 → 370°=10°.
        let rots_deg: Vec<f32> = rings
            .iter()
            .map(|(_, r, _, _)| r.to_degrees().rem_euclid(360.0))
            .collect();
        for i in 0..3 {
            let diff = (rots_deg[i + 1] - rots_deg[i]).rem_euclid(360.0);
            assert!(
                (diff - 90.0).abs() < 1.0,
                "adjacent slots 90° apart (diff={diff:.1}°)"
            );
        }
    }

    #[test]
    fn mappillar2_uses_distant_base_radius() {
        let mut e = MappillarEffect::new([0.0; 3], MAPPILLAR2);
        advance(&mut e, 100.0);
        let rings = radial_rings(&e);
        assert_eq!(rings.len(), 4);
        for (dist, _, _, _) in &rings {
            assert!(
                *dist >= 11.0 && *dist <= 12.5,
                "Mappillar2 rings at distant base ({dist})"
            );
        }
    }

    #[test]
    fn variants_have_correct_textures() {
        assert_eq!(MAPPILLAR.texture, "ring_blue.tga");
        assert_eq!(MAPPILLAR2.texture, "ring_blue.tga");
        assert_eq!(MAPPILLAR3.texture, "magic_green.tga");
        assert_eq!(MAPPILLAR4.texture, "ring_red.tga");
        for p in [MAPPILLAR, MAPPILLAR2, MAPPILLAR3, MAPPILLAR4] {
            assert!(TEXTURES.contains(&p.texture));
        }
    }

    #[test]
    fn never_self_terminates() {
        let mut e = MappillarEffect::new([0.0; 3], MAPPILLAR);
        for _ in 0..100 {
            assert_eq!(e.update(&ctx(0.1)), EffectStatus::Running);
        }
    }
}
