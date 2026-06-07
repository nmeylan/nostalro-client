//! Caster body-tint buffs — `EF_TWOHANDQUICKEN`, `EF_SPEARQUICKEN`,
//! `EF_LKCONCENTRATION`. Each tints the caster's body a fixed colour
//! while playing a `twohand.str` overlay and a one-shot sound:
//!
//! * Two-Hand Quicken / Spear Quicken — argb (-1, 200, 200, 0) (yellow).
//! * LK Concentration — color (255, 255, 160) (pale yellow).
//!
//! These are persistent buffs in the original game (kept
//! alive by the status); here they run for a self-contained window — the
//! game can override the duration when it ties them to a status. They emit no
//! world primitives: the visible parts are the STR overlay (rendered at the
//! caster via the holder's entity resolver) plus the body tint, applied by
//! the actor pass.

use crate::effect::draw::{EffectDrawList, EffectStatus};
use crate::effect::effect_trait::{Afterimage, BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 120.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub tint: [u8; 3],
    /// STR overlay played at the caster.
    pub str_name: &'static str,
    /// One-shot SFX (original game's backslash path).
    pub sfx: &'static str,
    /// Movement afterimage trail, when the buff spawns blur clones.
    /// `None` for buffs that only tint (LK Concentration).
    pub afterimage: Option<Afterimage>,
}

/// The original game's blur cadence: a clone every 5 frames, born at alpha
/// `180/255` and losing `4/255` per frame (~0.75 s lifetime).
const QUICKEN_BLUR: Afterimage = Afterimage {
    tint: [200, 200, 0],
    interval_frames: 5.0,
    start_alpha: 180.0 / 255.0,
    fade_per_frame: 4.0 / 255.0,
};

pub const TWOHAND_QUICKEN: Params = Params {
    tint: [200, 200, 0],
    str_name: "twohand",
    sfx: "effect\\knight_twohandquicken.wav",
    afterimage: Some(QUICKEN_BLUR),
};
pub const SPEAR_QUICKEN: Params = Params {
    tint: [200, 200, 0],
    str_name: "twohand",
    sfx: "effect\\knight_twohandquicken.wav",
    afterimage: Some(QUICKEN_BLUR),
};
pub const LK_CONCENTRATION: Params = Params {
    tint: [255, 255, 160],
    str_name: "twohand",
    sfx: "effect\\knight_twohandquicken.wav",
    afterimage: None,
};

pub const TEXTURES: &[&str] = &[];

pub struct BodyBuffEffect {
    params: Params,
    age_frames: f32,
    sfx_pending: bool,
}

impl BodyBuffEffect {
    pub fn new(params: Params) -> Self {
        Self {
            params,
            age_frames: 0.0,
            sfx_pending: true,
        }
    }
}

impl Effect for BodyBuffEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    /// No world primitives — the STR overlay and body tint are the visual.
    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn str_overlay(&self) -> Option<&'static str> {
        Some(self.params.str_name)
    }

    fn body_tint(&self) -> Option<BodyTint> {
        Some(BodyTint { rgb: self.params.tint })
    }

    fn body_afterimage(&self) -> Option<Afterimage> {
        self.params.afterimage
    }

    fn take_sfx_request(&mut self) -> Option<&'static str> {
        if self.sfx_pending {
            self.sfx_pending = false;
            Some(self.params.sfx)
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
            camera_target: None, caster_yaw: None,
        })
    }

    #[test]
    fn tints_and_overlays_str_with_one_shot_sfx() {
        let mut e = BodyBuffEffect::new(TWOHAND_QUICKEN);
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([200, 200, 0]));
        assert_eq!(e.str_overlay(), Some("twohand"));
        assert_eq!(e.take_sfx_request(), Some("effect\\knight_twohandquicken.wav"));
        assert_eq!(e.take_sfx_request(), None, "sfx is one-shot");

        // Quicken leaves a yellow movement trail; LK Concentration only
        // tints (paler yellow) with no afterimage.
        assert_eq!(e.body_afterimage().map(|a| a.tint), Some([200, 200, 0]));
        let lk = BodyBuffEffect::new(LK_CONCENTRATION);
        assert_eq!(lk.body_tint().map(|t| t.rgb), Some([255, 255, 160]));
        assert_eq!(lk.body_afterimage(), None);
    }

    #[test]
    fn emits_no_primitives_and_dies_after_window() {
        let mut e = BodyBuffEffect::new(SPEAR_QUICKEN);
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        });
        assert!(list.primitives.is_empty());
        assert_eq!(step(&mut e, TOTAL_FRAMES + 1.0), EffectStatus::Dead);
    }
}
