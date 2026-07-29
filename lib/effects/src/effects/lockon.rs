//! `EF_LOCKON` — lock-on targeting reticle (id 60).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const LOCKON_TEXTURE: &str = "lockon128.tga";
pub const TEXTURES: &[&str] = &[LOCKON_TEXTURE];

const STEADY_HALF_FALLBACK: f32 = 8.0;
/// Snap-in starts this many times larger than the steady size (30/8).
const START_SCALE: f32 = 30.0 / 8.0;
const CHANGE_POINT_FRAMES: f32 = 22.0;
const DEG_PER_FRAME: f32 = -4.5;
const FRAMES_PER_SECOND: f32 = 60.0;
const QUAD_OFFSET: f32 = std::f32::consts::FRAC_PI_4;
const RGB_RED: i32 = 250;
const RGB_GB_START: i32 = 150;
const RGB_GB_SPEED: i32 = -5;
const RGB_CYCLE_DELAY: i32 = 20;
const RGB_MAX: i32 = 254;
/// −Y is up; lift prevents terrain from depth-occluding the flat quad.
const GROUND_LIFT: f32 = -0.3;

pub struct LockonEffect {
    world_pos: [f32; 3],
    steady_half: f32,
    age: f32,
    gb: i32,
    cycle_cnt: i32,
    frame_accum: f32,
}

impl LockonEffect {
    pub fn new(world_pos: [f32; 3], target_size: Option<[f32; 2]>) -> Self {
        let steady_half = match target_size {
            Some([w, h]) => w.max(h) * 0.5,
            None => STEADY_HALF_FALLBACK,
        };
        Self {
            world_pos,
            steady_half,
            age: 0.0,
            gb: RGB_GB_START,
            cycle_cnt: 0,
            frame_accum: 0.0,
        }
    }

    fn half_size(&self) -> f32 {
        let frames = self.age * FRAMES_PER_SECOND;
        let t = (frames / CHANGE_POINT_FRAMES).clamp(0.0, 1.0);
        let scale = START_SCALE + (1.0 - START_SCALE) * t;
        self.steady_half * scale
    }

    fn step_rgb_cycle(&mut self) {
        if self.gb == RGB_MAX {
            self.cycle_cnt += 1;
            if self.cycle_cnt == RGB_CYCLE_DELAY {
                self.gb += RGB_GB_SPEED;
                self.cycle_cnt = 0;
            }
        } else {
            self.gb += RGB_GB_SPEED;
        }
        if self.gb > RGB_MAX {
            self.gb = 0;
        } else if self.gb < 0 {
            self.gb = RGB_MAX;
        }
    }

    fn color(&self) -> [f32; 4] {
        let gb = self.gb as f32 / 255.0;
        [RGB_RED as f32 / 255.0, gb, gb, 1.0]
    }
}

impl Effect for LockonEffect {
    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
    }

    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frame_accum += ctx.delta * FRAMES_PER_SECOND;
        while self.frame_accum >= 1.0 {
            self.step_rgb_cycle();
            self.frame_accum -= 1.0;
        }
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frames = self.age * FRAMES_PER_SECOND;
        let yaw = (frames * DEG_PER_FRAME).to_radians();
        let half = self.half_size();
        let color = self.color();
        let center = [
            self.world_pos[0],
            self.world_pos[1] + GROUND_LIFT,
            self.world_pos[2],
        ];
        for offset in [0.0, QUAD_OFFSET] {
            out.push(EffectPrimitiveDraw::Texture3D {
                center,
                size: [half, half],
                plane: QuadPlane::HorizontalYaw(yaw + offset),
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                texture: LOCKON_TEXTURE,
                color,
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

    fn yaws_and_size(e: &LockonEffect) -> ([f32; 2], f32) {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.primitives.len(), 2, "lock-on emits two crossed quads");
        let mut yaws = [0.0; 2];
        let mut size = 0.0;
        for (i, prim) in list.primitives.iter().enumerate() {
            match prim {
                EffectPrimitiveDraw::Texture3D {
                    plane: QuadPlane::HorizontalYaw(y),
                    size: s,
                    ..
                } => {
                    yaws[i] = *y;
                    size = s[0];
                }
                _ => panic!("expected Texture3D::HorizontalYaw"),
            }
        }
        (yaws, size)
    }

    fn advance(e: &mut LockonEffect, frames: u32) {
        for _ in 0..frames {
            assert_eq!(
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / FRAMES_PER_SECOND,
                    camera_target: None,
                    caster_yaw: None,
                }),
                EffectStatus::Running
            );
        }
    }

    #[test]
    fn snaps_in_then_holds_and_scales_to_target() {
        let mut e = LockonEffect::new([0.0, 0.0, 0.0], Some([16.0, 16.0]));

        let (yaws_a, size_a) = yaws_and_size(&e);
        assert!(
            (yaws_a[1] - yaws_a[0] - QUAD_OFFSET).abs() < 1e-5,
            "second quad is offset 45deg in-plane"
        );

        advance(&mut e, 30);
        let (yaws_b, size_b) = yaws_and_size(&e);
        assert_eq!(
            (yaws_b[0] - yaws_a[0]).signum(),
            DEG_PER_FRAME.signum(),
            "reticle spins in the DEG_PER_FRAME direction"
        );
        assert!(size_b < size_a, "reticle snaps inward at spawn");

        let color = e.color();
        assert!(
            color[1] <= 0.02 && color[2] <= 0.02,
            "reticle reaches pure red"
        );

        advance(&mut e, 30);
        let (_, size_c) = yaws_and_size(&e);
        advance(&mut e, 60);
        let (_, size_d) = yaws_and_size(&e);
        assert!(
            (size_d - size_c).abs() < 1e-5,
            "reticle size is constant after the change point"
        );

        let mut big = LockonEffect::new([0.0, 0.0, 0.0], Some([32.0, 32.0]));
        advance(&mut big, 60);
        let (_, big_steady) = yaws_and_size(&big);
        assert!(big_steady > size_d, "bigger target => bigger reticle");
    }
}
