//! `EF_BEGINSPELL*` / `EF_BEGINASURA*` — cast-circle aura under the caster's
//! feet at the start of a skill cast.
//!
//! The original game emits a triplet of stacked rotating textured quads to fake a
//! circle: each layer spins at a different rate, building an interference
//! pattern that reads as a rune circle. We model that as three `Billboard`
//! emissions per frame, parameterised by colour + base radius + base texture.
//!
//! The 20-odd cast-circle variants are essentially recolours of the same
//! geometry, with a couple of size and texture swaps for the
//! `EF_BEGINASURA*` family. We collapse them onto one struct with one
//! `pub const` parameter set per EF_*.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

const RING_COUNT: usize = 3;
const RING_RADII_SCALE: [f32; RING_COUNT] = [0.9, 1.0, 1.1];
const RING_Y_OFFSETS: [f32; RING_COUNT] = [-0.5, 0.0, 0.5];
const RING_BASE_ROT_DEG: [f32; RING_COUNT] = [0.0, 90.0, 180.0];
const RING_SPIN_DEG_PER_SEC: [f32; RING_COUNT] = [60.0, -45.0, 35.0];
const RING_ALPHA: f32 = 0.55;

#[derive(Clone, Copy, Debug)]
pub struct CastCircleParams {
    pub texture: &'static str,
    /// RGB tint applied to every ring layer. Alpha is fixed at [`RING_ALPHA`]
    /// to match the original game's blend stack.
    pub color_rgb: [f32; 3],
    /// World-space radius of the middle ring; outer/inner scale via
    /// [`RING_RADII_SCALE`].
    pub base_radius: f32,
}

const fn p(texture: &'static str, r: f32, g: f32, b: f32, radius: f32) -> CastCircleParams {
    CastCircleParams {
        texture,
        color_rgb: [r, g, b],
        base_radius: radius,
    }
}

// Beginspell — small (~5 unit) rune-circle under the caster, tinted by element.
pub const YELLOW: CastCircleParams = p("", 1.00, 0.90, 0.30, 5.0);
pub const WATER: CastCircleParams = p("", 0.30, 0.60, 1.00, 5.0);
pub const FIRE: CastCircleParams = p("", 1.00, 0.40, 0.15, 5.0);
pub const WIND: CastCircleParams = p("", 0.55, 1.00, 0.60, 5.0);
pub const EARTH: CastCircleParams = p("", 0.80, 0.55, 0.25, 5.0);
pub const HOLY: CastCircleParams = p("", 1.00, 0.95, 0.80, 5.0);
pub const POISON: CastCircleParams = p("", 0.70, 0.30, 0.85, 5.0);
pub const RED: CastCircleParams = p("", 1.00, 0.25, 0.25, 5.0);
pub const WHITE: CastCircleParams = p("", 0.95, 0.95, 1.00, 5.0);
pub const N_BLUE: CastCircleParams = p("", 0.55, 0.75, 1.00, 5.0);

// Beginasura — wider (~7 unit) rune-circle for Asura Strike chants.
pub const ASURA: CastCircleParams = p("", 1.00, 0.90, 0.30, 7.0);
pub const ASURA_EARTH: CastCircleParams = p("", 0.80, 0.55, 0.25, 7.0);
pub const ASURA_WIND: CastCircleParams = p("", 0.55, 1.00, 0.60, 7.0);
pub const ASURA_WATER: CastCircleParams = p("", 0.30, 0.60, 1.00, 7.0);
pub const ASURA_FIRE: CastCircleParams = p("", 1.00, 0.40, 0.15, 7.0);
pub const ASURA_UNDEAD: CastCircleParams = p("", 0.55, 0.45, 0.45, 7.0);
pub const ASURA_SHADOW: CastCircleParams = p("", 0.45, 0.20, 0.60, 7.0);
pub const ASURA_HOLY: CastCircleParams = p("", 1.00, 0.95, 0.80, 7.0);
pub const ASURA_CHAMPION: CastCircleParams = p("", 1.00, 0.85, 0.30, 9.0);

pub const TEXTURES: &[&str] = &[];

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
}

impl Effect for CastCircleEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        // The cast-circle lifetime is gated by the holder via the spec's
        // duration. The struct itself never self-terminates.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [r, g, b] = self.params.color_rgb;
        for i in 0..RING_COUNT {
            let theta_deg = RING_BASE_ROT_DEG[i] + RING_SPIN_DEG_PER_SEC[i] * self.age;
            let theta = theta_deg.to_radians();
            let (sin_t, cos_t) = theta.sin_cos();
            let rotate = |u: f32, v: f32| -> [f32; 2] {
                let cu = u - 0.5;
                let cv = v - 0.5;
                [cu * cos_t - cv * sin_t + 0.5, cu * sin_t + cv * cos_t + 0.5]
            };
            let uv = [
                rotate(0.0, 0.0),
                rotate(1.0, 0.0),
                rotate(0.0, 1.0),
                rotate(1.0, 1.0),
            ];
            let radius = self.params.base_radius * RING_RADII_SCALE[i];
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0],
                    self.world_pos[1] + RING_Y_OFFSETS[i],
                    self.world_pos[2],
                ],
                size: [radius * 2.0, radius * 2.0],
                uv,
                texture: self.params.texture,
                color: [r, g, b, RING_ALPHA],
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

    fn uv_of(prim: &EffectPrimitiveDraw) -> [[f32; 2]; 4] {
        match prim {
            EffectPrimitiveDraw::Billboard { uv, .. } => *uv,
            _ => panic!("expected billboard"),
        }
    }

    #[test]
    fn emits_three_rings_per_frame() {
        let c = CastCircleEffect::new(Attach::WorldPos([1.0, 2.0, 3.0]), YELLOW);
        let mut list = EffectDrawList::new();
        c.collect_draws(&mut list, &render_ctx());
        assert_eq!(list.len(), RING_COUNT);
        for prim in &list.primitives {
            let EffectPrimitiveDraw::Billboard { pos, blend, .. } = prim else {
                panic!("expected billboard");
            };
            assert!((pos[0] - 1.0).abs() < 1e-4);
            assert!((pos[2] - 3.0).abs() < 1e-4);
            assert_eq!(*blend, BlendKind::Additive);
        }
    }

    #[test]
    fn rings_spin_at_distinct_rates() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        let mut a = EffectDrawList::new();
        c.collect_draws(&mut a, &render_ctx());
        let uv_a = uv_of(&a.primitives[0]);
        c.update(&EffectUpdateCtx { delta: 0.5 });
        let mut b = EffectDrawList::new();
        c.collect_draws(&mut b, &render_ctx());
        let uv_b = uv_of(&b.primitives[0]);
        assert!(
            (uv_a[0][0] - uv_b[0][0]).abs() > 1e-4
                || (uv_a[0][1] - uv_b[0][1]).abs() > 1e-4,
            "UVs should rotate over time"
        );
    }

    #[test]
    fn variants_have_distinct_tints() {
        let palette = [
            YELLOW.color_rgb,
            WATER.color_rgb,
            FIRE.color_rgb,
            WIND.color_rgb,
            POISON.color_rgb,
        ];
        for i in 0..palette.len() {
            for j in (i + 1)..palette.len() {
                assert_ne!(palette[i], palette[j]);
            }
        }
    }

    #[test]
    fn asura_variant_is_wider_than_beginspell() {
        assert!(ASURA.base_radius > YELLOW.base_radius);
        assert!(ASURA_CHAMPION.base_radius > ASURA.base_radius);
    }

    #[test]
    fn never_self_terminates() {
        let mut c = CastCircleEffect::new(Attach::WorldPos([0.0; 3]), YELLOW);
        for _ in 0..200 {
            assert_eq!(c.update(&EffectUpdateCtx { delta: 0.1 }), EffectStatus::Running);
        }
    }
}
