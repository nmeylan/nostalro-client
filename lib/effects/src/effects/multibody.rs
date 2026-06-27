//! Multi-render body lights — extra concentric copies of the actor sprite.
//!
//!
//! * **Reflectbody** (419) — four concentric white **alpha**
//!   ghosts that ripple outward 0..20px and fade, staggered so a new ring is
//!   always emerging (a repeated wave), drawn BEHIND a body dimmed to ~150 alpha.
//! * **Assumptio** (375) — one additive white copy behind the body whose
//!   margin pulses cyclically (a doubled defensive body that breathes).
//! * **Lightblade** (382) — a light/spark weapon glow;
//!   approximated as a couple of small additive pale-blue copies (the original's
//!   spark-sword render is weapon-bone specific).
//! * **Undeadbody** (655) — two concentric additive green
//!   copies whose shared alpha rises with the body's clock then holds (a green
//!   glow that fades in).
//!
//! No primitive — the copies are emitted via [`Effect::body_copies`] and drawn
//! by the shared composer, which scales them concentrically about the body
//! centre (matches `asurabody.rs`).

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyCopy, BodyVertical, Effect, EffectRenderCtx, EffectUpdateCtx};

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

/// Assumptio "double body": one additive copy behind the live sprite whose
/// even pixel margin pulses cyclically — `base_px + amp_px·sin(phase°)` with
/// `phase` sweeping `0..period_frames` and wrapping. Mirrors the original
/// game's `m_BodySin` sweep (`sin·1.5 + 5`, 0..180 then reset), so the doubled
/// silhouette gently swells and shrinks rather than holding a fixed size.
#[derive(Clone, Copy)]
struct DoublePulse {
    base_px: f32,
    amp_px: f32,
    /// Frames for one full `0°..180°` sweep before the phase wraps.
    period_frames: f32,
    tint: [u8; 3],
}

/// Undeadbody aura: `count` concentric additive copies
/// each grown by `i·margin_unit` px, all sharing an alpha that rises with the
/// `min(age, ramp_frames)` clock and then holds at `max_alpha`.
#[derive(Clone, Copy)]
struct UndeadAura {
    count: u8,
    /// Per-copy outward margin (`i·margin_unit` px).
    margin_unit: f32,
    tint: [u8; 3],
    /// Frames over which the shared alpha ramps to `max_alpha`, then holds.
    ramp_frames: f32,
    max_alpha: f32,
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
    /// Undeadbody rising-alpha green aura, or `None`. Like `ripple`, overrides
    /// the static copy fields when set.
    undead: Option<UndeadAura>,
    /// Assumptio cyclic-pulse double body, or `None`. Like `ripple`, overrides
    /// the static copy fields when set.
    pulse: Option<DoublePulse>,
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
    undead: None,
    pulse: None,
    total_frames: 120.0,
};

pub const ASSUMPTIO: Params = Params {
    copies: 0,
    scale_step: 0.0,
    base_alpha: 1.0,
    alpha_step: 0.0,
    tint: [255, 255, 255],
    additive: true,
    behind: true,
    body_alpha: 1.0,
    ripple: None,
    undead: None,
    pulse: Some(DoublePulse {
        base_px: 8.0,
        amp_px: 2.0,
        period_frames: 180.0,
        tint: [255, 255, 255],
    }),
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
    undead: None,
    pulse: None,
    total_frames: 120.0,
};

pub const UNDEADBODY: Params = Params {
    // Undeadbody: two concentric additive green copies over the body whose
    // alpha rises with the body's clock — a green glow that fades in
    // and holds. A status-tied persistent buff in the original; finite here.
    copies: 0,
    scale_step: 0.0,
    base_alpha: 0.0,
    alpha_step: 0.0,
    tint: [5, 155, 5],
    additive: true,
    behind: false,
    body_alpha: 1.0,
    ripple: None,
    undead: Some(UndeadAura {
        count: 2,
        margin_unit: 5.0,
        tint: [5, 155, 5],
        ramp_frames: 200.0,
        max_alpha: 200.0 / 255.0,
    }),
    pulse: None,
    total_frames: 240.0,
};

pub const TEXTURES: &[&str] = &[];

pub struct MultiBodyEffect {
    params: Params,
    age_frames: f32,
    /// Status-tied lifetime in frames; the buff's `remain_ms` overrides the
    /// authored `total_frames` so the glow holds while the EFST is active
    /// (Assumptio / Undead property). `None` keeps the one-shot timing.
    life_frames: Option<f32>,
}

impl MultiBodyEffect {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            age_frames: 0.0,
            life_frames: None,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        self.life_frames = ms.map(|m| m as f32 / 1000.0 * FPS);
        self
    }
}

impl Effect for MultiBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= self.life_frames.unwrap_or(self.params.total_frames) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_vertical(&self) -> Option<BodyVertical> {
        (self.params.body_alpha < 1.0).then_some(BodyVertical {
            lift_px: 0.0,
            alpha: self.params.body_alpha,
            squeeze: 1.0,
        })
    }

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        if let Some(ripple) = self.params.ripple {
            return Some(self.reflect_copies(ripple));
        }
        if let Some(undead) = self.params.undead {
            return Some(self.undead_copies(undead));
        }
        if let Some(pulse) = self.params.pulse {
            return Some(vec![self.pulse_copy(pulse)]);
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
    /// Undeadbody aura: see [`UndeadAura`] for the model.
    fn undead_copies(&self, undead: UndeadAura) -> Vec<BodyCopy> {
        let alpha =
            (self.age_frames.min(undead.ramp_frames) / undead.ramp_frames) * undead.max_alpha;
        (1..=undead.count)
            .map(|i| BodyCopy {
                offset_px: [0.0, 0.0],
                margin_px: i as f32 * undead.margin_unit,
                scale: [1.0, 1.0],
                tint: undead.tint,
                alpha,
                additive: true,
                behind: false,
            })
            .collect()
    }

    /// Assumptio double body: see [`DoublePulse`] for the model. One additive
    /// copy behind the body whose even margin breathes as the phase sweeps a
    /// half-sine and wraps.
    fn pulse_copy(&self, pulse: DoublePulse) -> BodyCopy {
        let phase = self.age_frames % pulse.period_frames;
        let margin =
            pulse.base_px + pulse.amp_px * (phase / pulse.period_frames * std::f32::consts::PI).sin();
        BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: margin,
            scale: [1.0, 1.0],
            tint: pulse.tint,
            alpha: 1.0,
            additive: true,
            behind: true,
        }
    }

    /// Reflectbody ripple: see [`Ripple`] for the model.
    fn reflect_copies(&self, ripple: Ripple) -> Vec<BodyCopy> {
        let phase = self.age_frames % 200.0;
        (1..=ripple.count)
            .filter_map(|i| {
                let mut add = i as f32 * ripple.step + phase * ripple.speed;
                if add >= ripple.wrap {
                    add -= ripple.wrap;
                }
                let alpha = (ripple.alpha_base - add * ripple.alpha_falloff) / 255.0;

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
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn reflectbody_ripples_outward_with_a_fading_alpha() {
        let e = MultiBodyEffect::new(REFLECTBODY);
        let copies = e.body_copies().expect("ghosts");
        assert!(copies.len() >= 3, "several concentric ghosts");
        assert!(copies.iter().all(|c| !c.additive), "alpha-blended ghosts");
        // Growth is a small pixel margin (a few up to <20px), not a big scale.
        assert!(
            copies
                .iter()
                .all(|c| c.margin_px < 20.0 && c.scale == [1.0, 1.0])
        );
        // A wider ghost is fainter (alpha fades as the ripple grows).
        let widest = copies
            .iter()
            .max_by(|a, b| a.margin_px.total_cmp(&b.margin_px))
            .unwrap();
        let narrowest = copies
            .iter()
            .min_by(|a, b| a.margin_px.total_cmp(&b.margin_px))
            .unwrap();
        assert!(widest.alpha < narrowest.alpha, "outer ring fainter");
    }

    #[test]
    fn assumptio_is_one_glow_behind_whose_margin_pulses_cyclically() {
        let mut assumptio = MultiBodyEffect::new(ASSUMPTIO);
        let a = assumptio.body_copies().expect("halo");
        assert_eq!(a.len(), 1);
        // Additive glow BEHIND the body → white margin, main sprite untouched.
        // The pulse drives an even pixel margin, not a proportional scale.
        assert!(
            a[0].additive && a[0].behind && a[0].scale == [1.0, 1.0] && a[0].margin_px >= 5.0,
            "additive margin glow behind"
        );

        // The margin swells to its peak at the quarter-period (sin 90°) and
        // returns to the base at the half-period (sin 180°): a cyclic breath.
        let base = a[0].margin_px;
        step(&mut assumptio, 90.0);
        let peak = assumptio.body_copies().unwrap()[0].margin_px;
        step(&mut assumptio, 90.0);
        let back = assumptio.body_copies().unwrap()[0].margin_px;
        assert!(peak > base, "margin grows toward the peak");
        assert!((back - base).abs() < 0.1, "margin returns to base over a cycle");

        let lightblade = MultiBodyEffect::new(LIGHTBLADE);
        assert!(
            lightblade
                .body_copies()
                .unwrap()
                .iter()
                .all(|c| c.additive && !c.behind),
            "glow on top"
        );
    }

    #[test]
    fn reflectbody_dims_the_live_body() {
        let e = MultiBodyEffect::new(REFLECTBODY);
        assert!(
            e.body_vertical().unwrap().alpha < 1.0,
            "body is translucent"
        );
    }

    #[test]
    fn undeadbody_is_a_rising_green_additive_aura() {
        let mut e = MultiBodyEffect::new(UNDEADBODY);
        let early = e.body_copies().expect("aura");
        assert_eq!(early.len(), 2, "two concentric copies");
        assert!(
            early
                .iter()
                .all(|c| c.additive && !c.behind && c.tint == [5, 155, 5])
        );
        // The outer copy has the larger margin.
        assert!(
            early[1].margin_px > early[0].margin_px,
            "concentric expansion"
        );
        let early_alpha = early[0].alpha;
        step(&mut e, 100.0);
        let later_alpha = e.body_copies().unwrap()[0].alpha;
        assert!(later_alpha > early_alpha, "alpha rises with the body clock");
    }

    #[test]
    fn dies_after_window() {
        let mut e = MultiBodyEffect::new(ASSUMPTIO);
        assert_eq!(step(&mut e, 121.0), EffectStatus::Dead);
    }
}
