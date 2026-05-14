//! `EF_BEGINSPELL*` / `EF_BEGINASURA*` — cast-circle aura under the caster.
//!
//! Reconstruction of the original game's non-secondjob `BeginSpell` path:
//!
//! Frame 0:
//!   * One ground circle (latitude=90 → flat-on-ground) using alpha_down.tga,
//!     solid-disc fill pattern (not annulus), radius 15,
//!     duration 25, max alpha 128/255. Fades in over 10 frames, holds, fades
//!     out the last 10.
//!
//! Every `CYL_SPAWN_INTERVAL_FRAMES` frames (≈ every 8-10 in the original):
//!   * One cylinder (true cylinder, equal top/bottom radii). Each
//!     instance lives `CYL_DURATION_FRAMES = 25` frames.
//!   * Per-instance growth (closed-form integral of the original's
//!     Euler-stepped speed/accel kinematics — accel = -speed/duration/2):
//!         radius(age) = OUTER_SPEED * age * (1 - age / (4 * CYL_DURATION))
//!         height(age) = HEIGHT_SPEED * age * (1 - age / (4 * CYL_DURATION))
//!     i.e. fast at first, decelerating to a near-stop near end-of-life.
//!     At `age == CYL_DURATION`, both reach 0.75 * speed * duration.
//!   * `HEIGHT_SPEED` is ~7× `OUTER_SPEED` → the cylinder is tall and narrow
//!     (h/r ≈ 7), which is what reads as "vertical expansion" instead of
//!     horizontal.
//!   * `LONG_SPEED` advances `uv_scroll[0]` (the texture wraps around the
//!     circumference) — the ring texture's baked stripes sweep around the
//!     cylinder. With 3-5 staggered cylinder instances on-screen at different
//!     growth phases, this reads as the "left-to-right wave" sweeping over
//!     the column.
//!
//! Parent emitter duration `40` (no new spawns after frame 40) → the
//! visible tail extends to frame `40 + CYL_DURATION = 65`.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
/// Parent emitter lifetime — `SET_DURATION(40, EF_BEGINSPELL)` in the original.
const PARENT_DURATION_FRAMES: f32 = 40.0;
/// Each spawned cylinder lives this many frames.
const CYL_DURATION_FRAMES: f32 = 25.0;
/// Ground disc lifetime — original duration = 25 on the 3D circle.
const DISC_DURATION_FRAMES: f32 = 25.0;
/// Spawn a new cylinder every N frames while the parent is still active.
/// The original alternates `state_cnt % 10 == 0` and `state_cnt % 8 == 0`;
/// we pick 5 to roughly match the *combined* spawn rate (≈ 1 every 4-5
/// frames in the original counting both spawn paths). Tweak if the visual
/// gets too dense or too sparse.
const CYL_SPAWN_INTERVAL_FRAMES: f32 = 5.0;
/// Total visible duration — parent emitter window plus the lifetime of the
/// last cylinder it spawned.
const TOTAL_FRAMES: f32 = PARENT_DURATION_FRAMES + CYL_DURATION_FRAMES;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Original game max alpha = 128/255, applied per emitted primitive.
const ALPHA_MAX: f32 = 128.0 / 255.0;
/// Per-primitive fade-in / fade-out window in frames.
const PRIM_FADE_FRAMES: f32 = 10.0;

/// Polygon sides around the cylinder's circumference. 16 ≈ smooth.
const SIDES: u32 = 16;
/// Texture wraps once around the cylinder's circumference. The baked ring
/// stripes are what carry the rune pattern.
const UV_REPEAT: f32 = 1.0;
/// Radius growth rate. Original outer speed `0.4` scaled 2× to match the
/// gif's visible footprint (player is ~5-8 game units, original's literal
/// numbers are roughly 2× compressed against the gif).
const OUTER_SPEED: f32 = 0.4 * 2.0;
/// Height growth rate. Original `height_speed = 3.0`. At end-of-life
/// (age = 25), height = 0.75 * 6.0 * 25 = 112.5 game units. That tall &
/// narrow column is exactly what the gif shows.
const HEIGHT_SPEED: f32 = 3.0 * 2.0;
/// U-scroll speed of the texture around the cylinder. Original
/// `long_speed = 0.5` per frame; the texture wraps once around so a value
/// of 1.0 = one full circumferential sweep per frame. We pass it in
/// texture-coord units; the renderer multiplies onto u.
const LONG_SPEED: f32 = 0.5 / CYL_DURATION_FRAMES;
/// Ground disc radius — original `radius = 15`, kept 1:1 in game units
/// (a ~3×3 tile footprint at the caster's feet).
const DISC_RADIUS: f32 = 15.0;
/// Texture used by the flat-on-ground halo. `alpha_down.tga` is a radial
/// gradient that fades to transparent at the edge — the original uses it
/// universally regardless of element variant.
const DISC_TEXTURE: &str = "alpha_down.tga";

#[derive(Clone, Copy, Debug)]
pub struct CastCircleParams {
    /// Texture sampled on the cylinder side walls.
    pub texture: &'static str,
    /// RGB tint multiplied into both the cylinder and disc textures; alpha
    /// is driven by the per-primitive fade curve.
    pub color_rgb: [f32; 3],
    /// Multiplier on `OUTER_SPEED` — bigger variants have wider cylinders.
    /// Beginspell: 1.0 (radius peaks at 15). Beginasura: 1.33 (≈ 20).
    pub size_mult: f32,
}

const fn p(texture: &'static str, r: f32, g: f32, b: f32, size_mult: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        size_mult,
    }
}

// Beginspell — base size (~15 unit radius cylinder, ~112 unit tall at peak).
pub const YELLOW: CastCircleParams = p("ring_yellow.tga", 1.00, 0.90, 0.30, 1.0);
pub const WATER: CastCircleParams = p("ring_blue.tga", 0.30, 0.60, 1.00, 1.0);
pub const FIRE: CastCircleParams = p("ring_red.tga", 1.00, 0.40, 0.15, 1.0);
pub const WIND: CastCircleParams = p("ring_white.tga", 0.55, 1.00, 0.60, 1.0);
pub const EARTH: CastCircleParams = p("ring_yellow.tga", 0.80, 0.55, 0.25, 1.0);
pub const HOLY: CastCircleParams = p("ring_white.tga", 1.00, 0.95, 0.80, 1.0);
pub const POISON: CastCircleParams = p("ring_purple.tga", 0.70, 0.30, 0.85, 1.0);
pub const RED: CastCircleParams = p("ring_red.tga", 1.00, 0.25, 0.25, 1.0);
pub const WHITE: CastCircleParams = p("ring_white.tga", 0.95, 0.95, 1.00, 1.0);
pub const N_BLUE: CastCircleParams = p("ring_blue.tga", 0.55, 0.75, 1.00, 1.0);

// Beginasura — wider/taller for Asura Strike chants.
pub const ASURA: CastCircleParams = p("ring_yellow.tga", 1.00, 0.90, 0.30, 1.33);
pub const ASURA_EARTH: CastCircleParams = p("ring_yellow.tga", 0.80, 0.55, 0.25, 1.33);
pub const ASURA_WIND: CastCircleParams = p("ring_white.tga", 0.55, 1.00, 0.60, 1.33);
pub const ASURA_WATER: CastCircleParams = p("ring_blue.tga", 0.30, 0.60, 1.00, 1.33);
pub const ASURA_FIRE: CastCircleParams = p("ring_red.tga", 1.00, 0.40, 0.15, 1.33);
pub const ASURA_UNDEAD: CastCircleParams = p("ring_purple.tga", 0.55, 0.45, 0.45, 1.33);
pub const ASURA_SHADOW: CastCircleParams = p("ring_purple.tga", 0.45, 0.20, 0.60, 1.33);
pub const ASURA_HOLY: CastCircleParams = p("ring_white.tga", 1.00, 0.95, 0.80, 1.33);
pub const ASURA_CHAMPION: CastCircleParams = p("ring_yellow.tga", 1.00, 0.85, 0.30, 1.67);

pub const TEXTURES: &[&str] = &[
    "ring_yellow.tga",
    "ring_blue.tga",
    "ring_red.tga",
    "ring_white.tga",
    "ring_purple.tga",
    "alpha_down.tga",
];

pub struct CastCircleEffect {
    params: CastCircleParams,
    world_pos: [f32; 3],
    age: f32,
}

impl CastCircleEffect {
    pub fn new(attach: Attach, params: CastCircleParams) -> Self {
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

    fn frame(&self) -> f32 {
        self.age * FRAMES_PER_SECOND
    }
}

/// Linear fade-in over `PRIM_FADE_FRAMES`, hold, fade-out the last
/// `PRIM_FADE_FRAMES`. Shared by both the ground disc and every cylinder
/// emission, computed against each primitive's own age.
fn prim_alpha(age: f32, duration: f32) -> f32 {
    if age < 0.0 || age > duration {
        return 0.0;
    }
    let fade_in = (age / PRIM_FADE_FRAMES).clamp(0.0, 1.0);
    let fade_out_start = duration - PRIM_FADE_FRAMES;
    let fade_out = if age <= fade_out_start {
        1.0
    } else {
        ((duration - age) / PRIM_FADE_FRAMES).clamp(0.0, 1.0)
    };
    ALPHA_MAX * fade_in * fade_out
}

/// Closed-form integral of Euler-stepped speed/accel where
/// `accel = -speed/duration/2`. Matches the original game's per-frame
/// outer size += outer speed; outer speed += outer accel.
fn kinematic(speed: f32, duration: f32, age: f32) -> f32 {
    speed * age * (1.0 - age / (4.0 * duration))
}

impl Effect for CastCircleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let frame = self.frame();

        // Element 1: ground disc (alpha_down.tga), only during its 25-frame life.
        let disc_alpha = prim_alpha(frame, DISC_DURATION_FRAMES);
        if disc_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: DISC_RADIUS * self.params.size_mult,
                // Fill-circle in the original is a solid disc, not an
                // annulus — `thickness == radius` makes GroundDisc render
                // all the way to the center.
                thickness: DISC_RADIUS * self.params.size_mult,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: 1.0,
                texture: DISC_TEXTURE,
                color: [r, g, b, disc_alpha],
                blend: BlendKind::Additive,
            });
        }

        // Element 2: staggered cylinder spawns. The parent stops spawning
        // after PARENT_DURATION_FRAMES, but already-spawned cylinders keep
        // running until their own CYL_DURATION_FRAMES is up.
        let mut spawn_frame = 0.0f32;
        while spawn_frame <= PARENT_DURATION_FRAMES && spawn_frame <= frame {
            let age = frame - spawn_frame;
            if age >= 0.0 && age < CYL_DURATION_FRAMES {
                let cyl_alpha = prim_alpha(age, CYL_DURATION_FRAMES);
                if cyl_alpha > 0.0 {
                    let outer_speed = OUTER_SPEED * self.params.size_mult;
                    let radius = kinematic(outer_speed, CYL_DURATION_FRAMES, age);
                    let height = kinematic(HEIGHT_SPEED, CYL_DURATION_FRAMES, age);
                    let u_scroll = LONG_SPEED * age;
                    out.push(EffectPrimitiveDraw::Frustum {
                        base: self.world_pos,
                        bottom_size: radius,
                        top_size: radius,
                        height,
                        sides: SIDES,
                        rotation: 0.0,
                        uv_repeat: UV_REPEAT,
                        uv_scroll: [u_scroll, 0.0],
                        wave_amplitude: 0.0,
                        wave_frequency: 1.0,
                        wave_phase: 0.0,
                        texture: self.params.texture,
                        color: [r, g, b, cyl_alpha],
                        blend: BlendKind::Additive,
                    });
                }
            }
            spawn_frame += CYL_SPAWN_INTERVAL_FRAMES;
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

    fn run_to(c: &mut CastCircleEffect, target_frame: f32) {
        let current = c.frame();
        let delta = (target_frame - current) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta });
        }
    }

    #[test]
    fn frame_zero_emits_disc_and_first_cylinder() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([1.0, 0.0, 2.0]), YELLOW);
        // Step a tiny bit past 0 so the first cylinder has age > 0 and
        // produces a non-zero radius/height.
        c.update(&EffectUpdateCtx { delta: 2.0 / FRAMES_PER_SECOND });
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        let n_discs = list.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. })).count();
        let n_cyls = list.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. })).count();
        assert_eq!(n_discs, 1, "exactly one ground disc");
        assert_eq!(n_cyls, 1, "exactly one cylinder so early in the effect");
    }

    #[test]
    fn cylinders_stagger_so_multiple_overlap_at_peak() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        // Mid-effect: the parent has been spawning for ~25 frames, each
        // cylinder lives 25 frames, so several should be alive at once.
        run_to(&mut c, 22.0);
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        let n_cyls = list.primitives.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. })).count();
        assert!(
            n_cyls >= 3,
            "should see >= 3 overlapping cylinders at peak, got {n_cyls}"
        );
    }

    #[test]
    fn cylinder_grows_taller_faster_than_wider() {
        // Verifies "vertical expansion, not horizontal" — height growth
        // rate must dominate radius growth rate by a large factor.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 1.0);
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        let first = list.primitives.iter().find_map(|p| match p {
            EffectPrimitiveDraw::Frustum { bottom_size, height, .. } => Some((*bottom_size, *height)),
            _ => None,
        }).expect("first cylinder");
        // h/r must be > 5 — these cylinders are tall and narrow, not pancakes.
        assert!(
            first.1 / first.0 > 5.0,
            "cylinder should be tall & narrow (h/r > 5), got h={} r={}",
            first.1, first.0
        );
    }

    #[test]
    fn texture_uv_scrolls_around_circumference_over_time() {
        // The "left-to-right wave" feel comes from u-scrolling the texture
        // around the cylinder. uv_scroll[0] of the oldest live cylinder
        // must advance over time.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        let oldest_u_scroll = |c: &CastCircleEffect| -> f32 {
            let mut list = EffectDrawList::new();
            c.collect_draws(&mut list, &render_ctx());
            list.primitives.iter().find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { uv_scroll, .. } => Some(uv_scroll[0]),
                _ => None,
            }).expect("at least one cylinder")
        };
        run_to(&mut c, 5.0);
        let u_a = oldest_u_scroll(&c);
        run_to(&mut c, 15.0);
        let u_b = oldest_u_scroll(&c);
        assert!(u_b > u_a, "first-spawned cylinder's u-scroll should advance");
    }

    #[test]
    fn every_variant_has_a_real_texture() {
        for params in [
            YELLOW, WATER, FIRE, WIND, EARTH, HOLY, POISON, RED, WHITE, N_BLUE,
            ASURA, ASURA_EARTH, ASURA_WIND, ASURA_WATER, ASURA_FIRE,
            ASURA_UNDEAD, ASURA_SHADOW, ASURA_HOLY, ASURA_CHAMPION,
        ] {
            assert!(!params.texture.is_empty(), "{:?} has no texture", params);
            assert!(TEXTURES.contains(&params.texture), "{:?} texture not preloaded", params);
        }
        assert!(TEXTURES.contains(&DISC_TEXTURE), "ground disc texture must be preloaded");
    }

    #[test]
    fn asura_variant_is_wider_than_beginspell() {
        assert!(ASURA.size_mult > YELLOW.size_mult);
        assert!(ASURA_CHAMPION.size_mult > ASURA.size_mult);
    }

    #[test]
    fn never_self_terminates() {
        // Wall-clock duration is enforced by the effect spec table; this
        // module always reports Running and lets the holder shut it down.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1 }), EffectStatus::Running);
        }
    }
}
