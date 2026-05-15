//! `EF_BEGINSPELL*` / `EF_BEGINASURA*` — cast-circle aura under the caster.
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
//!    has `wave_frequency = 1` (one peak around its rim) and a distinct
//!    `wave_phase` 90° apart so the four peaks land at four compass
//!    headings. The wave goes from collapsed (top vertex on the bottom
//!    ring → invisible) to full extension (top vertex displaced
//!    radially outward + up by `max_height` along the rise tilt). The
//!    whole phase array advances over time so the four peaks orbit the
//!    column center.
//!
//! Total visible duration ≈ 56 frames (933 ms) — matches the original
//! game's on-screen casting-aura lifetime.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const NUM_PETALS: usize = 4;

/// Per-petal staggered start in frames — original game uses
/// `process = -ec * 5` for short casts, so ec=0 is visible immediately
/// and each subsequent petal is delayed by 5 frames.
const PETAL_FADE_IN_DELAYS: [f32; NUM_PETALS] = [0.0, 5.0, 10.0, 15.0];

/// Compass headings where each petal's wave-peak sits at frame 0 —
/// 90° apart, matching the original game's per-slot start rotation
/// (`{180, 270, 0, 90}` produces visible peaks at
/// `rot_start_deg + 180 = {0, 90, 180, 270}` per the 3D casting render's middle
/// segment).
const PETAL_PEAK_HEADINGS_DEG: [f32; NUM_PETALS] = [0.0, 90.0, 180.0, 270.0];

const ALPHA_MAX: f32 = 200.0 / 255.0;

/// Fade-in / fade-out duration for any one primitive emission.
const FADE_FRAMES: f32 = 8.0;

// ---------- Central column ----------

const COLUMN_SIDES: u32 = 12;
const COLUMN_UV_REPEAT: f32 = 1.0;
/// Time over which the column grows from 0 to full height.
const COLUMN_GROWTH_FRAMES: f32 = 12.0;

// ---------- Ground ring ----------

const RING_UV_REPEAT: f32 = 1.0;
/// Texture used for the flat ground band. `alpha_down.tga` is a radial
/// gradient that fades to transparent at the outer edge — the original
/// game uses it on the ground ring of the basic cast-circle.
const RING_TEXTURE: &str = "alpha_down.tga";

// ---------- Petals ----------

const PETAL_SIDES: u32 = 24;
const PETAL_UV_REPEAT: f32 = 1.0;
/// Rise angle from horizontal — original game uses 80°, near-vertical
/// with a small radial outward component. Maps onto `Frustum`'s
/// `(delta_r, height)` tilt direction.
const PETAL_RISE_ANGLE_DEG: f32 = 80.0;
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
    /// Final column height at peak.
    pub column_height: f32,
    /// Column radius (constant — no flare).
    pub column_radius: f32,
    /// Ground band outer radius.
    pub ring_radius: f32,
    /// Ground band thickness (`ring_radius` → solid disc).
    pub ring_thickness: f32,
    /// Radius of the ring on which each petal's bottom rim sits — the
    /// `bottom_size` of every petal Frustum. The four petals share this
    /// ring; their wave-peaks lie on it 90° apart.
    pub petal_distance: f32,
    /// Per-petal max flame length, in tilt-direction units (i.e. the
    /// hypotenuse of the radial + vertical extension at the peak). Four
    /// slightly different values give the "uneven petals" silhouette.
    pub petal_heights: [f32; NUM_PETALS],
}

const fn beg(texture: &'static str, r: f32, g: f32, b: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        column_height: 40.0,
        column_radius: 1.2,
        ring_radius: 4.0,
        ring_thickness: 2.0,
        petal_distance: 3.0,
        petal_heights: [4.5, 4.0, 3.5, 3.0],
    }
}

const fn asu(texture: &'static str, r: f32, g: f32, b: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        column_height: 50.0,
        column_radius: 1.5,
        ring_radius: 5.0,
        ring_thickness: 2.5,
        petal_distance: 3.8,
        petal_heights: [5.5, 5.0, 4.5, 4.0],
    }
}

pub const YELLOW: CastCircleParams = beg("ring_yellow.tga", 1.00, 0.90, 0.30);
pub const WATER: CastCircleParams = beg("ring_blue.tga", 0.30, 0.60, 1.00);
pub const FIRE: CastCircleParams = beg("ring_red.tga", 1.00, 0.40, 0.15);
pub const WIND: CastCircleParams = beg("ring_white.tga", 0.55, 1.00, 0.60);
pub const EARTH: CastCircleParams = beg("ring_yellow.tga", 0.80, 0.55, 0.25);
pub const HOLY: CastCircleParams = beg("ring_white.tga", 1.00, 0.95, 0.80);
pub const POISON: CastCircleParams = beg("ring_purple.tga", 0.70, 0.30, 0.85);
pub const RED: CastCircleParams = beg("ring_red.tga", 1.00, 0.25, 0.25);
pub const WHITE: CastCircleParams = beg("ring_white.tga", 0.95, 0.95, 1.00);
pub const N_BLUE: CastCircleParams = beg("ring_blue.tga", 0.55, 0.75, 1.00);

pub const ASURA: CastCircleParams = asu("ring_yellow.tga", 1.00, 0.90, 0.30);
pub const ASURA_EARTH: CastCircleParams = asu("ring_yellow.tga", 0.80, 0.55, 0.25);
pub const ASURA_WIND: CastCircleParams = asu("ring_white.tga", 0.55, 1.00, 0.60);
pub const ASURA_WATER: CastCircleParams = asu("ring_blue.tga", 0.30, 0.60, 1.00);
pub const ASURA_FIRE: CastCircleParams = asu("ring_red.tga", 1.00, 0.40, 0.15);
pub const ASURA_UNDEAD: CastCircleParams = asu("ring_purple.tga", 0.55, 0.45, 0.45);
pub const ASURA_SHADOW: CastCircleParams = asu("ring_purple.tga", 0.45, 0.20, 0.60);
pub const ASURA_HOLY: CastCircleParams = asu("ring_white.tga", 1.00, 0.95, 0.80);
pub const ASURA_CHAMPION: CastCircleParams = CastCircleParams {
    texture: "ring_yellow.tga",
    color_rgb: [1.00, 0.85, 0.30],
    column_height: 60.0,
    column_radius: 1.8,
    ring_radius: 6.0,
    ring_thickness: 3.0,
    petal_distance: 4.5,
    petal_heights: [6.5, 6.0, 5.5, 5.0],
};

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

fn fade(local_age: f32, local_life: f32) -> f32 {
    if local_age < 0.0 || local_age > local_life {
        return 0.0;
    }
    let fade_in = (local_age / FADE_FRAMES).clamp(0.0, 1.0);
    let fade_out = if local_age <= local_life - FADE_FRAMES {
        1.0
    } else {
        ((local_life - local_age) / FADE_FRAMES).clamp(0.0, 1.0)
    };
    ALPHA_MAX * fade_in * fade_out
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
        let col_alpha = fade(frame, TOTAL_FRAMES);
        if col_alpha > 0.0 {
            // Column height grows over the first COLUMN_GROWTH_FRAMES then
            // holds for the rest of the visible life.
            let growth = (frame / COLUMN_GROWTH_FRAMES).clamp(0.0, 1.0);
            let height = self.params.column_height * growth;
            if height > 0.0 {
                out.push(EffectPrimitiveDraw::Frustum {
                    base: self.world_pos,
                    bottom_size: self.params.column_radius,
                    top_size: self.params.column_radius,
                    height,
                    sides: COLUMN_SIDES,
                    rotation: 0.0,
                    uv_repeat: COLUMN_UV_REPEAT,
                    uv_scroll: [0.0, 0.0],
                    wave_amplitude: 0.0,
                    wave_frequency: 1.0,
                    wave_phase: 0.0,
                    texture: self.params.texture,
                    color: [r, g, b, col_alpha],
                    blend: BlendKind::Additive,
                });
            }
        }

        // -------- Element 2: ground ring --------
        let ring_alpha = fade(frame, TOTAL_FRAMES);
        if ring_alpha > 0.0 {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: self.params.ring_radius,
                thickness: self.params.ring_thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: RING_UV_REPEAT,
                texture: RING_TEXTURE,
                color: [r, g, b, ring_alpha * 0.5],
                blend: BlendKind::Additive,
            });
        }

        // -------- Element 3: four wave-peaks orbiting the column ----------
        // Each petal is a `Frustum` centered on the column. Its `rest`
        // pose sits halfway between collapsed (top vertex on the bottom
        // ring) and full extension (top vertex displaced radially out +
        // up by `max_height` along the rise tilt). `wave_amplitude =
        // tilt_len/2` then sweeps the rim between those two extremes
        // along the (cos_rise, sin_rise) direction:
        //   * at wave = +amp   → top vertex at full extension
        //   * at wave = -amp   → top vertex collapsed onto the bottom ring
        //
        // With `wave_frequency = 1`, sin gives one peak + one collapsed
        // antipode per revolution per Frustum. Four Frustums with
        // `wave_phase` 90° apart produce four peaks, naturally centered
        // on the column. The whole phase array advances over time so
        // peaks orbit around the column center.
        let rise_rad = PETAL_RISE_ANGLE_DEG.to_radians();
        let (sin_rise, cos_rise) = rise_rad.sin_cos();
        let spin_deg = frame * PETAL_ROT_SPEED_DEG_PER_FRAME;
        for i in 0..NUM_PETALS {
            let local_age = frame - PETAL_FADE_IN_DELAYS[i];
            let local_life = TOTAL_FRAMES - PETAL_FADE_IN_DELAYS[i];
            let alpha = fade(local_age, local_life);
            if alpha <= 0.0 {
                continue;
            }
            let max_h = self.params.petal_heights[i];
            let full_delta_r = cos_rise * max_h;
            let full_height = sin_rise * max_h;
            let rest_top_size = self.params.petal_distance + full_delta_r * 0.5;
            let rest_height = full_height * 0.5;
            // tilt_len at peak = sqrt(delta_r² + height²) = max_h, so
            // the wave amplitude that reaches both extremes from rest
            // is max_h/2.
            let wave_amp = max_h * 0.5;

            // sin peaks at sin_arg = π/2. We want the peak to land at
            // world angle θ = (PETAL_PEAK_HEADINGS_DEG[i] + spin_deg).
            // Frustum's wave uses local_angle (= world_angle, since we
            // pass `rotation = 0`); so wave_phase = π/2 - θ.
            let peak_angle_rad = (PETAL_PEAK_HEADINGS_DEG[i] + spin_deg).to_radians();
            let wave_phase = std::f32::consts::FRAC_PI_2 - peak_angle_rad;

            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: self.params.petal_distance,
                top_size: rest_top_size,
                height: rest_height,
                sides: PETAL_SIDES,
                rotation: 0.0,
                uv_repeat: PETAL_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: wave_amp,
                wave_frequency: 1.0,
                wave_phase,
                texture: self.params.texture,
                color: [r, g, b, alpha],
                blend: BlendKind::Additive,
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

    fn run_to(c: &mut CastCircleEffect, target_frame: f32) {
        let current = c.frame();
        let delta = (target_frame - current) / FRAMES_PER_SECOND;
        if delta > 0.0 {
            c.update(&EffectUpdateCtx { delta });
        }
    }

    fn collect(c: &CastCircleEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn emits_all_three_elements_at_peak() {
        // Column + ring + 4 petals = 6 primitives, all visible at peak.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
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
        matches!(p, EffectPrimitiveDraw::Frustum {
            bottom_size, top_size, wave_amplitude, ..
        } if (bottom_size - top_size).abs() < 1e-4 && *wave_amplitude == 0.0)
    }

    fn is_petal(p: &EffectPrimitiveDraw) -> bool {
        matches!(p, EffectPrimitiveDraw::Frustum { wave_amplitude, .. } if *wave_amplitude > 0.0)
    }

    #[test]
    fn column_has_constant_radius_no_flare() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 30.0);
        let mut found = false;
        for prim in collect(&c) {
            if let EffectPrimitiveDraw::Frustum { bottom_size, top_size, wave_amplitude, .. } = prim
                && (bottom_size - top_size).abs() < 1e-4
                && wave_amplitude == 0.0
            {
                found = true;
                assert!(bottom_size > 0.0);
            }
        }
        assert!(found, "central column (no flare, no wave) should exist");
    }

    #[test]
    fn petals_centered_on_column_with_rotating_phases() {
        // Each petal Frustum is centered on the caster (its base == the
        // caster's world position) — "center of rotation is the column".
        // The wave_phase array advances over time as the four peaks orbit.
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastCircleEffect::new(Attach::WorldPos(caster), YELLOW);
        run_to(&mut c, 30.0);
        let snapshot_petals = |c: &CastCircleEffect| -> Vec<([f32; 3], f32)> {
            collect(c).into_iter().filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { base, wave_amplitude, wave_phase, .. }
                    if wave_amplitude > 0.0 => Some((base, wave_phase)),
                _ => None,
            }).collect()
        };
        let early = snapshot_petals(&c);
        assert_eq!(early.len(), NUM_PETALS);
        for (base, _) in &early {
            assert!((base[0] - caster[0]).abs() < 1e-3, "petal X must equal caster X");
            assert!((base[2] - caster[2]).abs() < 1e-3, "petal Z must equal caster Z");
        }
        run_to(&mut c, 40.0);
        let later = snapshot_petals(&c);
        for (i, (_, phase_now)) in later.iter().enumerate() {
            assert!((phase_now - early[i].1).abs() > 1e-3,
                "petal {} wave_phase should advance over time ({} → {})",
                i, early[i].1, phase_now);
        }
    }

    #[test]
    fn petal_peaks_are_90deg_apart() {
        // Once all four petals have faded in, their wave_phase values
        // must be pairwise 90° apart (mod 360°) so the four peaks land
        // at four compass headings around the column.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 25.0);
        let phases: Vec<f32> = collect(&c).into_iter().filter_map(|p| match p {
            EffectPrimitiveDraw::Frustum { wave_amplitude, wave_phase, .. }
                if wave_amplitude > 0.0 => Some(wave_phase),
            _ => None,
        }).collect();
        assert_eq!(phases.len(), NUM_PETALS);
        for w in phases.windows(2) {
            let two_pi = std::f32::consts::TAU;
            let raw = ((w[0] - w[1]).abs() % two_pi + two_pi) % two_pi;
            let d = raw.min(two_pi - raw);
            assert!((d - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
                "adjacent petals must be 90° apart, got {}°", d.to_degrees());
        }
    }

    #[test]
    fn petals_appear_in_sequence_not_all_at_once() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 1.0);
        let count = |c: &CastCircleEffect| collect(c).iter().filter(|p| is_petal(p)).count();
        let early = count(&c);
        run_to(&mut c, 30.0);
        let peak = count(&c);
        assert!(early < peak, "petals stagger in (early {} → peak {})", early, peak);
        assert_eq!(peak, NUM_PETALS);
    }

    #[test]
    fn column_grows_over_growth_window() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
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
        assert!((h_full - YELLOW.column_height).abs() < 1e-3,
            "column should reach full height by frame {}, got {}", COLUMN_GROWTH_FRAMES, h_full);
    }

    #[test]
    fn every_variant_has_a_real_texture() {
        for params in [
            YELLOW, WATER, FIRE, WIND, EARTH, HOLY, POISON, RED, WHITE, N_BLUE,
            ASURA, ASURA_EARTH, ASURA_WIND, ASURA_WATER, ASURA_FIRE,
            ASURA_UNDEAD, ASURA_SHADOW, ASURA_HOLY, ASURA_CHAMPION,
        ] {
            assert!(!params.texture.is_empty());
            assert!(TEXTURES.contains(&params.texture));
        }
        assert!(TEXTURES.contains(&RING_TEXTURE));
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1 }), EffectStatus::Running);
        }
    }
}
