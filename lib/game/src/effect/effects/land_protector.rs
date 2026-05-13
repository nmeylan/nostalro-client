//! EF_LANDPROTECTOR — wreath of upward flame humps bursting from a point.
//!
//! Reference: original game `VOLCANO("ring_white.tga", 0)`
//! launching GI ring primitives +
//! the original game's 3D casting render. Visual ground truth:
//! `../ro-effects/effects/imgs/200-250/238.gif` (id 238 in the gif library).
//!
//! VOLCANO launches **four** emitters per spawn (`ec = 0..3`):
//!   * `distance = ec + 1` — base radii 1, 2, 3, 4.
//!   * `rot_start_deg = 90 * ec` — initial rotation offsets 0°, 90°, 180°, 270°.
//!   * Each emitter draws a full 360° quad strip whose per-segment top
//!     vertex height follows a half-sine across the ring (`sin(0..180°)`,
//!     peaking opposite rot_start). One emitter = one flame hump.
//!   * Combined, the four offset humps at different radii read as a
//!     rotating flame wreath in the gif.
//!
//! Per-frame state (from the GI ring-2 emitter):
//!   * `rot_start_deg += 3°` (rotation around the centre)
//!   * `distance += 0.1` (the ring grows outward)
//!   * `rise_angle -= 1°`, clamped to ≥ 40° (flames lean further out as
//!     they age; we keep this for parity but the rise direction shows up
//!     mostly as alpha-hidden expansion)
//!   * start alpha: ramps up `+20/frame` for 10 frames, then `-2/frame` until
//!     it hits 0 (~110 frames total visible).
//!
//! We map each original game emitter to one `Frustum` primitive shaped as a cylinder
//! (bottom_size == top_size = radius) with a half-sine wave (`wave_frequency
//! = 0.5`, so the argument sweeps `0..π` around the ring and the wave stays
//! non-negative — a single hump per emitter). The wave drives the vertical
//! displacement, and the four emitters share parameters except `bottom_size`
//! and `rotation`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

pub const TEXTURE: &str = "ring_white.tga";
pub const TEXTURES: &[&str] = &[TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
const NUM_EMITTERS: usize = 4;
/// original game uses world-unit distances 1, 2, 3, 4 at spawn. We follow the same
/// scale (1:1, like warp and magnum_break) but pack the four rings tighter
/// (`STEP = 0.25` instead of original game's 1.0) so the four flame blades merge
/// into a single visual wreath instead of reading as concentric circles.
/// The ring then grows over time at +0.1 world units / frame, so the gif's
/// expand-and-fade arc reads naturally — innermost ring starts at 1.5,
/// reaches ~12 by the time alpha hits 0.
const INITIAL_DISTANCE_BASE: f32 = 1.5;
const INITIAL_DISTANCE_STEP: f32 = 0.25;
/// original game `max_height = 25.0`. Scaled down to keep flames proportional to
/// the wreath silhouette (gif shows flames ≈ 1× the wreath radius, not
/// 6× as original game's literal numbers would give). This is the total tilt length
/// of each flame; it splits into outward + upward via `rise_angle`.
const MAX_FLAME_TILT: f32 = 7.0;
/// original game's `rise_angle` starts at 80° and decays at 1°/frame down to 40°.
/// 90° = purely vertical, 0° = flat on the ground. Lower → more "splat".
/// The gif's peak-visibility frames show ≈ 50–60° (notable outward lean);
/// we lower the initial value below original game's literal 80° so the lean reads
/// from the first visible frame.
const INITIAL_RISE_ANGLE_DEG: f32 = 60.0;
const MIN_RISE_ANGLE_DEG: f32 = 40.0;
const RISE_DECAY_DEG_PER_FRAME: f32 = 1.0;
/// original game's rotation start += 3 per frame. About one full revolution per ~2 s.
const ROT_DEG_PER_FRAME: f32 = 3.0;
/// original game `distance += 0.1` per frame, in original game-units.
const DISTANCE_GROWTH_PER_FRAME: f32 = 0.1;
/// original game alpha += 20 for 10 frames → reaches 200/255 then decays at 2/frame.
const ALPHA_RAMP_UP_FRAMES: f32 = 10.0;
const ALPHA_RAMP_UP_PER_FRAME: f32 = 20.0;
const ALPHA_RAMP_DOWN_PER_FRAME: f32 = 2.0;
const ALPHA_MAX: f32 = 200.0;
/// 10 frames up to 200, then 200/2 = 100 frames down to 0.
const VISIBLE_FRAMES: f32 = ALPHA_RAMP_UP_FRAMES + ALPHA_MAX / ALPHA_RAMP_DOWN_PER_FRAME;
/// `~1833 ms`. The default duration table reports 9990 ms for this id; the
/// visible animation only runs for one burst and the rest is dead air, so
/// the spec uses this constant.
pub const TOTAL_DURATION_MS: u32 = (VISIBLE_FRAMES * 1000.0 / FRAMES_PER_SECOND) as u32;
/// Resolution of the half-sine hump. original game uses 21 (E_DIVISION); we use
/// more for a smoother flame silhouette without visible facets.
const SIDES: u32 = 21;
/// Original game's rect renderer sets Tx = TexPart / divisions per segment, so
/// the texture wraps **once** around the full 360° ring (`Tx` ∈ [0, 1]).
/// The "shuriken" appearance — four prominent vertical flame tips at 90°
/// offsets — comes directly from `ring_white.tga`'s baked-in stripes, not
/// from procedural geometry. Tiling > 1 smears the stripes into a uniform
/// glow and loses the shape entirely.
const UV_REPEAT: f32 = 1.0;

pub struct LandProtectorEffect {
    world_pos: [f32; 3],
    age: f32,
}

impl LandProtectorEffect {
    pub fn new(attach: Attach) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        Self {
            world_pos,
            age: 0.0,
        }
    }
}

fn current_alpha(frame: f32) -> f32 {
    let alpha = if frame < ALPHA_RAMP_UP_FRAMES {
        frame * ALPHA_RAMP_UP_PER_FRAME
    } else {
        ALPHA_MAX - (frame - ALPHA_RAMP_UP_FRAMES) * ALPHA_RAMP_DOWN_PER_FRAME
    };
    (alpha / 255.0).clamp(0.0, ALPHA_MAX / 255.0)
}

impl Effect for LandProtectorEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        let frame = self.age * FRAMES_PER_SECOND;
        if frame >= VISIBLE_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let frame = self.age * FRAMES_PER_SECOND;
        let alpha = current_alpha(frame);
        if alpha <= 0.0 {
            return;
        }

        let rise_deg = (INITIAL_RISE_ANGLE_DEG - frame * RISE_DECAY_DEG_PER_FRAME)
            .max(MIN_RISE_ANGLE_DEG);
        let rise_rad = rise_deg.to_radians();
        let max_outward = MAX_FLAME_TILT * rise_rad.cos();
        let max_upward = MAX_FLAME_TILT * rise_rad.sin();

        for ec in 0..NUM_EMITTERS {
            let radius = INITIAL_DISTANCE_BASE
                + ec as f32 * INITIAL_DISTANCE_STEP
                + DISTANCE_GROWTH_PER_FRAME * frame;
            // original game's rotation start is per-emitter (90° apart) plus a global advance.
            let rotation_deg = ec as f32 * 90.0 + frame * ROT_DEG_PER_FRAME;
            let rotation_rad = rotation_deg.to_radians();

            // Cone at full extension by default (top displaced outward +
            // upward from the bottom ring along the rise-angle direction).
            // A negative wave pulls each segment's top vertex back along
            // the same tilt direction until it coincides with the bottom
            // — wave = 0 → full flame at this segment, wave = -MAX_TILT →
            // no flame. `wave_frequency = 0.5` puts the sin curve in
            // `[0, π]` across the ring → the hump sits at `local_angle = 0`
            // (i.e. at the rotation start) and the retraction is at the
            // opposite side.
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: radius,
                top_size: radius + max_outward,
                height: max_upward,
                sides: SIDES,
                rotation: rotation_rad,
                uv_repeat: UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: -MAX_FLAME_TILT,
                wave_frequency: 0.5,
                wave_phase: 0.0,
                texture: TEXTURE,
                color: [1.0, 1.0, 1.0, alpha],
                // original game size = 0 → standard alpha blend (inverse source alpha
                // blend), not additive. Additive blends multiple ring layers
                // into a uniform white glow and washes out the texture's
                // stripes — alpha blending preserves them.
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

    fn draws(effect: &LandProtectorEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut LandProtectorEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx { dt })
    }

    fn frustum_fields(prim: &EffectPrimitiveDraw) -> (f32, f32, f32) {
        // Returns (bottom_size, rotation_rad, alpha).
        match prim {
            EffectPrimitiveDraw::Frustum {
                bottom_size,
                rotation,
                color,
                ..
            } => (*bottom_size, *rotation, color[3]),
            _ => panic!("expected Frustum"),
        }
    }

    #[test]
    fn emits_four_concentric_humps_with_growing_radii() {
        let mut lp = LandProtectorEffect::new(Attach::WorldPos([0.0; 3]));
        step(&mut lp, 0.05);
        let prims = draws(&lp);
        assert_eq!(prims.len(), NUM_EMITTERS, "one frustum per original game emitter");
        let radii: Vec<f32> = prims.iter().map(|p| frustum_fields(p).0).collect();
        for win in radii.windows(2) {
            assert!(
                win[1] > win[0],
                "emitters must be at strictly increasing radii: got {radii:?}"
            );
        }
    }

    #[test]
    fn rotations_start_90_degrees_apart() {
        let mut lp = LandProtectorEffect::new(Attach::WorldPos([0.0; 3]));
        // Need alpha > 0 for the draws to be emitted.
        step(&mut lp, 1.0 / FRAMES_PER_SECOND);
        let prims = draws(&lp);
        let rots: Vec<f32> = prims.iter().map(|p| frustum_fields(p).1).collect();
        for i in 0..NUM_EMITTERS - 1 {
            let d = (rots[i + 1] - rots[i]).rem_euclid(std::f32::consts::TAU);
            assert!(
                (d - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
                "expected 90° offset between consecutive emitters, got {d}"
            );
        }
    }

    #[test]
    fn ring_grows_and_rotates_over_time() {
        let mut lp = LandProtectorEffect::new(Attach::WorldPos([0.0; 3]));
        step(&mut lp, 1.0 / FRAMES_PER_SECOND);
        let (r0, rot0, _) = frustum_fields(&draws(&lp)[0]);
        // Half-way through the visible burst.
        step(&mut lp, (VISIBLE_FRAMES / 2.0) / FRAMES_PER_SECOND);
        let (r1, rot1, _) = frustum_fields(&draws(&lp)[0]);
        assert!(r1 > r0, "innermost ring should grow over time");
        assert!(rot1 > rot0, "rotation should advance over time");
    }

    #[test]
    fn alpha_ramps_up_then_down() {
        let mut lp = LandProtectorEffect::new(Attach::WorldPos([0.0; 3]));
        step(&mut lp, 0.5 / FRAMES_PER_SECOND);
        let a_early = frustum_fields(&draws(&lp)[0]).2;
        // Peak alpha at the end of ramp-up.
        step(&mut lp, (ALPHA_RAMP_UP_FRAMES - 0.5) / FRAMES_PER_SECOND);
        let a_peak = frustum_fields(&draws(&lp)[0]).2;
        // Late in the fade-out.
        step(&mut lp, ((VISIBLE_FRAMES - ALPHA_RAMP_UP_FRAMES) * 0.8) / FRAMES_PER_SECOND);
        let a_late = frustum_fields(&draws(&lp)[0]).2;
        assert!(a_peak > a_early, "ramping up: {a_early} → {a_peak}");
        assert!(a_late < a_peak, "fading down: {a_peak} → {a_late}");
        assert!((a_peak - ALPHA_MAX / 255.0).abs() < 1e-4);
    }

    #[test]
    fn effect_dies_after_visible_burst() {
        let mut lp = LandProtectorEffect::new(Attach::WorldPos([0.0; 3]));
        let dt = (VISIBLE_FRAMES + 1.0) / FRAMES_PER_SECOND;
        assert_eq!(step(&mut lp, dt), EffectStatus::Dead);
    }
}
