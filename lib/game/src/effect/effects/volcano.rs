//! VOLCANO family — four-emitter ground wreath of upward flame humps.
//!
//! Reference: original game `VOLCANO(tName, F1)` launching
//! GI ring-2 cell rendered by the GI ring-2 emitter +
//! the original game's 3D casting render. One helper, six effects:
//!
//! | EffectId         | Texture           | F1 | Visual                                |
//! |------------------|-------------------|----|---------------------------------------|
//! | Landprotector    | `ring_white.tga`  | 0  | White four-blade wreath               |
//! | Volcano          | `ring_red.tga`    | 0  | Red wreath                            |
//! | Deluge           | `ring_blue.tga`   | 0  | Blue wreath                           |
//! | Violentgale      | `magic_green.tga` | 0  | Green wreath                          |
//! | Ganbantein       | `ring_white.tga`  | 2  | Shorter flames, sharper rise, fast-out|
//! | Gumgang3         | `ring_yellow.tga` | 1  | Yellow wreath with slower fade-in     |
//!
//! Per-frame state and geometry are described in detail in this module's
//! original life as `land_protector.rs`. The shape is identical across all
//! variants: four `Frustum` emitters at slightly increasing radii, rotating
//! at +3°/frame, each carrying a half-sine wave that retracts every segment
//! except the one centred on its rotation start. The texture's baked stripes
//! are what produce the four-blade silhouette.
//!
//! `VolcanoParams` exposes the few scalars that the original game `F1` switch flips:
//!   * `max_flame_tilt` — total flame length (scaled from the original `max_height`
//!     by the same factor we apply to LandProtector; see lessons in the
//!     plan doc about the original game's literal numbers being ~6× the gif).
//!   * `initial_rise_angle_deg` — flame lean at frame 0.
//!   * `alpha_ramp_up_per_frame` / `alpha_ramp_down_per_frame` — curve speeds.
//!     `alpha_max` always reaches 200/255 (the original game's max alpha = 200).

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const NUM_EMITTERS: usize = 4;
const INITIAL_DISTANCE_BASE: f32 = 1.5;
const INITIAL_DISTANCE_STEP: f32 = 0.25;
const MIN_RISE_ANGLE_DEG: f32 = 40.0;
const RISE_DECAY_DEG_PER_FRAME: f32 = 1.0;
const ROT_DEG_PER_FRAME: f32 = 3.0;
const DISTANCE_GROWTH_PER_FRAME: f32 = 0.1;
/// max alpha = 200; common to every VOLCANO variant.
const ALPHA_MAX: f32 = 200.0;
/// 21 = the original game's subdivision count; we go a bit higher for a smoother hump.
const SIDES: u32 = 21;
/// One texture wrap per ring. The four-blade
/// silhouette is carried by the texture stripes, not procedural geometry.
const UV_REPEAT: f32 = 1.0;

/// Per-variant parameter set. One `pub const` per EF_* in the family.
#[derive(Clone, Copy, Debug)]
pub struct VolcanoParams {
    pub texture: &'static str,
    /// Total flame extension (split into outward + upward by `rise_angle`).
    /// LandProtector = 7.0 (original `max_height = 25` scaled to gif silhouette).
    pub max_flame_tilt: f32,
    /// `rise_angle` starts here and decays at -1°/frame down to 40°.
    pub initial_rise_angle_deg: f32,
    /// start alpha += per_frame during ramp-up. Original game default = 20.
    pub alpha_ramp_up_per_frame: f32,
    /// start alpha -= per_frame after hitting `ALPHA_MAX`. Original game default = 2.
    pub alpha_ramp_down_per_frame: f32,
}

impl VolcanoParams {
    const fn ramp_up_frames(&self) -> f32 {
        ALPHA_MAX / self.alpha_ramp_up_per_frame
    }
    const fn ramp_down_frames(&self) -> f32 {
        ALPHA_MAX / self.alpha_ramp_down_per_frame
    }
    const fn visible_frames(&self) -> f32 {
        self.ramp_up_frames() + self.ramp_down_frames()
    }
    /// Wall-clock duration of the visible burst, ms. Used as the spec's
    /// `Custom { duration_ms }` so the holder doesn't sit on a dead spawn.
    pub const fn total_duration_ms(&self) -> u32 {
        (self.visible_frames() * 1000.0 / FRAMES_PER_SECOND) as u32
    }
}

/// EF_LANDPROTECTOR — white wreath, F1=0.
pub const LANDPROTECTOR: VolcanoParams = VolcanoParams {
    texture: "ring_white.tga",
    max_flame_tilt: 7.0,
    initial_rise_angle_deg: 60.0,
    alpha_ramp_up_per_frame: 20.0,
    alpha_ramp_down_per_frame: 2.0,
};

/// EF_VOLCANO — red wreath, F1=0.
pub const VOLCANO: VolcanoParams = VolcanoParams {
    texture: "ring_red.tga",
    ..LANDPROTECTOR
};

/// EF_DELUGE — blue wreath, F1=0.
pub const DELUGE: VolcanoParams = VolcanoParams {
    texture: "ring_blue.tga",
    ..LANDPROTECTOR
};

/// EF_VIOLENTGALE — green wreath, F1=0.
pub const VIOLENTGALE: VolcanoParams = VolcanoParams {
    texture: "magic_green.tga",
    ..LANDPROTECTOR
};

/// EF_GANBANTEIN — `VOLCANO("ring_white.tga", 2)`. F1=2: the original game sets
/// `max_height = 15` (vs 25), `rise_angle = 70` (vs 80) and a faster
/// alpha-down (4/frame vs 2). Scaled to the same gif-silhouette factor we
/// use for LandProtector.
pub const GANBANTEIN: VolcanoParams = VolcanoParams {
    texture: "ring_white.tga",
    max_flame_tilt: 4.2,
    initial_rise_angle_deg: 52.0,
    alpha_ramp_up_per_frame: 20.0,
    alpha_ramp_down_per_frame: 4.0,
};

/// EF_GUMGANG3 — `VOLCANO("ring_yellow.tga", 1)`. F1=1: the original game halves the
/// alpha-up speed (start alpha += 10 instead of 20), keeping the rest.
pub const GUMGANG3: VolcanoParams = VolcanoParams {
    texture: "ring_yellow.tga",
    alpha_ramp_up_per_frame: 10.0,
    ..LANDPROTECTOR
};

pub const TEXTURES: &[&str] = &[
    "ring_white.tga",
    "ring_red.tga",
    "ring_blue.tga",
    "magic_green.tga",
    "ring_yellow.tga",
];

pub struct VolcanoEffect {
    params: VolcanoParams,
    world_pos: [f32; 3],
    age: f32,
}

impl VolcanoEffect {
    pub fn new(attach: Attach, params: VolcanoParams) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        Self {
            params,
            world_pos,
            age: 0.0,
        }
    }

    fn current_alpha(&self, frame: f32) -> f32 {
        let ramp_up = self.params.ramp_up_frames();
        let alpha = if frame < ramp_up {
            frame * self.params.alpha_ramp_up_per_frame
        } else {
            ALPHA_MAX - (frame - ramp_up) * self.params.alpha_ramp_down_per_frame
        };
        (alpha / 255.0).clamp(0.0, ALPHA_MAX / 255.0)
    }
}

impl Effect for VolcanoEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        let frame = self.age * FRAMES_PER_SECOND;
        if frame >= self.params.visible_frames() {
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

        let rise_deg = (self.params.initial_rise_angle_deg - frame * RISE_DECAY_DEG_PER_FRAME)
            .max(MIN_RISE_ANGLE_DEG);
        let rise_rad = rise_deg.to_radians();
        let max_outward = self.params.max_flame_tilt * rise_rad.cos();
        let max_upward = self.params.max_flame_tilt * rise_rad.sin();

        for ec in 0..NUM_EMITTERS {
            let radius = INITIAL_DISTANCE_BASE
                + ec as f32 * INITIAL_DISTANCE_STEP
                + DISTANCE_GROWTH_PER_FRAME * frame;
            let rotation_deg = ec as f32 * 90.0 + frame * ROT_DEG_PER_FRAME;
            let rotation_rad = rotation_deg.to_radians();

            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: radius,
                top_size: radius + max_outward,
                height: max_upward,
                sides: SIDES,
                rotation: rotation_rad,
                uv_repeat: UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: -self.params.max_flame_tilt,
                wave_frequency: 0.5,
                wave_phase: 0.0,
                texture: self.params.texture,
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

    fn draws(effect: &VolcanoEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut VolcanoEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx { dt })
    }

    fn frustum_fields(prim: &EffectPrimitiveDraw) -> (f32, f32, f32, &'static str) {
        match prim {
            EffectPrimitiveDraw::Frustum {
                bottom_size,
                rotation,
                color,
                texture,
                ..
            } => (*bottom_size, *rotation, color[3], texture),
            _ => panic!("expected Frustum"),
        }
    }

    #[test]
    fn each_variant_emits_four_emitters_with_its_texture() {
        for params in [LANDPROTECTOR, VOLCANO, DELUGE, VIOLENTGALE, GANBANTEIN, GUMGANG3] {
            let mut e = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), params);
            step(&mut e, 1.0 / FRAMES_PER_SECOND);
            let prims = draws(&e);
            assert_eq!(prims.len(), NUM_EMITTERS, "{} should emit 4 frustums", params.texture);
            for prim in &prims {
                let (_, _, _, tex) = frustum_fields(prim);
                assert_eq!(tex, params.texture);
            }
        }
    }

    #[test]
    fn gumgang3_ramps_up_slower_than_landprotector() {
        let mut lp = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), LANDPROTECTOR);
        let mut g3 = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), GUMGANG3);
        // Sample at the same early time; LP should be brighter than G3.
        let dt = 5.0 / FRAMES_PER_SECOND;
        step(&mut lp, dt);
        step(&mut g3, dt);
        let a_lp = frustum_fields(&draws(&lp)[0]).2;
        let a_g3 = frustum_fields(&draws(&g3)[0]).2;
        assert!(a_lp > a_g3, "LP {a_lp} should ramp faster than GUMGANG3 {a_g3}");
    }

    #[test]
    fn ganbantein_has_shorter_burst_than_landprotector() {
        // F1=2 doubles the alpha-down rate → ~half the fade-out duration.
        assert!(GANBANTEIN.total_duration_ms() < LANDPROTECTOR.total_duration_ms());
    }

    #[test]
    fn ring_grows_and_rotates_over_time() {
        let mut e = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), LANDPROTECTOR);
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        let (r0, rot0, _, _) = frustum_fields(&draws(&e)[0]);
        step(&mut e, (LANDPROTECTOR.visible_frames() / 2.0) / FRAMES_PER_SECOND);
        let (r1, rot1, _, _) = frustum_fields(&draws(&e)[0]);
        assert!(r1 > r0, "innermost ring should grow over time");
        assert!(rot1 > rot0, "rotation should advance over time");
    }

    #[test]
    fn alpha_ramps_up_then_down() {
        let mut e = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), LANDPROTECTOR);
        step(&mut e, 0.5 / FRAMES_PER_SECOND);
        let a_early = frustum_fields(&draws(&e)[0]).2;
        step(&mut e, (LANDPROTECTOR.ramp_up_frames() - 0.5) / FRAMES_PER_SECOND);
        let a_peak = frustum_fields(&draws(&e)[0]).2;
        step(
            &mut e,
            (LANDPROTECTOR.ramp_down_frames() * 0.8) / FRAMES_PER_SECOND,
        );
        let a_late = frustum_fields(&draws(&e)[0]).2;
        assert!(a_peak > a_early, "ramping up: {a_early} → {a_peak}");
        assert!(a_late < a_peak, "fading down: {a_peak} → {a_late}");
        assert!((a_peak - ALPHA_MAX / 255.0).abs() < 1e-4);
    }

    #[test]
    fn effect_dies_after_visible_burst() {
        let mut e = VolcanoEffect::new(Attach::WorldPos([0.0; 3]), LANDPROTECTOR);
        let dt = (LANDPROTECTOR.visible_frames() + 1.0) / FRAMES_PER_SECOND;
        assert_eq!(step(&mut e, dt), EffectStatus::Dead);
    }
}
