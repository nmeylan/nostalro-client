use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURE: &str = "white02.bmp";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
pub const TOTAL_DURATION_MS: u32 = 30_000;
const TOTAL_DURATION_S: f32 = TOTAL_DURATION_MS as f32 / 1000.0;

const SPAWN_DELAY_FRAMES: f32 = 100.0;
const SPAWN_DELAY_S: f32 = SPAWN_DELAY_FRAMES / FRAMES_PER_SECOND;

const RADIUS: f32 = 8.0;
const FADE_IN_FRAMES: f32 = 15.0;
const FADE_OUT_FRAMES: f32 = 30.0;
const PEAK_ALPHA: f32 = 1.0;
const UV_REPEAT: f32 = 1.0;
const GROUND_OFFSET_Y: f32 = -1.0;

pub struct CallzoneEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl CallzoneEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
        }
    }

    fn visible_age_s(&self) -> Option<f32> {
        if self.age < SPAWN_DELAY_S {
            None
        } else {
            Some(self.age - SPAWN_DELAY_S)
        }
    }
}

impl Effect for CallzoneEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= TOTAL_DURATION_S {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let Some(visible_s) = self.visible_age_s() else {
            return;
        };
        let visible_frame = visible_s * FRAMES_PER_SECOND;
        let visible_total_frames = (TOTAL_DURATION_S - SPAWN_DELAY_S) * FRAMES_PER_SECOND;
        let fade_out_at = visible_total_frames - FADE_OUT_FRAMES;

        let alpha = if visible_frame <= FADE_IN_FRAMES {
            PEAK_ALPHA * (visible_frame / FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if visible_frame >= fade_out_at {
            let fade = ((visible_frame - fade_out_at) / FADE_OUT_FRAMES).clamp(0.0, 1.0);
            PEAK_ALPHA * (1.0 - fade)
        } else {
            PEAK_ALPHA
        };

        let center = [
            self.world_pos[0],
            self.world_pos[1] + GROUND_OFFSET_Y,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::GroundDisc {
            center,
            radius: RADIUS,
            thickness: RADIUS,
            rotation: 0.0,
            arc_angle_deg: 360.0,
            uv_repeat: UV_REPEAT,
            texture: TEXTURE,
            color: [1.0, 1.0, 1.0, alpha],
            blend: BlendKind::Alpha,
            no_depth: false,
            tilt_rad: 0.0,
            spin_rad: 0.0,
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

    fn draws(effect: &CallzoneEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut CallzoneEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn no_draw_before_spawn_delay_then_disc_appears() {
        let mut eff = CallzoneEffect::new([1.0, 2.0, 3.0]);
        step(&mut eff, 0.0);
        assert_eq!(draws(&eff).len(), 0, "disc hidden during 100-frame delay");

        step(&mut eff, SPAWN_DELAY_S + 0.5);
        let prims = draws(&eff);
        assert_eq!(prims.len(), 1, "single ground disc after delay");
        match &prims[0] {
            EffectPrimitiveDraw::GroundDisc {
                center,
                radius,
                arc_angle_deg,
                texture,
                ..
            } => {
                assert_eq!(*center, [1.0, 2.0 + GROUND_OFFSET_Y, 3.0]);
                assert_eq!(*radius, RADIUS);
                assert_eq!(*arc_angle_deg, 360.0);
                assert_eq!(*texture, TEXTURE);
            }
            _ => panic!("expected GroundDisc"),
        }
    }

    #[test]
    fn alpha_curve_fades_in_holds_then_fades_out() {
        let mut eff = CallzoneEffect::new([0.0; 3]);
        step(&mut eff, SPAWN_DELAY_S + 0.05);
        let a_early = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        step(&mut eff, TOTAL_DURATION_S * 0.5);
        let a_mid = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        step(&mut eff, TOTAL_DURATION_S * 0.45);
        let a_late = match &draws(&eff)[0] {
            EffectPrimitiveDraw::GroundDisc { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!(a_early < a_mid, "alpha rises during fade-in");
        assert!(a_late < a_mid, "alpha drops during fade-out");
    }

    #[test]
    fn dies_after_total_duration() {
        let mut eff = CallzoneEffect::new([0.0; 3]);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_S * 1.2 {
            status = step(&mut eff, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
