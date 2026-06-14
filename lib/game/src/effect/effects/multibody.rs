//! Multi-render body lights — extra concentric copies of the actor sprite.
//!
//!
//! * **Reflectbody** (419) — four concentric white **alpha**
//!   ghosts that ripple outward 0..20px and fade, staggered so a new ring is
//!   always emerging (a repeated wave), drawn BEHIND a body dimmed to ~150 alpha.
//! * **Assumptio** (375) — one larger ghost copy behind the
//!   body, a doubled defensive body.
//! * **Lightblade** (382) — a light/spark weapon glow;
//!   approximated as a couple of small additive pale-blue copies (the original's
//!   spark-sword render is weapon-bone specific).
//!
//! No primitive — the copies are emitted via [`Effect::body_copies`] and drawn
//! by the shared composer, which scales them concentrically about the body
//! centre (matches `asurabody.rs`).

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{BodyCopy, BodyVertical, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;

/// Reflectbody animated ripple: `count` white ghosts whose outward margin
/// `add = i·step + phase·speed` wraps in `[0, wrap)` px, with alpha
/// `(alpha_base − add·alpha_falloff)/255` fading as it grows. The `i·step`
/// stagger spreads the ghosts across one cycle so a fresh ring is always
/// emerging (the repeated wave).
#[derive(Clone, Copy)]
struct Ripple {
    count: u8,
    /// Per-ghost base margin (`i·step` px) — the stagger across the cycle.
    step: f32,
    /// Margin wraps back to 0 at this many px (the ripple's reach).
    wrap: f32,
    /// Margin growth per frame.
    speed: f32,
    /// Alpha (0..255) at margin 0, falling by `alpha_falloff` per px.
    alpha_base: f32,
    alpha_falloff: f32,
}

#[derive(Clone, Copy)]
pub struct Params {
    copies: u8,
    /// Per-copy scale increment (`add = i·k` source units condensed to a scale).
    scale_step: f32,
    /// Alpha (0..=1) of the innermost copy.
    base_alpha: f32,
    /// Alpha lost per outer copy.
    alpha_step: f32,
    tint: [u8; 3],
    /// Additive (glow) vs alpha (ghost) blend.
    additive: bool,
    /// Draw the copies BEHIND the body (so an opaque body leaves its interior
    /// untouched and only the margin shows) vs on top.
    behind: bool,
    /// Live-body opacity (`<1.0` = translucent, as Reflectbody uses).
    body_alpha: f32,
    /// Reflectbody animated ripple, or `None`. When set it ignores the
    /// `copies`/`scale_step`/`base_alpha`/`alpha_step` fields above.
    ripple: Option<Ripple>,
    total_frames: f32,
}

impl Params {
    pub const fn total_duration_ms(&self) -> u32 {
        (self.total_frames / FPS * 1000.0) as u32
    }
}

pub const REFLECTBODY: Params = Params {
    copies: 4,
    scale_step: 0.0,
    base_alpha: 0.0,
    alpha_step: 0.0,
    tint: [255, 255, 255],
    additive: false,
    behind: true,
    // The original dims the live body to alpha 150 so the ghosts show through.
    body_alpha: 150.0 / 255.0,
    ripple: Some(Ripple {
        count: 4,
        step: 10.0,
        wrap: 20.0,
        speed: 0.1,
        alpha_base: 100.0,
        alpha_falloff: 5.0,
    }),
    total_frames: 120.0,
};

pub const ASSUMPTIO: Params = Params {
    // Assumptio: one larger white copy drawn ADDITIVELY and BEHIND the
    // body — it only adds a soft white glow at the margin, leaving the opaque
    // main sprite untouched (a darker alpha ghost would double the sprite).
    copies: 1,
    scale_step: 0.10,
    // Near-opaque additive white → a bright white margin glow (the original's
    // double body is a full-strength copy, not a faint ghost).
    base_alpha: 1.0,
    alpha_step: 0.0,
    tint: [255, 255, 255],
    additive: true,
    behind: true,
    body_alpha: 1.0,
    ripple: None,
    total_frames: 120.0,
};

pub const LIGHTBLADE: Params = Params {
    copies: 2,
    scale_step: 0.04,
    base_alpha: 0.4,
    alpha_step: 0.15,
    tint: [200, 220, 255],
    additive: true,
    behind: false,
    body_alpha: 1.0,
    ripple: None,
    total_frames: 120.0,
};

pub const TEXTURES: &[&str] = &[];

pub struct MultiBodyEffect {
    params: Params,
    age_frames: f32,
}

impl MultiBodyEffect {
    pub fn new(params: Params) -> Self {
        Self { params, age_frames: 0.0 }
    }
}

impl Effect for MultiBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= self.params.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_vertical(&self) -> Option<BodyVertical> {
        (self.params.body_alpha < 1.0)
            .then_some(BodyVertical { lift_px: 0.0, alpha: self.params.body_alpha, squeeze: 1.0 })
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        if let Some(ripple) = self.params.ripple {
            return Some(self.reflect_copies(ripple));
        }
        let mut copies = Vec::with_capacity(self.params.copies as usize);
        for i in 1..=self.params.copies {
            let i_f = i as f32;
            let alpha = self.params.base_alpha - (i_f - 1.0) * self.params.alpha_step;
            if alpha <= 0.0 {
                continue;
            }
            let scale = 1.0 + i_f * self.params.scale_step;
            copies.push(BodyCopy {
                // Composer scales about the body centre → concentric on all sides.
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [scale, scale],
                tint: self.params.tint,
                alpha,
                additive: self.params.additive,
                behind: self.params.behind,
            });
        }
        (!copies.is_empty()).then_some(copies)
    }
}

impl MultiBodyEffect {
    /// Reflectbody ripple: see [`Ripple`] for the model.
    fn reflect_copies(&self, ripple: Ripple) -> Vec<BodyCopy> {
        let phase = self.age_frames % 200.0;
        (1..=ripple.count)
            .filter_map(|i| {
                let mut add = i as f32 * ripple.step + phase * ripple.speed;
                if add >= ripple.wrap {
                    add -= ripple.wrap;
                }
                let mut alpha = (ripple.alpha_base - add * ripple.alpha_falloff) / 255.0;

                (alpha > 0.0).then_some(BodyCopy {
                    offset_px: [0.0, 0.0],
                    margin_px: add,
                    scale: [1.0, 1.0],
                    tint: [255, 255, 255],
                    alpha,
                    additive: false,
                    behind: true,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut MultiBodyEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None })
    }

    #[test]
    fn reflectbody_ripples_outward_with_a_fading_alpha() {
        let e = MultiBodyEffect::new(REFLECTBODY);
        let copies = e.body_copies().expect("ghosts");
        assert!(copies.len() >= 3, "several concentric ghosts");
        assert!(copies.iter().all(|c| !c.additive), "alpha-blended ghosts");
        // Growth is a small pixel margin (a few up to <20px), not a big scale.
        assert!(copies.iter().all(|c| c.margin_px < 20.0 && c.scale == [1.0, 1.0]));
        // A wider ghost is fainter (alpha fades as the ripple grows).
        let widest = copies.iter().max_by(|a, b| a.margin_px.total_cmp(&b.margin_px)).unwrap();
        let narrowest = copies.iter().min_by(|a, b| a.margin_px.total_cmp(&b.margin_px)).unwrap();
        assert!(widest.alpha < narrowest.alpha, "outer ring fainter");
    }

    #[test]
    fn assumptio_is_one_glow_behind_lightblade_glows_on_top() {
        let assumptio = MultiBodyEffect::new(ASSUMPTIO);
        let a = assumptio.body_copies().expect("halo");
        assert_eq!(a.len(), 1);
        // Additive glow BEHIND the body → white margin, main sprite untouched.
        assert!(a[0].additive && a[0].behind && a[0].scale[0] > 1.0, "larger glow behind");

        let lightblade = MultiBodyEffect::new(LIGHTBLADE);
        assert!(lightblade.body_copies().unwrap().iter().all(|c| c.additive && !c.behind), "glow on top");
    }

    #[test]
    fn reflectbody_dims_the_live_body() {
        let e = MultiBodyEffect::new(REFLECTBODY);
        assert!(e.body_vertical().unwrap().alpha < 1.0, "body is translucent");
    }

    #[test]
    fn dies_after_window() {
        let mut e = MultiBodyEffect::new(ASSUMPTIO);
        assert_eq!(step(&mut e, 121.0), EffectStatus::Dead);
    }
}
