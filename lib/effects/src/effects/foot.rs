use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const MIN_DIR_DISTANCE: f32 = 0.001;

const FRAMES_PER_SECOND: f32 = 60.0;

const MAX_ALPHA: f32 = 150.0 / 255.0;
/// Negative Y = up; lift off ground to avoid z-fighting.
const GROUND_OFFSET_Y: f32 = -0.2;

/// Per-variant footprint parameters.
#[derive(Clone, Copy)]
pub struct FootParams {
    pub texture: &'static str,
    pub half_size: f32,
}

pub const FOOT: FootParams = FootParams {
    texture: "foot_l_b.tga",
    half_size: 2.5,
};
pub const FOOT2: FootParams = FootParams {
    texture: "foot_r_b.tga",
    half_size: 2.5,
};
pub const FOOT3: FootParams = FootParams {
    texture: "foot_l2.tga",
    half_size: 3.0,
};
pub const FOOT4: FootParams = FootParams {
    texture: "foot_r2.tga",
    half_size: 3.0,
};
pub const FOOT5: FootParams = FootParams {
    texture: "foot_l2.tga",
    half_size: 3.0,
};
pub const FOOT6: FootParams = FootParams {
    texture: "foot_r2.tga",
    half_size: 3.0,
};

pub const TEXTURES: &[&str] = &[
    FOOT.texture,
    FOOT2.texture,
    FOOT3.texture,
    FOOT4.texture,
    FOOT5.texture,
    FOOT6.texture,
];

pub const TOTAL_DURATION_MS: u32 = 3400;
const DURATION_SECS: f32 = TOTAL_DURATION_MS as f32 / 1000.0;
const FADE_IN_SECS: f32 = 5.0 / FRAMES_PER_SECOND;
const FADE_OUT_START: f32 = DURATION_SECS * 0.6;

pub struct FootEffect {
    params: FootParams,
    world_pos: [f32; 3],
    yaw: f32,
    age: f32,
}

impl FootEffect {
    pub fn new(from: [f32; 3], to: [f32; 3], params: FootParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let yaw = if (dx * dx + dz * dz).sqrt() > MIN_DIR_DISTANCE {
            dx.atan2(-dz)
        } else {
            0.0
        };
        Self {
            params,
            world_pos: from,
            yaw,
            age: 0.0,
        }
    }

    fn alpha(&self) -> f32 {
        let a = if self.age < FADE_IN_SECS {
            self.age / FADE_IN_SECS
        } else if self.age < FADE_OUT_START {
            1.0
        } else {
            (1.0 - (self.age - FADE_OUT_START) / (DURATION_SECS - FADE_OUT_START)).max(0.0)
        };
        a * MAX_ALPHA
    }
}

impl Effect for FootEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        if self.age >= DURATION_SECS {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let h = self.params.half_size;
        let center = [
            self.world_pos[0],
            self.world_pos[1] + GROUND_OFFSET_Y,
            self.world_pos[2],
        ];
        out.push(EffectPrimitiveDraw::Texture3D {
            center,
            size: [h, h],
            plane: QuadPlane::HorizontalYaw(self.yaw),
            uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            texture: self.params.texture,
            color: [1.0, 1.0, 1.0, self.alpha()],
            blend: BlendKind::Alpha,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(e: &mut FootEffect, secs: f32) -> EffectStatus {
        let mut status = EffectStatus::Running;
        let steps = (secs * FRAMES_PER_SECOND).round() as usize;
        for _ in 0..steps {
            status = e.update(&EffectUpdateCtx {
                delta: 1.0 / FRAMES_PER_SECOND,
                camera_target: None,
                caster_yaw: None,
            });
        }
        status
    }

    struct Quad {
        center: [f32; 3],
        size: [f32; 2],
        alpha: f32,
        texture: &'static str,
        plane: QuadPlane,
    }

    fn only_quad(e: &FootEffect) -> Quad {
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 256.0,
                screen_h: 256.0,
                elapsed: 0.0,
            },
        );
        assert_eq!(list.primitives.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D {
                center,
                size,
                plane,
                color,
                texture,
                ..
            } => Quad {
                center: *center,
                size: *size,
                alpha: color[3],
                texture: *texture,
                plane: *plane,
            },
            _ => panic!("expected a Texture3D ground decal"),
        }
    }

    #[test]
    fn emits_ground_decal_with_variant_texture_above_ground() {
        let mut e = FootEffect::new([3.0, 0.0, 7.0], [3.0, 0.0, 7.0], FOOT3);
        tick(&mut e, FADE_IN_SECS); // reach full alpha
        let q = only_quad(&e);
        assert_eq!(q.texture, "foot_l2.tga");
        assert_eq!(q.size, [FOOT3.half_size, FOOT3.half_size]);
        assert_eq!(q.center[1], 0.0 + GROUND_OFFSET_Y);
    }

    #[test]
    fn toe_points_toward_target() {
        let e = FootEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 5.0], FOOT);
        let yaw = match only_quad(&e).plane {
            QuadPlane::HorizontalYaw(y) => y,
            other => panic!("expected HorizontalYaw, got {other:?}"),
        };
        let (sin, cos) = yaw.sin_cos();
        // toe direction = -right = (sin yaw, 0, -cos yaw) should align with +Z.
        assert!(
            sin.abs() < 1e-4 && (-cos - 1.0).abs() < 1e-4,
            "toe should face +Z (yaw={yaw})"
        );

        let e0 = FootEffect::new([1.0, 0.0, 2.0], [1.0, 0.0, 2.0], FOOT);
        assert_eq!(only_quad(&e0).plane, QuadPlane::HorizontalYaw(0.0));
    }

    #[test]
    fn alpha_fades_in_then_out_and_effect_dies() {
        let mut e = FootEffect::new([0.0; 3], [0.0; 3], FOOT);
        let a_start = only_quad(&e).alpha;
        tick(&mut e, FADE_IN_SECS);
        let a_peak = only_quad(&e).alpha;
        tick(
            &mut e,
            DURATION_SECS - FADE_IN_SECS - 1.0 / FRAMES_PER_SECOND,
        );
        let a_late = only_quad(&e).alpha;
        assert!(a_start < a_peak, "alpha should fade in");
        assert!(a_late < a_peak, "alpha should fade out before death");

        assert_eq!(tick(&mut e, 0.1), EffectStatus::Dead);
    }
}
