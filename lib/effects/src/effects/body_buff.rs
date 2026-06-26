//! Caster body-tint buffs — `EF_TWOHANDQUICKEN`, `EF_SPEARQUICKEN`,
//! `EF_LKCONCENTRATION`. Each tints the caster's body a fixed colour
//! while playing a `twohand.str` overlay and a one-shot sound:
//!
//! * Two-Hand Quicken / Spear Quicken — body tint `(200, 200, 0)` (yellow).
//! * LK Concentration — body tint `(255, 255, 160)` (pale yellow).
//! * Bunsinjyutsu — body tint `(155, 155, 255)` (light blue) with a
//!   same-tinted afterimage clone every 20 frames; no STR overlay and no sound.
//!
//! These are persistent buffs in the original game (kept
//! alive by the status); here they run for a self-contained window — the
//! game can override the duration when it ties them to a status. They emit no
//! world primitives: the visible parts are the STR overlay (rendered at the
//! caster via the holder's entity resolver) plus the body tint, applied by
//! the actor pass.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Afterimage, BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 120.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub tint: [u8; 3],
    /// STR overlay played at the caster, or `None` (Bunsinjyutsu only tints).
    pub str_name: Option<&'static str>,
    /// One-shot SFX (original game's backslash path), or `None`.
    pub sfx: Option<&'static str>,
    /// Movement afterimage trail, when the buff spawns blur clones.
    /// `None` for buffs that only tint (LK Concentration).
    pub afterimage: Option<Afterimage>,
    /// Quicken family: render the per-weapon swing trail (`검광`) during attacks.
    pub weapon_trail: bool,
}

/// The original game's blur: each clone is born at alpha `180/255` and loses
/// `4/255` per frame (~0.75 s lifetime).
const QUICKEN_BLUR: Afterimage = Afterimage {
    tint: [200, 200, 0],
    start_alpha: 180.0 / 255.0,
    fade_per_frame: 4.0 / 255.0,
};

pub const TWOHAND_QUICKEN: Params = Params {
    tint: [200, 200, 0],
    str_name: Some("twohand"),
    sfx: Some("effect\\knight_twohandquicken.wav"),
    afterimage: Some(QUICKEN_BLUR),
    weapon_trail: true,
};
pub const SPEAR_QUICKEN: Params = Params {
    tint: [200, 200, 0],
    str_name: Some("twohand"),
    sfx: Some("effect\\knight_twohandquicken.wav"),
    afterimage: Some(QUICKEN_BLUR),
    weapon_trail: true,
};
pub const LK_CONCENTRATION: Params = Params {
    tint: [255, 255, 160],
    str_name: Some("twohand"),
    sfx: Some("effect\\knight_twohandquicken.wav"),
    afterimage: None,
    weapon_trail: true,
};

/// `EF_BUNSINJYUTSU`: no STR / SFX — a light-blue body tint
/// `(155,155,255)` plus a same-tinted afterimage clone.
const BUNSIN_BLUR: Afterimage = Afterimage {
    tint: [155, 155, 255],
    start_alpha: 150.0 / 255.0,
    fade_per_frame: 4.0 / 255.0,
};
pub const BUNSINJYUTSU: Params = Params {
    tint: [155, 155, 255],
    str_name: None,
    sfx: None,
    afterimage: Some(BUNSIN_BLUR),
    weapon_trail: false,
};

pub const TEXTURES: &[&str] = &[];

pub struct BodyBuffEffect {
    params: Params,
    age_frames: f32,
    sfx_pending: bool,
    /// Status-tied lifetime in frames; the buff's `remain_ms` overrides the
    /// authored `TOTAL_FRAMES` so the tint persists while the EFST is active.
    life_frames: Option<f32>,
}

impl BodyBuffEffect {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            age_frames: 0.0,
            sfx_pending: true,
            life_frames: None,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        self.life_frames = ms.map(|m| m as f32 / 1000.0 * FPS);
        self
    }
}

impl Effect for BodyBuffEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= self.life_frames.unwrap_or(TOTAL_FRAMES) {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    /// No world primitives — the STR overlay and body tint are the visual.
    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn str_overlay(&self) -> Option<&'static str> {
        self.params.str_name
    }

    fn body_tint(&self) -> Option<BodyTint> {
        Some(BodyTint {
            rgb: self.params.tint,
        })
    }

    fn body_afterimage(&self) -> Option<Afterimage> {
        self.params.afterimage
    }

    fn weapon_trail(&self) -> bool {
        self.params.weapon_trail
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        if self.sfx_pending {
            self.sfx_pending = false;
            self.params.sfx
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut BodyBuffEffect, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn tints_and_overlays_str_with_one_shot_sfx() {
        let mut e = BodyBuffEffect::new(TWOHAND_QUICKEN);
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([200, 200, 0]));
        assert_eq!(e.str_overlay(), Some("twohand"));
        assert_eq!(
            e.take_sfx_request(),
            Some("effect\\knight_twohandquicken.wav")
        );
        assert_eq!(e.take_sfx_request(), None, "sfx is one-shot");

        // Quicken leaves a yellow movement trail; LK Concentration only
        // tints (paler yellow) with no afterimage.
        assert_eq!(e.body_afterimage().map(|a| a.tint), Some([200, 200, 0]));
        let lk = BodyBuffEffect::new(LK_CONCENTRATION);
        assert_eq!(lk.body_tint().map(|t| t.rgb), Some([255, 255, 160]));
        assert_eq!(lk.body_afterimage(), None);

        // The server's remaining time overrides the authored TOTAL_FRAMES
        // window so the tint persists for the whole status.
        let mut held = BodyBuffEffect::new(TWOHAND_QUICKEN).with_life_ms(Some(60_000));
        assert_eq!(step(&mut held, TOTAL_FRAMES + 1.0), EffectStatus::Running);
    }

    #[test]
    fn bunsinjyutsu_is_a_blue_tint_with_afterimage_and_no_str_or_sfx() {
        let mut e = BodyBuffEffect::new(BUNSINJYUTSU);
        assert_eq!(
            e.body_tint().map(|t| t.rgb),
            Some([155, 155, 255]),
            "light-blue tint"
        );
        let blur = e.body_afterimage().expect("afterimage clones");
        assert_eq!(blur.tint, [155, 155, 255]);
        assert_eq!(e.str_overlay(), None, "no STR overlay");
        assert_eq!(e.take_sfx_request(), None, "no sound");
    }

    #[test]
    fn emits_no_primitives_and_dies_after_window() {
        let mut e = BodyBuffEffect::new(SPEAR_QUICKEN);
        let mut list = EffectDrawList::new();
        e.collect_draws(
            &mut list,
            &EffectRenderCtx {
                camera: Default::default(),
                screen_w: 800.0,
                screen_h: 600.0,
                elapsed: 0.0,
            },
        );
        assert!(list.primitives.is_empty());
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
