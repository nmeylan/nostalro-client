//! Multi-render body lights — `Reflectbody` (419), `Assumptio` (375), `Lightblade` (382), `Undeadbody` (655).

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{
    BodyCopy, BodyVertical, Effect, EffectRenderCtx, EffectUpdateCtx, WeaponLight,
};

const FPS: f32 = 60.0;

#[derive(Clone, Copy)]
struct Ripple {
    count: u8,
    step: f32,
    wrap: f32,
    speed: f32,
    alpha_base: f32,
    alpha_falloff: f32,
}

#[derive(Clone, Copy)]
struct DoublePulse {
    base_px: f32,
    amp_px: f32,
    period_frames: f32,
    tint: [u8; 3],
}

#[derive(Clone, Copy)]
struct UndeadAura {
    count: u8,
    margin_unit: f32,
    tint: [u8; 3],
    ramp_frames: f32,
    max_alpha: f32,
}

#[derive(Clone, Copy)]
pub struct Params {
    copies: u8,
    scale_step: f32,
    base_alpha: f32,
    alpha_step: f32,
    tint: [u8; 3],
    additive: bool,
    behind: bool,
    body_alpha: f32,
    /// When set, overrides `copies`/`scale_step`/`base_alpha`/`alpha_step`.
    ripple: Option<Ripple>,
    /// When set, overrides the static copy fields.
    undead: Option<UndeadAura>,
    /// When set, overrides the static copy fields.
    pulse: Option<DoublePulse>,
    weapon_light: WeaponLight,
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
    body_alpha: 150.0 / 255.0,
    ripple: Some(Ripple {
        count: 4,
        step: 5.0,
        wrap: 20.0,
        speed: 0.1,
        alpha_base: 100.0,
        alpha_falloff: 5.0,
    }),
    undead: None,
    pulse: None,
    weapon_light: WeaponLight::None,
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
        base_px: 5.0,
        amp_px: 1.5,
        period_frames: 90.0,
        tint: [255, 255, 255],
    }),
    weapon_light: WeaponLight::None,
    total_frames: 120.0,
};

/// The weapon is drawn a second time additively on alternating frames, so the
/// blade pulses without changing the actor.
pub const LIGHTBLADE: Params = Params {
    copies: 0,
    scale_step: 0.0,
    base_alpha: 0.0,
    alpha_step: 0.0,
    tint: [255, 255, 255],
    additive: true,
    behind: false,
    body_alpha: 1.0,
    ripple: None,
    undead: None,
    pulse: None,
    weapon_light: WeaponLight::Spark,
    total_frames: 120.0,
};

/// The blue-white sword light: a steadily glowing weapon. On a player this
/// lights the weapon only — the body halo and blue cast belong to the separate
/// path the original uses for monsters and NPCs.
pub const LIGHTSWORD: Params = Params {
    weapon_light: WeaponLight::Glow,
    ..LIGHTBLADE
};

pub const UNDEADBODY: Params = Params {
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
        margin_unit: 1.0,
        tint: [5, 155, 5],
        ramp_frames: 200.0,
        max_alpha: 200.0 / 255.0,
    }),
    pulse: None,
    weapon_light: WeaponLight::None,
    total_frames: 240.0,
};

pub const TEXTURES: &[&str] = &[];

pub struct MultiBodyEffect {
    params: Params,
    age_frames: f32,
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

    fn body_weapon_light(&self) -> WeaponLight {
        match self.params.weapon_light {
            WeaponLight::Spark if (self.age_frames as u32) % 2 != 0 => WeaponLight::None,
            light => light,
        }
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
                offset_px: [0.0, 0.0],
                margin_px: 0.0,
                scale: [scale, scale],
                tint: self.params.tint,
                alpha,
                additive: self.params.additive,
                behind: self.params.behind,
                body_layers_only: false,
            });
        }
        (!copies.is_empty()).then_some(copies)
    }
}

impl MultiBodyEffect {
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
                body_layers_only: false,
            })
            .collect()
    }

    fn pulse_copy(&self, pulse: DoublePulse) -> BodyCopy {
        let phase = self.age_frames % pulse.period_frames;
        let margin = pulse.base_px
            + pulse.amp_px * (phase / pulse.period_frames * std::f32::consts::PI).sin();
        BodyCopy {
            offset_px: [0.0, 0.0],
            margin_px: margin,
            scale: [1.0, 1.0],
            tint: pulse.tint,
            alpha: 1.0,
            additive: true,
            behind: true,
            body_layers_only: true,
        }
    }

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
                    body_layers_only: false,
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
        assert_eq!(copies.len(), 4, "every ring survives the 20px wrap");
        assert!(copies.iter().all(|c| !c.additive), "alpha-blended ghosts");
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
        assert!(
            a[0].additive && a[0].behind && a[0].scale == [1.0, 1.0] && a[0].body_layers_only,
            "additive margin glow behind, sparing the weapon"
        );
        assert!((a[0].margin_px - 5.0).abs() < 1e-4, "5px at the trough");

        step(&mut assumptio, 45.0);
        let peak = assumptio.body_copies().unwrap()[0].margin_px;
        assert!((peak - 6.5).abs() < 1e-3, "6.5px at the crest");
        step(&mut assumptio, 45.0);
        let back = assumptio.body_copies().unwrap()[0].margin_px;
        assert!(
            (back - 5.0).abs() < 0.1,
            "margin returns to base over a cycle"
        );
    }

    #[test]
    fn sword_lights_touch_the_weapon_only_and_the_spark_skips_every_other_frame() {
        let mut spark = MultiBodyEffect::new(LIGHTBLADE);
        assert_eq!(spark.body_weapon_light(), WeaponLight::Spark);
        step(&mut spark, 1.0);
        assert_eq!(
            spark.body_weapon_light(),
            WeaponLight::None,
            "skips every other frame"
        );
        step(&mut spark, 1.0);
        assert_eq!(spark.body_weapon_light(), WeaponLight::Spark);

        let mut glow = MultiBodyEffect::new(LIGHTSWORD);
        assert_eq!(glow.body_weapon_light(), WeaponLight::Glow);
        step(&mut glow, 1.0);
        assert_eq!(
            glow.body_weapon_light(),
            WeaponLight::Glow,
            "held every frame"
        );

        // Neither touches the actor: no halo, no tint, no dimming.
        for e in [&spark, &glow] {
            assert!(e.body_copies().is_none());
            assert!(e.body_tint().is_none());
            assert!(e.body_vertical().is_none());
        }
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
        assert_eq!(
            [early[0].margin_px, early[1].margin_px],
            [1.0, 2.0],
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
