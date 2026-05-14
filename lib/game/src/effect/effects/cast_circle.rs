//! `EF_BEGINSPELL*` / `EF_BEGINASURA*` — cast-circle aura under the caster.
//!
//! Reference 54.gif shows three distinct visual elements (the original
//! `SAINTCASTING` → GI-1 source describes a tilted-cone primitive
//! whose flared geometry doesn't match the kRO reference silhouette, so
//! we build the look as a composite of primitives we have):
//!
//! 1. **Central vertical column** — a narrow constant-width pillar of
//!    light rising from the caster's feet. Rendered as a single closed
//!    [`Frustum`] with `bottom_size == top_size` (cylinder, no flare).
//!    Grows from 0 to `COLUMN_HEIGHT` over the fade-in window.
//!
//! 2. **Ground base ring** — a flat dark band of light around the
//!    caster's feet (the "shadow" the column rises from). Rendered as a
//!    [`GroundDisc`] annulus at the caster's feet.
//!
//! 3. **Petals** — four small short tilted [`Frustum`] cones at radial
//!    offsets, leaning outward (modelled after the original game's 4 sub-emitters
//!    with `rise_angle ≈ 70°`). Staggered fade-ins make them appear in
//!    sequence around the ring — that's the "wave around the circle"
//!    feel.
//!
//! Total visible duration ≈ 56 frames (933 ms) — matches the original's clamp
//! `duration >= 56` inside the saint-casting path.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const FRAMES_PER_SECOND: f32 = 60.0;
const TOTAL_FRAMES: f32 = 56.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

const NUM_PETALS: usize = 4;

/// Per-petal staggered fade-in delay in frames — first petal at 0,
/// others arrive in sequence. Same offsets pattern the original game uses on
/// `GI[ec].alpha_b`.
const PETAL_FADE_IN_DELAYS: [f32; NUM_PETALS] = [22.0, 14.0, 7.0, 0.0];

/// Compass headings of each petal around the caster — 90° apart.
const PETAL_ROT_STARTS_DEG: [f32; NUM_PETALS] = [180.0, 270.0, 0.0, 90.0];

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
/// uses it on the 3D circle in the non-secondjob BeginSpell path.
const RING_TEXTURE: &str = "alpha_down.tga";

// ---------- Petals ----------

const PETAL_SIDES: u32 = 12;
const PETAL_UV_REPEAT: f32 = 1.0;
/// Petal cone tilt from horizontal. Low value (35°) makes petals splay
/// outward in a mostly horizontal arc — short curved flames at the base,
/// matching the reference silhouette.
const PETAL_RISE_ANGLE_DEG: f32 = 35.0;
/// How fast the 4-petal ring rotates around the column (degrees/frame).
/// Combined with the staggered fade-in, this gives the "rotating rune
/// circle" sweep around the caster.
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
    /// Radial offset of each petal's base from the column center. The
    /// four petals form a ring around the column; the whole ring rotates
    /// over time via [`PETAL_ROT_SPEED_DEG_PER_FRAME`].
    pub petal_distance: f32,
    /// Per-petal max flame length (4 slightly different values give the
    /// "uneven petals" silhouette).
    pub petal_heights: [f32; NUM_PETALS],
    /// Petal cone base radius. Small — petals are short outward-leaning
    /// flames, not tall spikes.
    pub petal_radius: f32,
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
        petal_radius: 0.5,
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
        petal_radius: 0.6,
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
    petal_radius: 0.7,
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

        // -------- Element 3: rotating ring of 4 outward-leaning petals -----
        // Each petal sits at radial offset `petal_distance` from the
        // column center; the whole ring rotates around the vertical axis
        // at PETAL_ROT_SPEED_DEG_PER_FRAME. Petal cone tilts outward
        // (rise_angle low → mostly horizontal sweep) so it reads as a
        // short curved flame splaying away from the column.
        let rise_rad = PETAL_RISE_ANGLE_DEG.to_radians();
        let (sin_rise, cos_rise) = rise_rad.sin_cos();
        let spin_rad = (frame * PETAL_ROT_SPEED_DEG_PER_FRAME).to_radians();
        for i in 0..NUM_PETALS {
            let local_age = frame - PETAL_FADE_IN_DELAYS[i];
            let local_life = TOTAL_FRAMES - PETAL_FADE_IN_DELAYS[i];
            let alpha = fade(local_age, local_life);
            if alpha <= 0.0 {
                continue;
            }
            let max_h = self.params.petal_heights[i];
            let heading_rad = PETAL_ROT_STARTS_DEG[i].to_radians() + spin_rad;
            let (sin_h, cos_h) = heading_rad.sin_cos();
            let base = [
                self.world_pos[0] + self.params.petal_distance * cos_h,
                self.world_pos[1],
                self.world_pos[2] + self.params.petal_distance * sin_h,
            ];
            out.push(EffectPrimitiveDraw::Frustum {
                base,
                bottom_size: self.params.petal_radius,
                top_size: self.params.petal_radius + cos_rise * max_h,
                height: sin_rise * max_h,
                sides: PETAL_SIDES,
                rotation: heading_rad,
                uv_repeat: PETAL_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
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
        let frustum_count = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. })).count();
        let disc_count = prims.iter().filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { .. })).count();
        // 1 column + 4 petals
        assert_eq!(frustum_count, 1 + NUM_PETALS);
        assert_eq!(disc_count, 1);
    }

    #[test]
    fn column_has_constant_radius_no_flare() {
        // Distinguishing the column from petals: it's the only Frustum
        // with `bottom_size == top_size`.
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 30.0);
        let mut found_column = false;
        for prim in collect(&c) {
            if let EffectPrimitiveDraw::Frustum { bottom_size, top_size, .. } = prim {
                if (bottom_size - top_size).abs() < 1e-4 {
                    found_column = true;
                    assert!(bottom_size > 0.0);
                }
            }
        }
        assert!(found_column, "central column with bottom_size == top_size should exist");
    }

    #[test]
    fn petals_form_rotating_ring_around_column() {
        // Petals sit on a circle of radius petal_distance around the
        // column center, and the whole ring's rotation advances over
        // time.
        let caster = [10.0, 5.0, 20.0];
        let mut c = CastCircleEffect::new(Attach::WorldPos(caster), YELLOW);
        run_to(&mut c, 30.0);
        let snapshot_petals = |c: &CastCircleEffect| -> Vec<([f32; 3], f32)> {
            collect(c).into_iter().filter_map(|p| match p {
                EffectPrimitiveDraw::Frustum { base, top_size, bottom_size, rotation, .. }
                    if top_size > bottom_size + 1e-4 => Some((base, rotation)),
                _ => None,
            }).collect()
        };
        let early = snapshot_petals(&c);
        assert_eq!(early.len(), NUM_PETALS);
        for (base, _) in &early {
            let dx = base[0] - caster[0];
            let dz = base[2] - caster[2];
            let r = (dx * dx + dz * dz).sqrt();
            assert!((r - YELLOW.petal_distance).abs() < 1e-3,
                "petal base should sit at radius {} from caster, got {} (base {:?})",
                YELLOW.petal_distance, r, base);
        }
        run_to(&mut c, 40.0);
        let later = snapshot_petals(&c);
        for (i, (_, rot_now)) in later.iter().enumerate() {
            assert!((rot_now - early[i].1).abs() > 1e-3,
                "petal {} rotation should advance over time ({} → {})",
                i, early[i].1, rot_now);
        }
    }

    #[test]
    fn petals_appear_in_sequence_not_all_at_once() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        run_to(&mut c, 1.0);
        let count_petals = |c: &CastCircleEffect| -> usize {
            collect(c).into_iter().filter(|p| matches!(p,
                EffectPrimitiveDraw::Frustum { bottom_size, top_size, .. }
                    if (top_size - bottom_size).abs() > 1e-4
            )).count()
        };
        let early = count_petals(&c);
        run_to(&mut c, 30.0);
        let peak = count_petals(&c);
        assert!(early < peak, "petals stagger in (early {} → peak {})", early, peak);
        assert_eq!(peak, NUM_PETALS);
    }

    #[test]
    fn column_grows_over_growth_window() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        let height_of_column = |c: &CastCircleEffect| -> f32 {
            collect(c).into_iter().find_map(|p| match p {
                EffectPrimitiveDraw::Frustum { bottom_size, top_size, height, .. }
                    if (bottom_size - top_size).abs() < 1e-4 => Some(height),
                _ => None,
            }).unwrap_or(0.0)
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
