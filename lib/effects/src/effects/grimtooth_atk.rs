//! `EF_GRIMTOOTHATK` — Assassin Cross Grimtooth impact (id 132).
//!
//! The original game launches three big
//! `stone.bmp` blades at frame 0, splayed 120° apart into a
//! tripod over the impact point. This is the "bigger spike" half of the
//! attack; the travelling small-spike trail is [`super::frost_diver`]'s
//! `GRIMTOOTH` param set.
//!
//! Per blade: tilt 75° (leaning out), headings
//! 0 / 240 / 120, width 0.9, height 25,
//! 10-frame speed-limit window. Heights are scaled to our world
//! units (~⅓ of the original, cf. `frost_diver`) and tuned against the gif.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::STONE_TEXTURE;
use crate::effects::spike_util::{
    FRAMES_PER_SECOND, apex_velocity, fade_tail_alpha, rise_step,
};

pub const TEXTURES: &[&str] = &[STONE_TEXTURE];

const SPIKE_COUNT: usize = 3;
/// Tilt 75°; a lower tilt leans the blades further out so the
/// tripod silhouette reads clearly from the side.
const TILT_DEG: f32 = 70.0;
const SIZE: f32 = 1.3;
const HEIGHT: f32 = 15.0;
/// XZ base offsets per blade (the original horizontal placement, scaled)
/// so the three blades fan out from a shared footprint.
const BASE_OFFSETS: [[f32; 2]; SPIKE_COUNT] = [[0.0, -3.0], [3.0, 1.5], [-3.0, 1.5]];
/// Headings 0 / 240 / 120, 120° apart.
const HEADINGS_DEG: [f32; SPIKE_COUNT] = [0.0, 240.0, 120.0];

const SPIKE_SPEED_PER_S: f32 = 0.21 * FRAMES_PER_SECOND;
/// Speed-limit window of 10 frames — blade grows then freezes.
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
            rise_step(&mut blade.base, blade.velocity, self.age, ctx.delta, SPEED_LIMIT_S);
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
                // Opaque brown stone — alpha keeps the colour (cf. grimtooth).
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
        // Sociable test: three big stone blades present at frame 0, each at a
        // distinct 120°-apart heading, all using stone.bmp; the effect ends
        // after its fixed duration.
        let mut e = GrimToothAtkEffect::new([5.0, 0.0, -2.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None });
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

        // Runs until the fixed duration, then dies.
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        while t < TOTAL_DURATION_MS as f32 / 1000.0 + 0.1 {
            status = e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            t += 1.0 / 60.0;
            if status == EffectStatus::Dead {
                break;
            }
        }
        assert_eq!(status, EffectStatus::Dead);
    }

    #[test]
    fn alpha_fades_in_final_window() {
        // Sociable test: full alpha until the fade tail, then it drops.
        let mut e = GrimToothAtkEffect::new([0.0, 0.0, 0.0]);
        e.update(&EffectUpdateCtx { delta: 0.0, camera_target: None, caster_yaw: None });
        let a0 = match &draws(&e)[0] {
            EffectPrimitiveDraw::QuadHorn { color, .. } => color[3],
            _ => unreachable!(),
        };
        assert!((a0 - 1.0).abs() < 1e-4);

        let near_end = (DURATION_FRAMES - FADE_OUT_FRAMES / 2.0) / FRAMES_PER_SECOND;
        let mut t = 0.0;
        while t < near_end {
            e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None });
            t += 1.0 / 60.0;
        }
        let a_fade = match draws(&e).first() {
            Some(EffectPrimitiveDraw::QuadHorn { color, .. }) => color[3],
            _ => 0.0,
        };
        assert!(a_fade < 1.0, "alpha fades near end: {a_fade}");
    }
}
