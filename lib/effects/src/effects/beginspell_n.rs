//! `EF_BEGINSPELL_N` (id 730) — the yellow cast aura, relaunched on a loop.

use crate::draw::{BlendKind, EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::saint_casting::{SaintCastingConfig, SaintCastingEffect};

const FRAMES_PER_SECOND: f32 = 60.0;
const RELAUNCH_AT: u32 = 35;
const RELAUNCH_RESET: u32 = 1;

pub const TEXTURES: &[&str] = &["ring_yellow.tga"];

const CONFIG: SaintCastingConfig = SaintCastingConfig {
    texture: "ring_yellow.tga",
    pass_textures: None,
    max_heights: [20.0, 19.0, 18.0, 17.0],
    color_rgb: [1.0, 1.0, 170.0 / 255.0],
    blend: BlendKind::Additive,
    refill_per_frame: 10.0,
    reset_rise_deg: 74.0,
};

pub struct BeginspellNEffect {
    world_pos: [f32; 3],
    waves: Vec<SaintCastingEffect>,
    state_cnt: u32,
    age: f32,
    life_frames: Option<f32>,
}

impl BeginspellNEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            waves: vec![SaintCastingEffect::new(world_pos, CONFIG)],
            state_cnt: 0,
            age: 0.0,
            life_frames: None,
        }
    }

    pub fn with_life_ms(mut self, ms: Option<u32>) -> Self {
        self.life_frames = ms.map(|ms| (ms as f32 / 1000.0 * FRAMES_PER_SECOND).max(1.0));
        self
    }
}

impl Effect for BeginspellNEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let before = self.age * FRAMES_PER_SECOND;
        self.age += ctx.delta;
        let after = self.age * FRAMES_PER_SECOND;

        for _ in 0..(after.floor() - before.floor()).max(0.0) as u32 {
            self.state_cnt += 1;
            if self.state_cnt == RELAUNCH_AT {
                self.state_cnt = RELAUNCH_RESET;
                self.waves
                    .push(SaintCastingEffect::new(self.world_pos, CONFIG));
            }
        }

        self.waves
            .retain_mut(|w| w.update(ctx) == EffectStatus::Running);

        match self.life_frames {
            Some(life) if after >= life => EffectStatus::Dead,
            _ => EffectStatus::Running,
        }
    }

    fn set_position(&mut self, pos: [f32; 3]) {
        self.world_pos = pos;
        for w in &mut self.waves {
            w.set_position(pos);
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        for w in &self.waves {
            w.collect_draws(out, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::EffectPrimitiveDraw;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(e: &mut BeginspellNEffect, n: u32) {
        for _ in 0..n {
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            });
        }
    }

    fn ring_count(e: &BeginspellNEffect) -> usize {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::RadialRing { .. }))
            .count()
    }

    #[test]
    fn relaunches_a_second_wave_before_the_first_expires() {
        let mut e = BeginspellNEffect::new([0.0; 3]);
        step(&mut e, 18);
        assert_eq!(ring_count(&e), 8, "one wave up");
        step(&mut e, 35);
        assert!(
            ring_count(&e) > 8,
            "a second wave joined while the first is still alive"
        );
    }

    #[test]
    fn runs_past_a_single_wave_lifetime() {
        let mut e = BeginspellNEffect::new([0.0; 3]);
        step(&mut e, 200);
        assert_eq!(
            e.update(&EffectUpdateCtx {
                delta: 1.0 / 60.0,
                camera_target: None,
                caster_yaw: None,
            }),
            EffectStatus::Running
        );
        assert!(ring_count(&e) > 0, "still emitting after many waves");
    }
}
