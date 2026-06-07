//! `EF_BEGINSPELL*` — cast-circle aura under the caster.
//!
//! The casting aura walks a closed strip around the FULL 360° ring at
//! a fixed radius (4.1) with `rise_angle = 80°` measured from
//! horizontal (i.e. the petals lean almost straight up). Each
//! segment's bottom vertex sits on the ring, the top
//! vertex is offset radially outward by `cos(rise)·height` and up by
//! `sin(rise)·height`. Four sub-emitters stack at the
//! same ring with different rotation starts (180°, 270°, 0°, 90° — 90°
//! apart), different max-height, and staggered `process = -ec * 5`
//! fade-ins. The four visible petals in the reference gif come from
//! that 90°-apart starts pattern.
//!
//! Mapping that onto the [`Frustum`] primitive (closed cone, centered on
//! a vertical axis, supports a single sin-wave peak around its rim via
//! `wave_amplitude` / `wave_frequency` / `wave_phase`):
//!
//! 1. **Central vertical column** — narrow constant-width pillar of
//!    light. Single closed [`Frustum`] with `bottom_size == top_size`
//!    (cylinder). Grows over the fade-in window.
//!
//! 2. **Ground base ring** — flat band around the caster's feet, the
//!    "shadow" the column rises from. [`GroundDisc`] annulus.
//!
//! 3. **Petals** — four [`Frustum`]s, ALL centered on the column. Each
//!    is a full closed ring at `bottom_size = petal_distance`, flared
//!    radially out + up by `(cos rise, sin rise) * max_height` at the
//!    top rim. The visible "petal" stripes come from
//!    `ring_yellow.tga` (and siblings), which contain many narrow
//!    flame stripes — wrapping the texture once around the ring
//!    (`uv_repeat = 1`) paints those stripes as flames. Each Frustum
//!    has a distinct `rotation` 90° apart so its stripe set lands
//!    offset from the others (the per-emitter rotation start rotates
//!    the strip, which carries the texture with it). The
//!    whole rotation array advances over time so the flames orbit the
//!    column center.
//!
//! Total visible duration ≈ 56 frames (933 ms) — matches the original
//! game's on-screen casting-aura lifetime.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// Three flame-ring emitters at rotation starts 0°/90°/180° (the original
/// casting aura uses three petal emitters plus a 4th vertical-column
/// emitter; we render the column as a separate primitive). Each petal has
/// its own `distance` / `rise_angle` / max-height — steeper near vertical
/// for the inner ring, flatter outward for the outer rings.
const NUM_PETALS: usize = 3;

/// Peak alpha per primitive — matches the per-emitter alpha levels of the
/// original casting aura. The column is much dimmer than the petals so
/// the bright vertical shaft of light doesn't dominate the composition.
const PETAL_ALPHA_MAX: f32 = 180.0 / 255.0;
const COLUMN_ALPHA_MAX: f32 = 70.0 / 255.0;
const RING_ALPHA_MAX: f32 = 120.0 / 255.0;

/// Fade-in / fade-out duration for any one primitive emission.
const FADE_FRAMES: f32 = 8.0;

// ---------- Central column ----------

const COLUMN_SIDES: u32 = 12;
// Texture tiles 3× around the column (12 sides × 0.25 per segment), matching
// the original game's per-segment U increment of 0.25 wrapping at 1.
const COLUMN_UV_REPEAT: f32 = 3.0;
/// Time over which the column grows from 0 to full height.
const COLUMN_GROWTH_FRAMES: f32 = 12.0;
/// Column rise angle — 89°, matching the original game's near-vertical
/// shaft: the column is a near-vertical cone (not a pure cylinder), flaring
/// slightly outward at the top.
const COLUMN_RISE_ANGLE_DEG: f32 = 89.0;

// ---------- Ground ring ----------

const RING_UV_REPEAT: f32 = 1.0;
/// Texture used for the flat ground band. `alpha_down.tga` is a radial
/// gradient that fades to transparent at the outer edge — the original
/// game uses it on the ground ring of the basic cast-circle.
const RING_TEXTURE: &str = "alpha_down.tga";

// ---------- Petals ----------

/// Segment count around the ring. The original casting aura uses 20
/// segments; we match it so the texture's flame stripes
/// land on the same per-segment cadence.
const PETAL_SIDES: u32 = 20;
const PETAL_UV_REPEAT: f32 = 1.0;
/// The three petal emitters each span 315° of arc, leaving a 45° gap in
/// the ring. The three petals' rotation starts 0°/90°/180° interleave so
/// the gaps don't stack and the ring still reads as a closed circle of
/// flame.
const PETAL_ARC_DEG: f32 = 315.0;
/// Per-petal rise angle from horizontal — matches the original casting
/// aura's three angles (70°, 57°, 45°): the innermost ring flares almost
/// straight up, the outermost ring is closer to a flat splay.
const PETAL_RISE_ANGLES_DEG: [f32; NUM_PETALS] = [70.0, 57.0, 45.0];
/// How fast the 4-petal phase array rotates around the column
/// (degrees/frame). Combined with the staggered fade-in, this gives
/// the "rotating rune circle" sweep around the caster.
const PETAL_ROT_SPEED_DEG_PER_FRAME: f32 = 4.0;

// ---------- Per-variant ----------

#[derive(Clone, Copy, Debug)]
pub struct CastCircleParams {
    /// Texture sampled on the column and on the petal cones.
    pub texture: &'static str,
    /// RGB tint multiplied into every primitive's color.
    pub color_rgb: [f32; 3],
    /// Column max-height in tilt-direction units — matches the original
    /// game's column emitter. Actual world-space height at peak is
    /// `sin(COLUMN_RISE_ANGLE_DEG) * column_max_height`.
    pub column_max_height: f32,
    /// Column bottom-rim distance — matches the original game's
    /// column emitter. Top rim flares outward by
    /// `cos(COLUMN_RISE_ANGLE_DEG) * column_max_height`.
    pub column_radius: f32,
    /// Ground band outer radius.
    pub ring_radius: f32,
    /// Ground band thickness (`ring_radius` → solid disc).
    pub ring_thickness: f32,
    /// Per-petal bottom-rim radius — 4.5, 5.0, 5.5: rings nest from inner
    /// to outer.
    pub petal_distances: [f32; NUM_PETALS],
    /// Per-petal max flame length, in tilt-direction units (hypotenuse of
    /// the radial + vertical extension at the peak).
    pub petal_heights: [f32; NUM_PETALS],
}

const fn spell_cast(texture: &'static str, r: f32, g: f32, b: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        // Original casting aura's per-emitter sizing literals.
        column_max_height: 250.0,
        column_radius: 4.0,
        ring_radius: 4.0,
        ring_thickness: 2.0,
        petal_distances: [4.5, 5.0, 5.5],
        petal_heights: [25.0, 22.0, 19.0],
    }
}

// Tint kept at white (1, 1, 1) for every variant: the original casting aura
// does not multiply an RGB tint into the
// primitives — the per-spell color comes entirely from the chosen texture
// (`ring_yellow`, `ring_blue`, `ring_red`, `ring_white`, `ring_purple`).
pub const YELLOW: CastCircleParams = spell_cast("ring_yellow.tga", 1.00, 1.00, 1.00);
pub const WATER: CastCircleParams = spell_cast("ring_blue.tga", 1.00, 1.00, 1.00);
pub const FIRE: CastCircleParams = spell_cast("ring_red.tga", 1.00, 1.00, 1.00);
pub const WIND: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const EARTH: CastCircleParams = spell_cast("ring_yellow.tga", 1.00, 1.00, 1.00);
pub const HOLY: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const POISON: CastCircleParams = spell_cast("ring_purple.tga", 1.00, 1.00, 1.00);
pub const RED: CastCircleParams = spell_cast("ring_red.tga", 1.00, 1.00, 1.00);
pub const WHITE: CastCircleParams = spell_cast("ring_white.tga", 1.00, 1.00, 1.00);
pub const N_BLUE: CastCircleParams = spell_cast("ring_blue.tga", 1.00, 1.00, 1.00);

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
    pub fn new(world_pos: [f32; 3], params: CastCircleParams) -> Self {
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

fn fade(local_age: f32, local_life: f32, alpha_max: f32) -> f32 {
    if local_age < 0.0 || local_age > local_life {
        return 0.0;
    }
    let fade_in = (local_age / FADE_FRAMES).clamp(0.0, 1.0);
    let fade_out = if local_age <= local_life - FADE_FRAMES {
        1.0
    } else {
        ((local_life - local_age) / FADE_FRAMES).clamp(0.0, 1.0)
    };
    alpha_max * fade_in * fade_out
}

impl Effect for CastCircleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        let frame = self.frame();

        // -------- Element 1: central vertical column --------
        let col_alpha = fade(frame, TOTAL_FRAMES, COLUMN_ALPHA_MAX);
        if col_alpha > 0.0 {
            let growth = (frame / COLUMN_GROWTH_FRAMES).clamp(0.0, 1.0);
            let col_rise_rad = COLUMN_RISE_ANGLE_DEG.to_radians();
            let (col_sin, col_cos) = col_rise_rad.sin_cos();
            let max_h = self.params.column_max_height * growth;
            let height = col_sin * max_h;
            if height > 0.0 {
                out.push(EffectPrimitiveDraw::Frustum {
                    base: self.world_pos,
                    bottom_size: self.params.column_radius,
                    top_size: self.params.column_radius + col_cos * max_h,
                    height,
                    sides: COLUMN_SIDES,
                    arc_angle_deg: 360.0,
                    rotation: 0.0,
                    uv_repeat: COLUMN_UV_REPEAT,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: self.params.texture,
                    color: [r, g, b, col_alpha],
                    blend: BlendKind::Alpha,
                });
            }
        }

        // -------- Element 2: ground ring --------
        let ring_alpha = fade(frame, TOTAL_FRAMES, RING_ALPHA_MAX);
        if ring_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: self.params.ring_radius,
                thickness: self.params.ring_thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: RING_UV_REPEAT,
                texture: RING_TEXTURE,
                color: [r, g, b, ring_alpha],
                blend: BlendKind::Alpha,
            });
        }

        // -------- Element 3: three flame rings at rotation 0°/90°/180° --------
        let spin_rad = (frame * PETAL_ROT_SPEED_DEG_PER_FRAME).to_radians();
        let alpha = fade(frame, TOTAL_FRAMES, PETAL_ALPHA_MAX);
        if alpha > 0.0 {
            for i in 0..NUM_PETALS {
                let rise_rad = PETAL_RISE_ANGLES_DEG[i].to_radians();
                let (sin_rise, cos_rise) = rise_rad.sin_cos();
                let max_h = self.params.petal_heights[i];
                let distance = self.params.petal_distances[i];
                let offset_rad =
                    (i as f32) * std::f32::consts::FRAC_PI_2;
                out.push(EffectPrimitiveDraw::Frustum {
                    base: self.world_pos,
                    bottom_size: distance,
                    top_size: distance + cos_rise * max_h,
                    height: sin_rise * max_h,
                    sides: PETAL_SIDES,
                    arc_angle_deg: PETAL_ARC_DEG,
                    rotation: spin_rad + offset_rad,
                    uv_repeat: PETAL_UV_REPEAT,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    wave_mode: FrustumWaveMode::Sine,
                    tilt_x_rad: 0.0,
                    rotation_y_rad: 0.0,
                    cull_back: false,
                    texture: self.params.texture,
                    color: [r, g, b, alpha],
                    blend: BlendKind::Alpha,
                });
            }
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
            c.update(&EffectUpdateCtx { delta, camera_target: None, caster_yaw: None });
        }
    }

    fn collect(c: &CastCircleEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_all_three_elements_at_peak() {
        // Column + ground disc + 4 flame rings = 6 primitives at peak.
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        run_to(&mut c, 30.0);
        let prims = collect(&c);
        let columns = prims.iter().filter(|p| is_column(p)).count();
        let petals = prims.iter().filter(|p| is_petal(p)).count();
        let discs = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. })).count();
        assert_eq!(columns, 1);
        assert_eq!(petals, NUM_PETALS);
        assert_eq!(discs, 1);
    }

    fn is_column(p: &EffectPrimitiveDraw) -> bool {
        matches!(p, EffectPrimitiveDraw::Frustum { sides, .. } if *sides == COLUMN_SIDES)
    }

    fn is_petal(p: &EffectPrimitiveDraw) -> bool {
        matches!(p, EffectPrimitiveDraw::Frustum { sides, .. } if *sides == PETAL_SIDES)
    }

    #[test]
    fn flame_ring_centered_on_column_with_rotating_texture() {
        // The flame ring Frustums are centered on the caster and their
        // `rotation` advances over time as the stripes orbit.
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastCircleEffect::new(caster, YELLOW);
        run_to(&mut c, 30.0);
        let snapshot = |c: &CastCircleEffect| -> Option<([f32; 3], f32)> {
            collect(c).into_iter().find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { base, rotation, sides, .. }
                    if sides == PETAL_SIDES => Some((base, rotation)),
                _ => None,
            })
        };
        let (base_early, rot_early) = snapshot(&c).expect("flame ring should be emitted");
        assert!((base_early[0] - caster[0]).abs() < 1e-3, "petal X must equal caster X");
        assert!((base_early[2] - caster[2]).abs() < 1e-3, "petal Z must equal caster Z");
        run_to(&mut c, 40.0);
        let (_, rot_later) = snapshot(&c).unwrap();
        assert!((rot_later - rot_early).abs() > 1e-3,
            "flame ring rotation should advance over time ({} → {})", rot_early, rot_later);
    }

    #[test]
    fn column_grows_over_growth_window() {
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        let height_of_column = |c: &CastCircleEffect| -> f32 {
            collect(c).into_iter().find_map(|p| if is_column(&p) {
                if let EffectPrimitiveDraw::Frustum { height, .. } = p { Some(height) } else { None }
            } else { None }).unwrap_or(0.0)
        };
        run_to(&mut c, 2.0);
        let h_early = height_of_column(&c);
        run_to(&mut c, COLUMN_GROWTH_FRAMES);
        let h_full = height_of_column(&c);
        assert!(h_full > h_early, "column should grow ({} → {})", h_early, h_full);
        let expected = COLUMN_RISE_ANGLE_DEG.to_radians().sin() * YELLOW.column_max_height;
        assert!((h_full - expected).abs() < 1e-3,
            "column should reach full height by frame {}, got {} (expected {})",
            COLUMN_GROWTH_FRAMES, h_full, expected);
    }

    #[test]
    fn every_variant_has_a_real_texture() {
        for params in [
            YELLOW, WATER, FIRE, WIND, EARTH, HOLY, POISON, RED, WHITE, N_BLUE,
        ] {
            assert!(!params.texture.is_empty());
            assert!(TEXTURES.contains(&params.texture));
        }
        assert!(TEXTURES.contains(&RING_TEXTURE));
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastCircleEffect::new([0.0; 3], YELLOW);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1, camera_target: None, caster_yaw: None }), EffectStatus::Running);
        }
    }
}
