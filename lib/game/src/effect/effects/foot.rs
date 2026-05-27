//! `EF_FOOT`..`EF_FOOT6` — footprint ground decals (ids 326-327, 447-448,
//! 703-704).
//!
//! A single flat textured quad laid on the ground at the
//! caster's position, alpha-blended, fading out over its lifetime. A flag
//! selects a slightly larger footprint for the "print" variants. The original
//! radius (5/6) is an engine-scaled aura, ~6× the on-screen
//! silhouette — the reference gif shows a small decal, so the world size is
//! tuned to that, not the literal number. The quad is drawn through the
//! same flat-on-ground path the aura decals use.
//!
//! The decal is yawed to point along the caster→target direction (from the
//! anchor trail) so a step reads as facing the way the actor moves.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, QuadPlane,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

/// Below this caster→target distance the trail carries no usable direction
/// (single-point anchor), so the footprint keeps its default orientation.
const MIN_DIR_DISTANCE: f32 = 0.001;

const FRAMES_PER_SECOND: f32 = 60.0;

/// Max alpha — 150 of 255.
const MAX_ALPHA: f32 = 150.0 / 255.0;
/// Raise the decal slightly off the ground to avoid z-fighting. Native RO
/// coordinates use `-Y = up`, so the lift is -0.2.
const GROUND_OFFSET_Y: f32 = -0.2;

/// Per-variant footprint parameters.
#[derive(Clone, Copy)]
pub struct FootParams {
    pub texture: &'static str,
    pub half_size: f32,
}

pub const FOOT: FootParams = FootParams { texture: "foot_l_b.tga", half_size: 2.5 };
pub const FOOT2: FootParams = FootParams { texture: "foot_r_b.tga", half_size: 2.5 };
pub const FOOT3: FootParams = FootParams { texture: "foot_l2.tga", half_size: 3.0 };
pub const FOOT4: FootParams = FootParams { texture: "foot_r2.tga", half_size: 3.0 };
// Foot5/Foot6 want `print_foot_l/r.tga`, which are absent
// from the classic GRF — fall back to the sibling footprint art so they still
// render a footprint rather than a white placeholder quad.
pub const FOOT5: FootParams = FootParams { texture: "foot_l2.tga", half_size: 3.0 };
pub const FOOT6: FootParams = FootParams { texture: "foot_r2.tga", half_size: 3.0 };

pub const TEXTURES: &[&str] = &[
    FOOT.texture,
    FOOT2.texture,
    FOOT3.texture,
    FOOT4.texture,
    FOOT5.texture,
    FOOT6.texture,
];

/// Parent emitter lifetime — 3400 ms.
pub const TOTAL_DURATION_MS: u32 = 3400;
const DURATION_SECS: f32 = TOTAL_DURATION_MS as f32 / 1000.0;
/// Fade-in over the first 5 frames, then hold, then fade out over the last
/// 40% of the lifetime.
const FADE_IN_SECS: f32 = 5.0 / FRAMES_PER_SECOND;
const FADE_OUT_START: f32 = DURATION_SECS * 0.6;

pub struct FootEffect {
    params: FootParams,
    world_pos: [f32; 3],
    /// Yaw around world Y so the footprint points along the caster→target
    /// direction. `0` (default `+X`) when no trail direction is available.
    yaw: f32,
    age: f32,
}

impl FootEffect {
    /// `from` is the footprint anchor (caster's feet); `to` gives the facing
    /// direction. A single-point anchor (`from == to`) leaves the default
    /// orientation.
    pub fn new(from: [f32; 3], to: [f32; 3], params: FootParams) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        // The sole texture's toe points up (V = 0), which maps to the quad's
        // `-right` axis under `HorizontalYaw`, so the toe must be aimed at the
        // target — same heading convention as the skill projectiles
        // (`dx.atan2(-dz)`), not `forward`-along-direction.
        let yaw = if (dx * dx + dz * dz).sqrt() > MIN_DIR_DISTANCE {
            dx.atan2(-dz)
        } else {
            0.0
        };
        Self { params, world_pos: from, yaw, age: 0.0 }
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
            status = e.update(&EffectUpdateCtx { delta: 1.0 / FRAMES_PER_SECOND, camera_target: None });
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
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        });
        assert_eq!(list.primitives.len(), 1);
        match &list.primitives[0] {
            EffectPrimitiveDraw::Texture3D { center, size, plane, color, texture, .. } => Quad {
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
        // The sole's toe is the quad's `-right` axis. For a target along +Z,
        // `yaw = atan2(0, -dz) = π` makes `-right` point at +Z.
        let e = FootEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 5.0], FOOT);
        let yaw = match only_quad(&e).plane {
            QuadPlane::HorizontalYaw(y) => y,
            other => panic!("expected HorizontalYaw, got {other:?}"),
        };
        let (sin, cos) = yaw.sin_cos();
        // toe direction = -right = (sin yaw, 0, -cos yaw) should align with +Z.
        assert!(sin.abs() < 1e-4 && (-cos - 1.0).abs() < 1e-4, "toe should face +Z (yaw={yaw})");

        // single-point anchor keeps the default orientation (yaw 0).
        let e0 = FootEffect::new([1.0, 0.0, 2.0], [1.0, 0.0, 2.0], FOOT);
        assert_eq!(only_quad(&e0).plane, QuadPlane::HorizontalYaw(0.0));
    }

    #[test]
    fn alpha_fades_in_then_out_and_effect_dies() {
        let mut e = FootEffect::new([0.0; 3], [0.0; 3], FOOT);
        let a_start = only_quad(&e).alpha;
        tick(&mut e, FADE_IN_SECS);
        let a_peak = only_quad(&e).alpha;
        tick(&mut e, DURATION_SECS - FADE_IN_SECS - 1.0 / FRAMES_PER_SECOND);
        let a_late = only_quad(&e).alpha;
        assert!(a_start < a_peak, "alpha should fade in");
        assert!(a_late < a_peak, "alpha should fade out before death");

        assert_eq!(tick(&mut e, 0.1), EffectStatus::Dead);
    }
}
