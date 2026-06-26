//! EF_POTION_CON (Concentration Potion) and EF_POTION_ (Awakening Potion).
//!
//! Reference: `ro-effects/effects/imgs/200-250/218.gif` (concentration) and
//! `219.gif` (awakening).
//!
//! Both are the white [`super::potion_pillar`] cylinder column plus an STR
//! overlay — the yellow star-burst (concentration) and green ground ring
//! (awakening) live entirely in the STR files, so the cylinder itself stays
//! white in both. Same shape as [`super::potion_berserk`] minus the red
//! cross-texture sparks.
//!
//! Observed behaviour:
//! * Concentration — frame 0 → pillar with height speed 2.0
//!   (default 50-frame life) + the
//!   concentration STR overlay.
//! * Awakening — at frame 30 → pillar with height speed 1.9 over 30 frames
//!   + the awakening STR overlay. The STR runs from frame 0, so
//!   the column enters partway through.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::potion_pillar::{PotionPillarEffect, PotionPillarParams};

const FRAMES_PER_SECOND: f32 = 60.0;

#[derive(Clone, Copy, Debug)]
pub struct PotionConParams {
    pub pillar: PotionPillarParams,
    /// Parent frame at which the cylinder column launches.
    /// The STR overlay always plays from frame 0.
    pub launch_delay_frames: f32,
    /// Total effect lifetime — the STR overlay plays for as long as the effect
    /// is alive. Tuned to the reference gif so the column's growth/fade (and,
    /// for awakening, the lead-in capsule + sparkles) play out without a long
    /// empty tail.
    pub total_frames: f32,
}

/// EF_POTION_CON — pillar (height speed 2.0) at frame 0; column lives 50 frames.
pub const CONCENTRATION: PotionConParams = PotionConParams {
    pillar: PotionPillarParams {
        height_speed: 2.0,
        duration_frames: 50,
    },
    launch_delay_frames: 0.0,
    total_frames: 54.0,
};

/// EF_POTION_ — pillar (height speed 1.9, 30 frames) at frame 30; STR lead-in plays first,
/// column lives frames 30..60.
pub const AWAKENING: PotionConParams = PotionConParams {
    pillar: PotionPillarParams {
        height_speed: 1.9,
        duration_frames: 30,
    },
    launch_delay_frames: 30.0,
    total_frames: 63.0,
};

pub const CONCENTRATION_DURATION_MS: u32 =
    (CONCENTRATION.total_frames * 1000.0 / FRAMES_PER_SECOND) as u32;
pub const AWAKENING_DURATION_MS: u32 = (AWAKENING.total_frames * 1000.0 / FRAMES_PER_SECOND) as u32;

pub struct PotionConEffect {
    world_pos: [f32; 3],
    params: PotionConParams,
    str_name: &'static str,
    age: f32,
    pillar: Option<PotionPillarEffect>,
}

impl PotionConEffect {
    pub fn new(world_pos: [f32; 3], str_name: &'static str, params: PotionConParams) -> Self {
        Self {
            world_pos,
            params,
            str_name,
            age: 0.0,
            pillar: None,
        }
    }
}

impl Effect for PotionConEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let frame = self.age * FRAMES_PER_SECOND;

        // Launch the column once the delay elapses, then keep ticking it so it
        // runs its own ramp / fade / death from a frame-0 start. On the
        // launch frame, seed the column with only the time elapsed *since* the
        // delay so a large step doesn't overshoot its short lifetime.
        if frame >= self.params.launch_delay_frames {
            if let Some(pillar) = self.pillar.as_mut() {
                pillar.update(ctx);
            } else {
                let mut pillar = PotionPillarEffect::new(self.world_pos, self.params.pillar);
                let since_launch = self.age - self.params.launch_delay_frames / FRAMES_PER_SECOND;
                pillar.update(&EffectUpdateCtx {
                    delta: since_launch.max(0.0),
                    camera_target: ctx.camera_target,
                    caster_yaw: ctx.caster_yaw,
                });
                self.pillar = Some(pillar);
            }
        }

        if frame >= self.params.total_frames {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        if let Some(pillar) = self.pillar.as_ref() {
            pillar.collect_draws(out, ctx);
        }
    }

    fn str_overlay(&self) -> Option<&'static str> {
        Some(self.str_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::EffectPrimitiveDraw;

    fn ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut PotionConEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    fn has_cylinder(e: &PotionConEffect) -> bool {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &ctx());
        list.primitives
            .iter()
            .any(|p| matches!(p, EffectPrimitiveDraw::Cylinder { .. }))
    }

    #[test]
    fn concentration_shows_white_pillar_from_frame_zero_with_str_overlay() {
        let mut e = PotionConEffect::new([1.0, 2.0, 3.0], "집중", CONCENTRATION);
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        assert!(has_cylinder(&e), "column visible immediately");
        assert_eq!(e.str_overlay(), Some("집중"));
    }

    #[test]
    fn awakening_delays_pillar_until_frame_thirty_but_plays_str_from_start() {
        let mut e = PotionConEffect::new([0.0; 3], "awakening", AWAKENING);
        // STR overlay reported from the first frame.
        step(&mut e, 1.0 / FRAMES_PER_SECOND);
        assert_eq!(e.str_overlay(), Some("awakening"));
        assert!(!has_cylinder(&e), "no column before the launch delay");

        // Past frame 30 the column enters.
        step(&mut e, 35.0 / FRAMES_PER_SECOND);
        assert!(has_cylinder(&e), "column enters after the launch delay");
    }

    #[test]
    fn dies_after_total_duration() {
        let mut e = PotionConEffect::new([0.0; 3], "집중", CONCENTRATION);
        let total_s = CONCENTRATION.total_frames / FRAMES_PER_SECOND;
        assert!(matches!(step(&mut e, total_s + 0.1), EffectStatus::Dead));
    }
}
