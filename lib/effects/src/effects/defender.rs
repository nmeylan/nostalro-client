use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::{
    RADIAL_EMITTER_DIVISION, RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot,
};

pub const TEXTURES: &[&str] = &["ring_black.tga", "ring_yellow.tga"];

#[derive(Clone, Copy)]
pub struct DefenderParams {
    pub texture: &'static str,
    pub tint_rgb: [f32; 3],
}

pub const DEFENDER: DefenderParams = DefenderParams {
    texture: "ring_black.tga",
    tint_rgb: [1.0, 1.0, 1.0],
};

pub const REFLECTSHIELD: DefenderParams = DefenderParams {
    texture: "ring_yellow.tga",
    tint_rgb: [1.0, 1.0, 1.0],
};

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: u32 = 200;
pub const TOTAL_DURATION_MS: u32 = ((TOTAL_FRAMES as f32) / FRAMES_PER_SECOND * 1000.0) as u32;

const SEGMENTS: u32 = (RADIAL_EMITTER_DIVISION - 1) as u32;
const FULL_ARC_RAD: f32 = std::f32::consts::TAU;
const RISE_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_2;

const HEIGHT_SCALE: f32 = 0.9;

const SLOT_DISTANCES: [f32; RADIAL_EMITTER_SLOTS] = [8.0, 7.9, 7.8, 7.7];
const SLOT_ROT_START_DEG: [f32; RADIAL_EMITTER_SLOTS] = [180.0, 270.0, 0.0, 90.0];
const SLOT_MAX_HEIGHT: [f32; RADIAL_EMITTER_SLOTS] = [40.0, 39.0, 38.0, 37.0];

const ALPHA_PEAK: f32 = 80.0 / 255.0;
const FADE_IN_FRAMES: u32 = 100;
const FADE_OUT_FRAMES: u32 = 20;
const FADE_OUT_STEP: f32 = 5.0 / 255.0;
const FADE_IN_STEP: f32 = 1.0 / 255.0;

const SIN_LIMIT_MIDDLE: i32 = 10;
const SIN_LIMIT_STEP_DEG: f32 = 9.0;

pub struct DefenderEffect {
    world_pos: [f32; 3],
    params: DefenderParams,
    age_frames: f32,
    last_processed_frame: u32,
    emitter: RadialEmitter,
}

impl DefenderEffect {
    pub fn new(world_pos: [f32; 3], params: DefenderParams) -> Self {
        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        for ec in 0..RADIAL_EMITTER_SLOTS {
            let mut s = RadialEmitterSlot::spawn(SLOT_DISTANCES[ec], 90.0, SLOT_MAX_HEIGHT[ec]);
            s.rot_start_deg = SLOT_ROT_START_DEG[ec];
            s.full_display_angle_deg = 360.0;
            s.alpha_b = 0.0;
            slots[ec] = s;
        }
        Self {
            world_pos,
            params,
            age_frames: 0.0,
            last_processed_frame: 0,
            emitter: RadialEmitter::from_slots(slots),
        }
    }

    fn integrate_frames(&mut self, target_frame: u32) {
        while self.last_processed_frame < target_frame {
            self.emitter.tick();
            for (ec, slot) in self.emitter.slots.iter_mut().enumerate() {
                if !slot.alive {
                    continue;
                }
                slot.rot_start_deg += if ec == 0 || ec == 2 { 1.0 } else { 2.0 };
                if slot.rot_start_deg >= 360.0 {
                    slot.rot_start_deg -= 360.0;
                }

                if slot.process >= TOTAL_FRAMES.saturating_sub(FADE_OUT_FRAMES) {
                    slot.alpha_b = (slot.alpha_b - FADE_OUT_STEP).max(0.0);
                } else if slot.process < FADE_IN_FRAMES {
                    slot.alpha_b = (slot.alpha_b + FADE_IN_STEP).min(ALPHA_PEAK);
                }

                let pr_base = if ec < 2 {
                    slot.process as i32
                } else {
                    (slot.process as i32) * 2
                };
                let pr_deg = ((pr_base + (ec as i32) * 90).rem_euclid(360)) as f32;
                let sin_pr = pr_deg.to_radians().sin();

                for i in 0..RADIAL_EMITTER_DIVISION {
                    let sin_limit_deg =
                        90.0 + (i as i32 - SIN_LIMIT_MIDDLE) as f32 * SIN_LIMIT_STEP_DEG;
                    let bell = sin_limit_deg.to_radians().sin().max(0.0);
                    slot.height[i] = slot.max_height * bell * (1.0 + 0.3 * sin_pr);
                }
            }
            self.last_processed_frame += 1;
        }
    }
}

impl Effect for DefenderEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = (self.age_frames as u32).min(TOTAL_FRAMES);
        self.integrate_frames(target);

        if self.age_frames >= TOTAL_FRAMES as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for (_ec, slot) in self.emitter.active() {
            if slot.alpha_b <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center: self.world_pos,
                distance: slot.distance,
                rise_angle_rad: RISE_ANGLE_RAD,
                rot_start_rad: slot.rot_start_deg.to_radians(),
                full_arc_rad: FULL_ARC_RAD,
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights: slot.height,
                texture: self.params.texture,
                color: [
                    self.params.tint_rgb[0],
                    self.params.tint_rgb[1],
                    self.params.tint_rgb[2],
                    slot.alpha_b,
                ],
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

    fn step(e: &mut DefenderEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &DefenderEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn ring(prim: &EffectPrimitiveDraw) -> (f32, u32, f32, [f32; RADIAL_EMITTER_DIVISION]) {
        match prim {
            EffectPrimitiveDraw::RadialRing {
                distance,
                segments,
                color,
                heights,
                ..
            } => (*distance, *segments, color[3], *heights),
            _ => panic!("expected RadialRing, got {:?}", prim),
        }
    }

    #[test]
    fn rings_follow_entity_after_set_position() {
        let mut e = DefenderEffect::new([5.0, 0.0, -3.0], DEFENDER);
        step(&mut e, 30.0);
        e.set_position([12.0, 1.0, 8.0]);
        let prims = draws(&e);
        assert!(!prims.is_empty());
        for p in &prims {
            match p {
                EffectPrimitiveDraw::RadialRing { center, .. } => {
                    assert_eq!(*center, [12.0, 1.0, 8.0]);
                }
                _ => panic!("expected RadialRing, got {p:?}"),
            }
        }
    }

    #[test]
    fn emits_four_layered_rings_with_distinct_radii_and_spin() {
        let mut e = DefenderEffect::new([5.0, 0.0, -3.0], DEFENDER);
        step(&mut e, 30.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 4, "all four slots alive and visible");

        let radii: Vec<f32> = prims.iter().map(|p| ring(p).0).collect();
        assert_eq!(radii, vec![8.0, 7.9, 7.8, 7.7]);

        let mut shield = DefenderEffect::new([0.0; 3], REFLECTSHIELD);
        step(&mut shield, 30.0);
        let mut list = EffectDrawList::new();
        shield.collect_draws(&mut list, &render_ctx());
        match &list.primitives[0] {
            EffectPrimitiveDraw::RadialRing { texture, .. } => {
                assert_eq!(*texture, "ring_yellow.tga");
            }
            _ => panic!("expected RadialRing"),
        }

        for (ec, prim) in prims.iter().enumerate() {
            let (_, segs, alpha, heights) = ring(prim);
            assert_eq!(segs, SEGMENTS);
            assert!(alpha > 0.0, "alpha lifted for slot {ec}");
            let max_h = SLOT_MAX_HEIGHT[ec];
            assert!(heights[10] >= max_h * 0.7 && heights[10] <= max_h * 1.3);
        }
    }

    #[test]
    fn alpha_ramps_in_then_fades_out() {
        let mut e = DefenderEffect::new([0.0; 3], DEFENDER);
        step(&mut e, 1.0);
        let alpha_1 = ring(&draws(&e)[0]).2;
        assert!(alpha_1 > 0.0 && alpha_1 < ALPHA_PEAK);

        step(&mut e, (FADE_IN_FRAMES - 1) as f32);
        let alpha_peak = ring(&draws(&e)[0]).2;
        assert!(
            (alpha_peak - ALPHA_PEAK).abs() < 1e-3,
            "alpha at peak: {alpha_peak}"
        );

        step(&mut e, (TOTAL_FRAMES - FADE_IN_FRAMES - 1) as f32);
        let prims = draws(&e);
        let alpha_late = prims.first().map(|p| ring(p).2).unwrap_or(0.0);
        assert!(alpha_late < 0.05, "alpha late: {alpha_late}");
    }

    #[test]
    fn dies_after_total_frames() {
        let mut e = DefenderEffect::new([0.0; 3], DEFENDER);
        let s = step(&mut e, TOTAL_FRAMES as f32 + 1.0);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
