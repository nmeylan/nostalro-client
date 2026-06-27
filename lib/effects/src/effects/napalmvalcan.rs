//! `EF_NAPALMVALCAN` (id 399) — five Hit2 bursts at frames 20, 30, 40, 50, 60.

use crate::draw::{EffectDrawList, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::hit2::Hit2Effect;

const FRAMES_PER_SECOND: f32 = 60.0;
const SPAWN_FRAMES: [f32; 5] = [20.0, 30.0, 40.0, 50.0, 60.0];
const HIT2_LIFETIME_FRAMES: f32 = 30.0;

pub const TOTAL_DURATION_MS: u32 =
    (((*SPAWN_FRAMES.last().unwrap() + HIT2_LIFETIME_FRAMES) as u32) * 1000)
        / FRAMES_PER_SECOND as u32;

pub struct NapalmValcanEffect {
    world_pos: [f32; 3],
    age: f32,
    next_burst_idx: usize,
    bursts: Vec<Hit2Effect>,
}

impl NapalmValcanEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age: 0.0,
            next_burst_idx: 0,
            bursts: Vec::with_capacity(SPAWN_FRAMES.len()),
        }
    }
}

impl Effect for NapalmValcanEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        let frame = self.age * FRAMES_PER_SECOND;

        while self.next_burst_idx < SPAWN_FRAMES.len() && frame >= SPAWN_FRAMES[self.next_burst_idx]
        {
            self.bursts.push(Hit2Effect::new(self.world_pos));
            self.next_burst_idx += 1;
        }

        let dt = ctx.delta;
        self.bursts.retain_mut(|child| {
            !matches!(
                child.update(&EffectUpdateCtx {
                    delta: dt,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Dead
            )
        });

        if self.next_burst_idx >= SPAWN_FRAMES.len() && self.bursts.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, ctx: &EffectRenderCtx) {
        for child in &self.bursts {
            child.collect_draws(out, ctx);
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

    fn step(e: &mut NapalmValcanEffect, dt: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        })
    }

    #[test]
    fn emits_one_hit2_per_scheduled_frame() {
        let mut e = NapalmValcanEffect::new([0.0; 3]);
        let dt = 1.0 / FRAMES_PER_SECOND;
        let mut seen_spawn_counts = Vec::new();
        let mut last_idx = 0;
        for _ in 0..70 {
            step(&mut e, dt);
            if e.next_burst_idx != last_idx {
                seen_spawn_counts.push(e.next_burst_idx - last_idx);
                last_idx = e.next_burst_idx;
            }
        }
        assert_eq!(seen_spawn_counts, vec![1, 1, 1, 1, 1]);

        let mut e = NapalmValcanEffect::new([0.0; 3]);
        for _ in 0..22 {
            step(&mut e, dt);
        }
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        let billboard_count = list
            .primitives
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::Billboard { .. }))
            .count();
        assert!(billboard_count > 0, "first burst is rendering petals");
    }

    #[test]
    fn dies_after_last_burst_finishes() {
        let mut e = NapalmValcanEffect::new([0.0; 3]);
        let total_s = TOTAL_DURATION_MS as f32 / 1000.0 + 0.5;
        let s = step(&mut e, total_s);
        assert!(matches!(s, EffectStatus::Dead));
    }
}
