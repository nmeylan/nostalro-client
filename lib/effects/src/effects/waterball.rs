//! `EF_WATERBALL` — Sage Waterball projectile (id 116).
//!
//! Reference: `../ro-effects/effects/imgs/100-150/116.gif` shows a textured
//! 3D water sphere with animated highlights, rendered as a full sphere so
//! it reads the same from any camera angle.
//!
//! Trajectory follows the sibling Jupiter-Thunder ball (`yupitel.rs`): a
//! constant-Y horizontal flight from caster to target at `dist / duration`
//! speed, lifted `Y_OFFSET` off the ground. No vertical arc. When spawned
//! without trail data (`from == to`) it sits in place and animates.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const TEXTURES: &[&str] = &["water_out_a.bmp", "water_out_b.bmp", "water_out_c.bmp"];

const FPS: f32 = 60.0;
const DURATION_FRAMES: f32 = 30.0;
const FRAMES_PER_TEX: f32 = 5.0;
const RADIUS: f32 = 1.75;
const Y_OFFSET: f32 = -5.0;
const TARGET_KILL_DISTANCE: f32 = 3.0;
const SPHERE_SIDES_LAT: u32 = 8;
const SPHERE_SIDES_LON: u32 = 16;
const SPIN_DEG_PER_FRAME: f32 = 6.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FPS * 1000.0) as u32;

pub struct WaterballEffect {
    from: [f32; 3],
    to: [f32; 3],
    age: f32,
    velocity: [f32; 3],
    is_trail: bool,
}

impl WaterballEffect {
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let dist = (dx * dx + dz * dz).sqrt();
        let is_trail = dist > TARGET_KILL_DISTANCE;
        let velocity = if dist > 0.001 {
            let speed_per_s = dist / DURATION_FRAMES * FPS;
            [dx / dist * speed_per_s, 0.0, dz / dist * speed_per_s]
        } else {
            [0.0; 3]
        };
        Self {
            from,
            to,
            age: 0.0,
            velocity,
            is_trail,
        }
    }

    fn current_pos(&self) -> [f32; 3] {
        [
            self.from[0] + self.velocity[0] * self.age,
            self.from[1] + Y_OFFSET,
            self.from[2] + self.velocity[2] * self.age,
        ]
    }

    fn reached_target(&self) -> bool {
        let pos = self.current_pos();
        let dx = pos[0] - self.to[0];
        let dz = pos[2] - self.to[2];
        (dx * dx + dz * dz).sqrt() <= TARGET_KILL_DISTANCE
    }
}

impl Effect for WaterballEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let done = self.age * FPS >= DURATION_FRAMES || (self.is_trail && self.reached_target());
        if done {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FPS;
        let tex_idx = (frame / FRAMES_PER_TEX) as usize % TEXTURES.len();
        out.push(EffectPrimitiveDraw::Sphere {
            center: self.current_pos(),
            radius: RADIUS,
            sides_lat: SPHERE_SIDES_LAT,
            sides_lon: SPHERE_SIDES_LON,
            longitude_offset: (frame * SPIN_DEG_PER_FRAME).to_radians(),
            longitude_arc: std::f32::consts::TAU,
            uv_repeat: [1.0, 1.0],
            texture: TEXTURES[tex_idx],
            color: [1.0, 1.0, 1.0, 1.0],
            blend: BlendKind::Alpha,
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

    fn step_n(e: &mut WaterballEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FPS,
                camera_target: None, caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn sphere_pos(e: &WaterballEffect) -> ([f32; 3], &'static str) {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        match &l.primitives[0] {
            EffectPrimitiveDraw::Sphere {
                center, texture, ..
            } => (*center, *texture),
            _ => panic!("expected Sphere"),
        }
    }

    #[test]
    fn travels_horizontally_toward_target_no_vertical_arc() {
        let mut e = WaterballEffect::new([0.0, 0.0, 0.0], [60.0, 0.0, 0.0]);
        let (p0, t0) = sphere_pos(&e);
        step_n(&mut e, 6);
        let (p1, t1) = sphere_pos(&e);
        assert!(p1[0] > p0[0], "advances along +X");
        assert_eq!(p0[1], p1[1], "constant Y, no vertical arc");
        assert_ne!(t0, t1, "texture cycled");
    }

    #[test]
    fn dies_on_reaching_target() {
        let mut e = WaterballEffect::new([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        let status = step_n(&mut e, 40);
        assert_eq!(status, EffectStatus::Dead);
    }
}
