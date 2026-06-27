use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 180.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

pub const TEXTURES: &[&str] = &["pikapika2.bmp"];

const WORLD_SCALE: f32 = 1.0;
const SIZE: f32 = 10.0 * WORLD_SCALE * std::f32::consts::SQRT_2;
const Y_OFFSET: f32 = -12.0 * WORLD_SCALE;

const RAMP_FRAMES: f32 = 20.0;
const ALPHA_RAMP_PER_FRAME: f32 = 4.0 / 255.0;
const ALPHA_PEAK: f32 = 80.0 / 255.0;
const FADE_START_FRAME: f32 = 100.0;
const ALPHA_DRAIN_PER_FRAME: f32 = 1.0 / 255.0;
const PULSE_DEG_PER_FRAME: f32 = 3.0;
const COLOR: [f32; 3] = [150.0 / 255.0, 150.0 / 255.0, 250.0 / 255.0];

pub struct FirstaidEffect {
    world_pos: [f32; 3],
    rotation_deg: f32,
    process: f32,
    alpha: f32,
    pulse: f32,
}

impl FirstaidEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let seed = (world_pos[0] * 53.0 + world_pos[2] * 29.0) as i64 as u32 ^ 0x0BAD_F00D;
        Self {
            world_pos,
            rotation_deg: (seed % 360) as f32,
            process: 0.0,
            alpha: 0.0,
            pulse: ((seed >> 8) % 360) as f32,
        }
    }
}

impl Effect for FirstaidEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FRAMES_PER_SECOND;
        self.process += frames;
        self.pulse = (self.pulse + PULSE_DEG_PER_FRAME * frames) % 360.0;
        if self.process <= RAMP_FRAMES {
            self.alpha = (self.alpha + ALPHA_RAMP_PER_FRAME * frames).min(ALPHA_PEAK);
        } else if self.process > FADE_START_FRAME {
            self.alpha = (self.alpha - ALPHA_DRAIN_PER_FRAME * frames).max(0.0);
        }
        if self.process >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        if self.alpha <= 0.0 {
            return;
        }
        let [cx, cy, cz] = self.world_pos;
        let size = SIZE * (1.0 + 0.05 * self.pulse.to_radians().sin());
        let [r, g, b] = COLOR;
        out.push(EffectPrimitiveDraw::Billboard {
            pos: [cx, cy + Y_OFFSET, cz],
            size: [size, size],
            uv: [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            rotation: self.rotation_deg.to_radians(),
            texture: "pikapika2.bmp",
            color: [r, g, b, self.alpha],
            blend: BlendKind::Additive,
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

    fn run_to(e: &mut FirstaidEffect, frame: f32) {
        while e.process < frame {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn sparkle(e: &FirstaidEffect) -> Option<(f32, f32, [f32; 3])> {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        l.primitives.into_iter().find_map(|p| match p {
            EffectPrimitiveDraw::Billboard {
                color, size, pos, ..
            } => Some((color[3], size[0], pos)),
            _ => None,
        })
    }

    #[test]
    fn single_sparkle_above_caster_on_pikapika() {
        let mut e = FirstaidEffect::new([4.0, 1.0, 6.0]);
        run_to(&mut e, RAMP_FRAMES);
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        assert_eq!(l.primitives.len(), 1);
        match &l.primitives[0] {
            EffectPrimitiveDraw::Billboard { texture, pos, .. } => {
                assert_eq!(*texture, "pikapika2.bmp");
                assert!(pos[1] < 1.0, "floats above the caster's feet");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn alpha_ramps_in_then_fades_out() {
        let mut e = FirstaidEffect::new([0.0; 3]);
        run_to(&mut e, 3.0);
        let early = sparkle(&e).unwrap().0;
        run_to(&mut e, RAMP_FRAMES);
        let peak = sparkle(&e).unwrap().0;
        run_to(&mut e, TOTAL_FRAMES - 5.0);
        let late = sparkle(&e).unwrap().0;
        assert!(peak > early, "ramps in ({early} → {peak})");
        assert!(late < peak, "fades out ({peak} → {late})");
    }

    #[test]
    fn size_pulses_over_time() {
        let mut e = FirstaidEffect::new([0.0; 3]);
        run_to(&mut e, RAMP_FRAMES);
        let s0 = sparkle(&e).unwrap().1;
        run_to(&mut e, RAMP_FRAMES + 30.0);
        let s1 = sparkle(&e).unwrap().1;
        assert!((s0 - s1).abs() > 1e-4, "sparkle pulses ({s0} vs {s1})");
    }

    #[test]
    fn self_terminates() {
        let mut e = FirstaidEffect::new([0.0; 3]);
        run_to(&mut e, TOTAL_FRAMES - 1.0);
        assert_eq!(
            e.update(&EffectUpdateCtx {
                delta: 0.1,
                camera_target: None,
                caster_yaw: None
            }),
            EffectStatus::Dead
        );
    }
}
