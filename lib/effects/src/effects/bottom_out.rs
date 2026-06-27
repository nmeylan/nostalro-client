use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_THRESHOLD_FRAMES: u32 = 10;
const CYCLE_LENGTH_FRAMES: u32 = 61;

#[derive(Clone, Copy, Debug)]
pub struct BottomOutParams {
    pub texture: &'static str,
    pub tint_rgb: [f32; 3],
    pub vertical_offset: f32,
}

pub const ROKISWEIL: BottomOutParams = BottomOutParams {
    texture: "safeline.bmp",
    tint_rgb: [130.0 / 255.0, 130.0 / 255.0, 250.0 / 255.0],
    vertical_offset: 0.0,
};

pub const TEXTURES: &[&str] = &["safeline.bmp"];

const CELL_COUNT: usize = 2;

pub struct BottomOutEffect {
    world_pos: [f32; 3],
    params: BottomOutParams,
    age: f32,
    frames: u32,
    cell_sizes: [f32; CELL_COUNT],
    cell_phase_init: [u32; CELL_COUNT],
}

impl BottomOutEffect {
    pub fn new(world_pos: [f32; 3], params: BottomOutParams) -> Self {
        let seed = position_hash(&world_pos);
        Self {
            world_pos,
            params,
            age: 0.0,
            frames: 0,
            cell_sizes: [
                5.0 + rand_in_range(seed, 1, 0.0, 6.0),
                5.0 + rand_in_range(seed, 2, 0.0, 6.0),
            ],
            cell_phase_init: [
                rand_in_range(seed, 3, 0.0, 11.0) as u32,
                rand_in_range(seed, 4, 0.0, 11.0) as u32,
            ],
        }
    }

    fn cell_state(&self, cell: usize) -> CellState {
        let t = (self.frames + self.cell_phase_init[cell]) % CYCLE_LENGTH_FRAMES;
        if t < FADE_THRESHOLD_FRAMES {
            let alpha = ((t + 1) as f32 * 25.0 / 255.0).min(250.0 / 255.0);
            CellState {
                alpha,
                y_offset: 0.0,
            }
        } else {
            let fade_t = (t - FADE_THRESHOLD_FRAMES) as f32;
            let alpha = ((250.0 - fade_t * 5.0).max(0.0)) / 255.0;
            CellState {
                alpha,
                y_offset: fade_t * -0.2,
            }
        }
    }
}

struct CellState {
    alpha: f32,
    y_offset: f32,
}

impl Effect for BottomOutEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.delta;
        self.frames = (self.age * FRAMES_PER_SECOND) as u32;
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let [tr, tg, tb] = self.params.tint_rgb;
        for cell in 0..CELL_COUNT {
            let st = self.cell_state(cell);
            if st.alpha <= 0.0 {
                continue;
            }
            let side = self.cell_sizes[cell] * std::f32::consts::SQRT_2;
            out.push(EffectPrimitiveDraw::Billboard {
                pos: [
                    self.world_pos[0],
                    self.world_pos[1] + self.params.vertical_offset + st.y_offset,
                    self.world_pos[2],
                ],
                size: [side, side],
                uv: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                rotation: 0.0,
                texture: self.params.texture,
                color: [tr, tg, tb, st.alpha],
                blend: BlendKind::Additive,
            });
        }
    }
}

fn position_hash(pos: &[f32; 3]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    pos[0].to_bits().hash(&mut h);
    pos[1].to_bits().hash(&mut h);
    pos[2].to_bits().hash(&mut h);
    h.finish()
}

fn rand_in_range(seed: u64, salt: u64, lo: f32, hi: f32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(salt);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    let t = ((x >> 40) as f32) / ((1u64 << 24) as f32);
    lo + t * (hi - lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    fn step(effect: &mut BottomOutEffect, dt: f32) {
        effect.update(&EffectUpdateCtx {
            delta: dt,
            camera_target: None,
            caster_yaw: None,
        });
    }

    fn draws(effect: &BottomOutEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn rokisweil_emits_blue_additive_billboards_within_size_band() {
        let mut e = BottomOutEffect::new([12.0, 5.0, 34.0], ROKISWEIL);
        step(&mut e, 0.2);
        let prims = draws(&e);
        assert!(
            (1..=2).contains(&prims.len()),
            "expected 1-2 billboards, got {}",
            prims.len()
        );

        for p in &prims {
            let EffectPrimitiveDraw::Billboard {
                pos,
                size,
                color,
                blend,
                texture,
                ..
            } = p
            else {
                panic!("expected Billboard");
            };
            assert_eq!(*blend, BlendKind::Additive);
            assert_eq!(*texture, "safeline.bmp");
            assert!((pos[0] - 12.0).abs() < 1e-3);
            assert!((pos[2] - 34.0).abs() < 1e-3);
            assert!(
                (7.0..=15.6).contains(&size[0]) && size[0] == size[1],
                "size out of band: {:?}",
                size
            );
            assert!(color[2] > color[0] && color[2] > color[1]);
        }
    }

    #[test]
    fn cell_pulses_through_full_cycle_with_y_drift_during_fade() {
        let mut e = BottomOutEffect::new([0.0, 0.0, 0.0], ROKISWEIL);

        let mut min_y = 0.0_f32;
        let mut max_alpha_seen = 0.0_f32;
        let mut zero_alpha_seen = false;
        for frame in 0..(CYCLE_LENGTH_FRAMES * 2) {
            // Force-set frame count to step through deterministically.
            e.frames = frame;
            let st = e.cell_state(0);
            min_y = min_y.min(st.y_offset);
            max_alpha_seen = max_alpha_seen.max(st.alpha);
            if st.alpha == 0.0 {
                zero_alpha_seen = true;
            }
        }
        assert!(
            max_alpha_seen >= 0.95,
            "alpha should peak near 1.0; got {max_alpha_seen}"
        );
        assert!(zero_alpha_seen, "alpha should hit 0 at end of fade");
        assert!(
            min_y < -5.0,
            "Y should drift up at least 5 units; got {min_y}"
        );
    }

    #[test]
    fn two_cells_get_distinct_random_sizes() {
        let e = BottomOutEffect::new([7.0, 0.0, 11.0], ROKISWEIL);
        assert!(
            (e.cell_sizes[0] - e.cell_sizes[1]).abs() > 0.05,
            "cells should have distinct random sizes",
        );
    }
}
