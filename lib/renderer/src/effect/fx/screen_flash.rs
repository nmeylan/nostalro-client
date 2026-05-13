use crate::camera::Camera;
use crate::effect::custom_effect::{CustomEffect, CustomParams, EffectRenderCtx};
use crate::effect::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, EffectUpdateCtx};

const FLASH_DURATION: f32 = 0.45;
const PEAK_AT: f32 = 0.18;
const PEAK_ALPHA: f32 = 0.85;
const QUAD_SIZE: f32 = 600.0;
const QUAD_CAMERA_OFFSET: f32 = 30.0;
const BASE_COLOR: [f32; 3] = [1.0, 1.0, 0.95];

pub struct ScreenFlash {
    tint: [f32; 4],
    age: f32,
    texture: &'static str,
}

impl ScreenFlash {
    pub fn new(params: &CustomParams) -> Self {
        Self {
            tint: params.tint.unwrap_or([1.0, 1.0, 1.0, 1.0]),
            age: 0.0,
            texture: params.texture.unwrap_or(""),
        }
    }
}

impl CustomEffect for ScreenFlash {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        if self.age > FLASH_DURATION {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        if self.age > FLASH_DURATION {
            return;
        }
        let alpha = current_alpha(self.age) * self.tint[3];
        if alpha <= 0.0 {
            return;
        }
        let pos = quad_position(ctx.camera);
        let uv = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let color = [
            BASE_COLOR[0] * self.tint[0],
            BASE_COLOR[1] * self.tint[1],
            BASE_COLOR[2] * self.tint[2],
            alpha,
        ];
        out.push(EffectPrimitiveDraw::Billboard {
            pos,
            size: [QUAD_SIZE, QUAD_SIZE],
            uv,
            texture: self.texture,
            color,
            blend: BlendKind::Additive,
        });
    }
}

fn current_alpha(age: f32) -> f32 {
    if age < PEAK_AT {
        PEAK_ALPHA * (age / PEAK_AT)
    } else {
        let decay = (age - PEAK_AT) / (FLASH_DURATION - PEAK_AT);
        PEAK_ALPHA * (1.0 - decay).max(0.0)
    }
}

fn quad_position(camera: &Camera) -> [f32; 3] {
    let dir = (camera.target - camera.eye()).normalize_or_zero();
    let p = camera.eye() + dir * QUAD_CAMERA_OFFSET;
    [p.x, p.y, p.z]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx<'static> {
        static CAM: std::sync::OnceLock<crate::camera::Camera> = std::sync::OnceLock::new();
        let cam = CAM.get_or_init(crate::camera::Camera::default);
        EffectRenderCtx {
            camera: cam,
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    #[test]
    fn alpha_rises_then_falls() {
        let mut flash = ScreenFlash::new(&CustomParams::default());
        flash.update(&EffectUpdateCtx { dt: PEAK_AT });
        let mut peak = EffectDrawList::new();
        flash.collect_draws(&mut peak, &ctx());
        let peak_alpha = match &peak.primitives[0] {
            EffectPrimitiveDraw::Billboard { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(peak_alpha > 0.5);

        flash.update(&EffectUpdateCtx {
            dt: FLASH_DURATION * 0.5,
        });
        let mut decay = EffectDrawList::new();
        flash.collect_draws(&mut decay, &ctx());
        let decay_alpha = match decay.primitives.first() {
            Some(EffectPrimitiveDraw::Billboard { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(decay_alpha < peak_alpha);
    }

    #[test]
    fn dies_after_duration() {
        let mut flash = ScreenFlash::new(&CustomParams::default());
        assert_eq!(
            flash.update(&EffectUpdateCtx {
                dt: FLASH_DURATION + 0.1
            }),
            EffectStatus::Dead
        );
    }
}
