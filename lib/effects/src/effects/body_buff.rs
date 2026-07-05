//! Body-tint buffs: Two-Hand Quicken / Spear Quicken / LK Concentration /
//! Bunsinjyutsu / Energy Coat / Overthrust. These are the original game's
//! `MakeBlur`-style buffs: they tint the caster's body and shed fading
//! afterimage copies, rather than drawing a detached world effect.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Afterimage, BodyTint, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
const TOTAL_FRAMES: f32 = 120.0;
pub const TOTAL_DURATION_MS: u32 = (TOTAL_FRAMES / FPS * 1000.0) as u32;

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub tint: [u8; 3],
    pub str_name: Option<&'static str>,
    pub sfx: Option<&'static str>,
    pub afterimage: Option<Afterimage>,
    pub weapon_trail: bool,
}

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

const ENERGY_COAT_BLUR: Afterimage = Afterimage {
    tint: [150, 175, 255],
    start_alpha: 150.0 / 255.0,
    fade_per_frame: 4.0 / 255.0,
};
pub const ENERGY_COAT: Params = Params {
    tint: [170, 190, 255],
    str_name: Some("energycoat"),
    sfx: None,
    afterimage: Some(ENERGY_COAT_BLUR),
    weapon_trail: false,
};

const OVERTHRUST_BLUR: Afterimage = Afterimage {
    tint: [255, 120, 120],
    start_alpha: 160.0 / 255.0,
    fade_per_frame: 4.0 / 255.0,
};
pub const OVERTHRUST: Params = Params {
    tint: [255, 150, 150],
    str_name: None,
    sfx: None,
    afterimage: Some(OVERTHRUST_BLUR),
    weapon_trail: true,
};

pub const TEXTURES: &[&str] = &[];

pub struct BodyBuffEffect {
    params: Params,
    age_frames: f32,
    sfx_pending: bool,
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
        assert_eq!(e.take_sfx_request(), None);

        assert_eq!(e.body_afterimage().map(|a| a.tint), Some([200, 200, 0]));
        let lk = BodyBuffEffect::new(LK_CONCENTRATION);
        assert_eq!(lk.body_tint().map(|t| t.rgb), Some([255, 255, 160]));
        assert_eq!(lk.body_afterimage(), None);

        let mut held = BodyBuffEffect::new(TWOHAND_QUICKEN).with_life_ms(Some(60_000));
        assert_eq!(step(&mut held, TOTAL_FRAMES + 1.0), EffectStatus::Running);
    }

    #[test]
    fn energy_coat_and_overthrust_change_the_body_not_a_detached_effect() {
        let ec = BodyBuffEffect::new(ENERGY_COAT);
        assert!(ec.body_tint().is_some(), "energy coat tints the body");
        assert!(ec.body_afterimage().is_some(), "energy coat sheds afterimages");
        assert_eq!(ec.str_overlay(), Some("energycoat"));

        let ot = BodyBuffEffect::new(OVERTHRUST);
        assert!(ot.body_tint().is_some(), "overthrust tints the body");
        assert!(ot.body_afterimage().is_some(), "overthrust sheds afterimages");
    }

    #[test]
    fn bunsinjyutsu_is_a_blue_tint_with_afterimage_and_no_str_or_sfx() {
        let mut e = BodyBuffEffect::new(BUNSINJYUTSU);
        assert_eq!(e.body_tint().map(|t| t.rgb), Some([155, 155, 255]));
        let blur = e.body_afterimage().expect("afterimage clones");
        assert_eq!(blur.tint, [155, 155, 255]);
        assert_eq!(e.str_overlay(), None);
        assert_eq!(e.take_sfx_request(), None);
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
