use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::radial_emitter::{
    RADIAL_EMITTER_DIVISION, RADIAL_EMITTER_SLOTS, RadialEmitter, RadialEmitterSlot,
};

#[derive(Clone, Copy, Debug)]
pub struct BottomVolcanoParams {
    pub texture: &'static str,
}

pub const VOLCANO_RED: BottomVolcanoParams = BottomVolcanoParams {
    texture: "ring_red.tga",
};
pub const VOLCANO_BLUE: BottomVolcanoParams = BottomVolcanoParams {
    texture: "ring_blue.tga",
};
pub const VOLCANO_GREEN: BottomVolcanoParams = BottomVolcanoParams {
    texture: "magic_green.tga",
};
pub const SUITON: BottomVolcanoParams = BottomVolcanoParams {
    texture: "ring_blue.tga",
};

pub const TEXTURES: &[&str] = &["ring_red.tga", "ring_blue.tga", "magic_green.tga"];

const FRAMES_PER_SECOND: f32 = 60.0;
const BASE_DISTANCE: f32 = 2.6;
const BASE_RISE_ANGLE_DEG: f32 = 80.0;
const MAX_HEIGHT: f32 = 15.0;
const E_DIV: u32 = 10;
const SEGMENTS: u32 = E_DIV;
const HEIGHT_SCALE: f32 = 1.0;

const DISTANCE_GROWTH_PER_FRAME: f32 = 0.04;
const DISTANCE_MAX: f32 = 2.5;
const DISTANCE_RESET: f32 = DISTANCE_MAX - 0.36;
const RISE_ANGLE_FLOOR_DEG: f32 = 10.0;
const RISE_ANGLE_RESET_DEG: f32 = 74.0;

const ALPHA_FADE_IN_PER_FRAME: f32 = 20.0 / 255.0;
const ALPHA_FADE_OUT_PER_FRAME: f32 = 2.0 / 255.0;

const MIDDLE: f32 = ((RADIAL_EMITTER_DIVISION - 1) / 2) as f32;
const M2: f32 = 90.0 / MIDDLE;
const WOBBLE_AMPLITUDE: f32 = 0.3;

pub struct BottomVolcanoEffect {
    world_pos: [f32; 3],
    params: BottomVolcanoParams,
    age_frames: f32,
    last_processed_frame: u32,
    emitter: RadialEmitter,
}

impl BottomVolcanoEffect {
    pub fn new(world_pos: [f32; 3], params: BottomVolcanoParams) -> Self {
        let hash = position_hash(&world_pos);
        let rot_start_deg = (hash % 360) as f32;
        let alpha_init = ((hash / 360) % 100) as f32 / 255.0;

        let mut slots = [RadialEmitterSlot::dormant(); RADIAL_EMITTER_SLOTS];
        let mut slot = RadialEmitterSlot::spawn(BASE_DISTANCE, BASE_RISE_ANGLE_DEG, MAX_HEIGHT);
        slot.rot_start_deg = rot_start_deg;
        slot.full_display_angle_deg = 360.0;
        slot.alpha_b = alpha_init;
        slots[0] = slot;

        let mut effect = Self {
            world_pos,
            params,
            age_frames: 0.0,
            last_processed_frame: 0,
            emitter: RadialEmitter::from_slots(slots),
        };
        effect.update_petal_heights();
        effect
    }

    fn integrate_frames(&mut self, target_frame: u32) {
        while self.last_processed_frame < target_frame {
            self.emitter.tick();
            for slot in self.emitter.slots.iter_mut().filter(|s| s.alive) {
                slot.distance += DISTANCE_GROWTH_PER_FRAME;

                slot.rise_angle_deg -= 1.0;
                if slot.rise_angle_deg < RISE_ANGLE_FLOOR_DEG {
                    slot.rise_angle_deg = RISE_ANGLE_FLOOR_DEG;
                    slot.alpha_b = 0.0;
                }

                if slot.distance >= DISTANCE_MAX {
                    slot.alpha_b -= ALPHA_FADE_OUT_PER_FRAME;
                    if slot.alpha_b <= 0.0 {
                        slot.alpha_b = 0.0;
                        slot.distance = DISTANCE_RESET;
                        slot.rise_angle_deg = RISE_ANGLE_RESET_DEG;
                    }
                } else {
                    slot.alpha_b += ALPHA_FADE_IN_PER_FRAME;
                }
            }
            self.update_petal_heights();
            self.last_processed_frame += 1;
        }
    }

    fn update_petal_heights(&mut self) {
        for (ec, slot) in self.emitter.slots.iter_mut().enumerate() {
            if !slot.alive {
                continue;
            }
            let pr_deg = ((slot.process as f32) + (ec as f32) * 90.0).rem_euclid(360.0);
            let pr_sin = pr_deg.to_radians().sin();
            for i in 0..RADIAL_EMITTER_DIVISION {
                let sin_limit_deg = 90.0 + (i as f32 - MIDDLE) * M2;
                let amplitude = sin_limit_deg.to_radians().sin();
                slot.height[i] =
                    slot.max_height + slot.max_height * amplitude * WOBBLE_AMPLITUDE * pr_sin;
            }
        }
    }

    pub fn distance(&self) -> f32 {
        self.emitter.slots[0].distance
    }

    pub fn rise_angle_deg(&self) -> f32 {
        self.emitter.slots[0].rise_angle_deg
    }
}

impl Effect for BottomVolcanoEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = self.age_frames as u32;
        self.integrate_frames(target);
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let center = self.world_pos;
        for (_ec, slot) in self.emitter.active() {
            if slot.alpha_b <= 0.0 {
                continue;
            }
            out.push(EffectPrimitiveDraw::RadialRing {
                center,
                distance: slot.distance,
                rise_angle_rad: slot.rise_angle_deg.to_radians(),
                rot_start_rad: slot.rot_start_deg.to_radians(),
                full_arc_rad: slot.full_display_angle_deg.to_radians(),
                segments: SEGMENTS,
                height_scale: HEIGHT_SCALE,
                heights: slot.height,
                texture: self.params.texture,
                color: [1.0, 1.0, 1.0, slot.alpha_b.clamp(0.0, 1.0)],
                blend: BlendKind::Alpha,
            });
        }
    }
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
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

    fn step(e: &mut BottomVolcanoEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn draws(e: &BottomVolcanoEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn volcano_emits_one_ground_anchored_ribbon_with_petal_heights() {
        let mut e = BottomVolcanoEffect::new([7.0, 3.5, -2.0], VOLCANO_RED);
        step(&mut e, 1.0);
        let prims = draws(&e);
        assert_eq!(prims.len(), 1);
        match &prims[0] {
            EffectPrimitiveDraw::RadialRing {
                center,
                segments,
                heights,
                texture,
                ..
            } => {
                assert_eq!(*center, [7.0, 3.5, -2.0], "ground-anchored, not lifted");
                assert_eq!(*segments, SEGMENTS);
                assert!(heights.iter().any(|h| *h > 0.0), "petals must be non-zero");
                assert_eq!(*texture, "ring_red.tga");
            }
            other => panic!("expected RadialRing, got {other:?}"),
        }
    }

    #[test]
    fn distance_pulses_via_bloom_then_reset_cycle() {
        let mut e = BottomVolcanoEffect::new([0.0, 0.0, 0.0], VOLCANO_BLUE);
        let initial = e.distance();
        let mut saw_smaller = false;
        let mut saw_larger = false;
        for _ in 0..400 {
            step(&mut e, 1.0);
            let d = e.distance();
            if d < initial - 0.1 {
                saw_smaller = true;
            }
            if d > initial + 0.05 {
                saw_larger = true;
            }
        }
        assert!(
            saw_smaller && saw_larger,
            "distance must pulse (saw_smaller={saw_smaller}, saw_larger={saw_larger}) \
             across the bloom cycle",
        );
    }

    #[test]
    fn rise_angle_decreases_then_floors_or_resets() {
        let mut e = BottomVolcanoEffect::new([0.0, 0.0, 0.0], VOLCANO_GREEN);
        let initial = e.rise_angle_deg();
        assert_eq!(initial, BASE_RISE_ANGLE_DEG);
        step(&mut e, 30.0);
        assert!(
            e.rise_angle_deg() < initial,
            "rise_angle must decrease (got {} from {initial})",
            e.rise_angle_deg(),
        );
    }

    #[test]
    fn volcano_does_not_self_terminate() {
        let mut e = BottomVolcanoEffect::new([0.0, 0.0, 0.0], VOLCANO_GREEN);
        let mut status = EffectStatus::Running;
        for _ in 0..30 {
            status = step(&mut e, 20.0);
        }
        assert!(matches!(status, EffectStatus::Running));
    }
}
