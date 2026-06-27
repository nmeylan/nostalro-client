use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::STONE_TEXTURE;
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const SPIKE_COUNT: usize = 3;
const TILT_DEG: f32 = 70.0;
const SIZE: f32 = 0.7;
const HEIGHT: f32 = 15.0;
const BASE_OFFSETS: [[f32; 2]; SPIKE_COUNT] = [[0.0, -3.0], [3.0, 1.5], [-3.0, 1.5]];
const HEADINGS_DEG: [f32; SPIKE_COUNT] = [0.0, 240.0, 120.0];

const SPIKE_SPEED_PER_S: f32 = 0.21 * FRAMES_PER_SECOND;
const SPEED_LIMIT_S: f32 = 10.0 / FRAMES_PER_SECOND;
const DURATION_FRAMES: f32 = 150.0;
const FADE_OUT_FRAMES: f32 = 20.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

struct Blade {
    base: [f32; 3],
    velocity: [f32; 3],
    heading_deg: f32,
}

pub struct GrimToothAtkEffect {
    blades: Vec<Blade>,
    age: f32,
}

impl GrimToothAtkEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let blades = (0..SPIKE_COUNT)
            .map(|i| {
                let [ox, oz] = BASE_OFFSETS[i];
                Blade {
                    base: [world_pos[0] + ox, world_pos[1], world_pos[2] + oz],
                    velocity: apex_velocity(TILT_DEG, HEADINGS_DEG[i], SPIKE_SPEED_PER_S),
                    heading_deg: HEADINGS_DEG[i],
                }
            })
            .collect();
        Self { blades, age: 0.0 }
    }

    fn duration_s(&self) -> f32 {
        DURATION_FRAMES / FRAMES_PER_SECOND
    }
}

impl Effect for GrimToothAtkEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        for blade in &mut self.blades {
            rise_step(
                &mut blade.base,
                blade.velocity,
                self.age,
                ctx.delta,
                SPEED_LIMIT_S,
            );
        }
        self.age += ctx.delta;
        if self.age >= self.duration_s() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = fade_tail_alpha(self.age, self.duration_s(), 1.0, FADE_OUT_FRAMES);
        for blade in &self.blades {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: blade.base,
                size: SIZE,
                height: HEIGHT,
                tilt_x_deg: TILT_DEG,
                rotation_y_deg: blade.heading_deg,
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
                rotation_y_deg,
                texture,
                height,
                ..
            } = p
            else {
                panic!("expected QuadHorn, got {p:?}");
            };
            assert_eq!(*texture, STONE_TEXTURE);
            assert!(*height > 5.0, "blades are tall");
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
        assert!((a0 - 1.0).abs() < 1e-4);

        let near_end = (DURATION_FRAMES - FADE_OUT_FRAMES / 2.0) / FRAMES_PER_SECOND;
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
        assert!(a_fade < 1.0, "alpha fades near end: {a_fade}");
    }
}
