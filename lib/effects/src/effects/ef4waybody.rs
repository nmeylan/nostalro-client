//! `EF_EF4WAYBODY` (425) — four sliding body ghosts.
//!
//! The centre body stays put while 4 alpha-blended white
//! copies slide out E / W / down / up by an offset that grows with body time
//! and sprite height, fading as `alpha = 150 − bodytime·5`. Each ghost
//! keeps the body's facing. No primitive — emitted via [`Effect::body_copies`].

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{BodyCopy, Effect, EffectRenderCtx, EffectUpdateCtx};

const FPS: f32 = 60.0;
/// `alpha = 150 − bodytime·5` reaches 0 at body time 30.
const END_FRAME: f32 = 90.0;
/// Offset grows with body time and sprite height; condensed to
/// screen pixels per frame (tune on a real actor).
const SLIDE_PER_FRAME: f32 = 9.0;

pub const TEXTURES: &[&str] = &[];

#[derive(Default)]
pub struct Ef4wayBodyEffect {
    age_frames: f32,
}

impl Ef4wayBodyEffect {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Effect for Ef4wayBodyEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FPS;
        if self.age_frames >= END_FRAME {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, _out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {}

    fn body_copies(&self) -> Option<Vec<BodyCopy>> {
        let bt = self.age_frames;
        let alpha = ((150.0 - bt * 5.0) / 255.0).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return None;
        }
        let add = bt * SLIDE_PER_FRAME;
        // E / W / down / up (screen +x = right, +y = down).
        let offsets = [[add, 0.0], [-add, 0.0], [0.0, add], [0.0, -add]];
        Some(
            offsets
                .iter()
                .map(|&offset_px| BodyCopy {
                    offset_px,
                    margin_px: 0.0,
                    scale: [1.0, 1.0],
                    tint: [255, 255, 255],
                    alpha,
                    additive: false,
                    behind: true,
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(e: &mut Ef4wayBodyEffect, frames: f32) {
        e.update(&EffectUpdateCtx { delta: frames / FPS, camera_target: None, caster_yaw: None });
    }

    #[test]
    fn four_ghosts_slide_out_and_fade() {
        let mut e = Ef4wayBodyEffect::new();
        step(&mut e, 5.0);
        let early = e.body_copies().expect("ghosts present");
        assert_eq!(early.len(), 4, "four cardinal ghosts");
        assert!(early.iter().all(|c| !c.additive), "alpha-blended");
        let spread_early = early[0].offset_px[0];
        step(&mut e, 10.0);
        let later = e.body_copies().unwrap();
        assert!(later[0].offset_px[0] > spread_early, "ghosts slide further out");
        assert!(later[0].alpha < early[0].alpha, "and fade");
    }
}
