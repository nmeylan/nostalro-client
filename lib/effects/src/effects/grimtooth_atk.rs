use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::STONE_TEXTURE;
use crate::effects::spike_util::{
    FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, speed_limited_travel,
};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const BASE_OFFSETS: [[f32; 3]; 3] = [[0.0, 40.0, -12.0], [12.0, 40.0, 6.0], [-12.0, 40.0, 6.0]];
const HEADINGS_DEG: [f32; 3] = [0.0, 240.0, 120.0];
const TILT_DEG: f32 = 75.0;
const SIZE: f32 = 0.9;
const HEIGHT: f32 = 25.0;
const RISE_SPEED: f32 = 3.5;
const RISE_ACCEL: f32 = 0.001;
const RISE_LAST_FRAME: f32 = 10.0;
const ALPHA: f32 = 254.0 / 255.0;
const DURATION_FRAMES: f32 = 1000.0;
const FADE_OUT_FRAMES: f32 = 10.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub struct GrimToothAtkEffect {
    origin: [f32; 3],
    age: f32,
}

impl GrimToothAtkEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            origin: world_pos,
            age: 0.0,
        }
    }

    fn duration_s(&self) -> f32 {
        DURATION_FRAMES / FRAMES_PER_SECOND
    }
}

impl Effect for GrimToothAtkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= self.duration_s() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = fade_tail_alpha(self.age, self.duration_s(), ALPHA, FADE_OUT_FRAMES);
        let travel = speed_limited_travel(
            self.age * FRAMES_PER_SECOND,
            RISE_SPEED,
            RISE_ACCEL,
            RISE_LAST_FRAME,
        );
        for (offset, heading_deg) in BASE_OFFSETS.iter().zip(HEADINGS_DEG) {
            let dir = apex_velocity(TILT_DEG, heading_deg, travel);
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: [
                    self.origin[0] + offset[0] + dir[0],
                    self.origin[1] + offset[1] + dir[1],
                    self.origin[2] + offset[2] + dir[2],
                ],
                size: SIZE,
                height: HEIGHT,
                tilt_x_deg: TILT_DEG,
                rotation_y_deg: heading_deg,
                texture: STONE_TEXTURE,
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

    fn draws(e: &GrimToothAtkEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_three_splayed_blades_then_dies() {
        let mut e = GrimToothAtkEffect::new([5.0, 0.0, -2.0]);
        e.update(&EffectUpdateCtx {
            delta: 0.0,
            camera_target: None,
            caster_yaw: None,
        });
        let prims = draws(&e);
        assert_eq!(prims.len(), 3);

        let mut headings = Vec::new();
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn {
                base,
                rotation_y_deg,
                texture,
                ..
            } = p
            else {
                panic!("expected QuadHorn, got {p:?}");
            };
            assert_eq!(*texture, STONE_TEXTURE);
            assert!(base[1] > 30.0, "blades start buried (+Y is down)");
            headings.push(*rotation_y_deg);
        }
        headings.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(headings, vec![0.0, 120.0, 240.0]);

        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_MS as f32 / 1000.0 + 0.1 {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
            t += 1.0 / 60.0;
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn alpha_fades_in_final_window() {
        let mut e = GrimToothAtkEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx {
            delta: 0.0,
            camera_target: None,
            caster_yaw: None,
        });
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a0 - ALPHA).abs() < 1e-4);

        let near_end = (DURATION_FRAMES - FADE_OUT_FRAMES / 4.0) / FRAMES_PER_SECOND;
        let mut t = 0.0;
        while t < near_end {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
            t += 1.0 / 60.0;
        }
        let a_fade = match draws(&e).first() {
            Some(EffectPrimitiveDraw::QuadHorn { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_fade < ALPHA / 2.0, "alpha fades near end: {a_fade}");
    }
}
