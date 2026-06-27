use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "alpha_down.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const SIDES: u32 = 10;
const RADIUS: f32 = 4.5;
const INITIAL_HEIGHT: f32 = 10.0;
const HEIGHT_ACCEL_PER_FRAME: f32 = 0.01;
const ALPHA_MAX: f32 = 90.0 / 255.0;
const ALPHA_RAMP_FRAMES: f32 = 20.0;
const FADE_OUT_FRAMES: f32 = 10.0;

#[derive(Clone, Copy, Debug)]
pub struct PotionPillarParams {
    pub height_speed: f32,
    pub duration_frames: u32,
}

pub const DEFAULT: PotionPillarParams = PotionPillarParams {
    height_speed: 1.0,
    duration_frames: 50,
};

pub const BERSERK: PotionPillarParams = PotionPillarParams {
    height_speed: 1.0,
    duration_frames: 50,
};

impl PotionPillarParams {
    pub const fn duration_ms(&self) -> u32 {
        (self.duration_frames as f32 * 1000.0 / FRAMES_PER_SECOND) as u32
    }
}

pub const TOTAL_DURATION_MS: u32 = DEFAULT.duration_ms();

pub struct PotionPillarEffect {
    params: PotionPillarParams,
    world_pos: [f32; 3],
    age: f32,
}

impl PotionPillarEffect {
    pub fn new(world_pos: [f32; 3], params: PotionPillarParams) -> Self {
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }

    fn current_height(&self, frame: f32) -> f32 {
        // Discrete integration: speed += accel each frame, then height +=
        // speed. With constant accel that's the closed-form
        // `h0 + v0*t + a*t*(t+1)/2`. The +1 mirrors the integration order
        // (speed updated before size).
        INITIAL_HEIGHT
            + self.params.height_speed * frame
            + HEIGHT_ACCEL_PER_FRAME * frame * (frame + 1.0) * 0.5
    }

    fn current_alpha(&self, frame: f32) -> f32 {
        let fade_start = self.params.duration_frames as f32 - FADE_OUT_FRAMES;
        if frame < ALPHA_RAMP_FRAMES {
            ALPHA_MAX * (frame / ALPHA_RAMP_FRAMES)
        } else if frame < fade_start {
            ALPHA_MAX
        } else {
            let t = (self.params.duration_frames as f32 - frame).max(0.0) / FADE_OUT_FRAMES;
            ALPHA_MAX * t
        }
    }
}

impl Effect for PotionPillarEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let frame = self.age * FRAMES_PER_SECOND;
        if frame >= self.params.duration_frames as f32 {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let alpha = self.current_alpha(frame);
        if alpha <= 0.0 {
            return;
        }
        let height = self.current_height(frame);

        out.push(EffectPrimitiveDraw::Cylinder {
            base: self.world_pos,
            bottom_size: RADIUS,
            top_size: RADIUS,
            height,
            sides: SIDES,
            rotation: 0.0,
            tilt_x_rad: 0.0,
            rotation_y_rad: 0.0,
            uv_scroll: [0.0, 0.0],
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            alpha_bottom: alpha,
            blend: BlendKind::Alpha,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step_and_collect(e: &mut PotionPillarEffect, dt: f32) -> Vec<EffectPrimitiveDraw> {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
    }

    #[test]
    fn pillar_grows_and_ramps_in_then_fades() {
        let mut e = PotionPillarEffect::new([1.0, 2.0, 3.0], DEFAULT);
        let prims_early = step_and_collect(&mut e, 5.0 / FRAMES_PER_SECOND);
        let (h_early, a_early) = match &prims_early[0] {
            EffectPrimitiveDraw::Cylinder {
                base,
                bottom_size,
                top_size,
                height,
                color,
                texture,
                ..
            } => {
                assert_eq!(*base, [1.0, 2.0, 3.0]);
                assert!((*bottom_size - *top_size).abs() < f32::EPSILON);
                assert_eq!(*texture, TEXTURE);
                (*height, color[3])
            }
            other => panic!("expected Cylinder, got {other:?}"),
        };
        assert!(h_early > INITIAL_HEIGHT, "pillar lengthens immediately");
        assert!(a_early < ALPHA_MAX, "still ramping in at frame ~5");

        let prims_hold = step_and_collect(&mut e, 20.0 / FRAMES_PER_SECOND);
        let a_hold = match &prims_hold[0] {
            EffectPrimitiveDraw::Cylinder { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a_hold - ALPHA_MAX).abs() < 1e-4, "alpha holds at max");
    }

    #[test]
    fn dies_at_duration() {
        let mut e = PotionPillarEffect::new([0.0; 3], DEFAULT);
        let total_s = DEFAULT.duration_frames as f32 / FRAMES_PER_SECOND;
        let s = e.update(&EffectUpdateCtx {
            delta: total_s + 0.1,
            camera_target: None,
            caster_yaw: None,
        });
        assert!(matches!(s, EffectStatus::Dead));
    }
}
