use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const FIREIVY_TEXTURE: &str = "fire_ivy.bmp";
pub const TEXTURES: &[&str] = &[FIREIVY_TEXTURE];

const FPS: f32 = 60.0;
const DURATION_FRAMES: f32 = 30.0;
const SIZE: [f32; 2] = [3.5, 3.5];
const Y_OFFSET: f32 = -5.0;
const TARGET_KILL_DISTANCE: f32 = 3.0;
pub const TOTAL_DURATION_MS: u32 = (DURATION_FRAMES / FPS * 1000.0) as u32;
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

pub struct FireivyEffect {
    from: [f32; 3],
    to: [f32; 3],
    age: f32,
    velocity: [f32; 3],
    is_trail: bool,
}

impl FireivyEffect {
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

impl Effect for FireivyEffect {
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
        out.push(EffectPrimitiveDraw::Billboard {
            pos: self.current_pos(),
            size: SIZE,
            uv: UNIT_UV,
            rotation: 0.0,
            texture: FIREIVY_TEXTURE,
            color: [1.0, 1.0, 1.0, 1.0],
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

    fn step_n(e: &mut FireivyEffect, n: u32) -> EffectStatus {
        let mut s = EffectStatus::Running;
        for _ in 0..n {
            s = e.update(&EffectUpdateCtx {
                delta: 1.0 / FPS,
                camera_target: None,
                caster_yaw: None,
            });
            if s == EffectStatus::Dead {
                break;
            }
        }
        s
    }

    fn pos(e: &FireivyEffect) -> [f32; 3] {
        let mut l = EffectDrawList::new();
        e.collect_draws(&mut l, &render_ctx());
        match &l.primitives[0] {
            EffectPrimitiveDraw::Billboard { pos, .. } => *pos,
            _ => panic!("expected Billboard"),
        }
    }

    #[test]
    fn travels_horizontally_no_vertical_arc() {
        let mut e = FireivyEffect::new([0.0, 0.0, 0.0], [40.0, 0.0, 0.0]);
        let p0 = pos(&e);
        step_n(&mut e, 10);
        let p1 = pos(&e);
        assert!(p1[0] > p0[0]);
        assert_eq!(p0[1], p1[1], "constant Y, no vertical arc");
    }

    #[test]
    fn dies_on_reaching_target() {
        let mut e = FireivyEffect::new([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        assert_eq!(step_n(&mut e, 40), EffectStatus::Dead);
    }
}
