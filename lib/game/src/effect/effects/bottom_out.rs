//! Bottom_Out family — two pulsing camera-facing billboards anchored
//! on the actor's XZ position.
//!
//! These two billboards share the render path of the
//! Bottom_Music songs (both are camera-facing cloud quads) so we
//! reuse the existing `Billboard` primitive — a screen-facing
//! quad.
//!
//! Setup (F1=0 path, the only one EF_BOTTOM_ROKISWEIL uses):
//!   * 2 alive cells.
//!   * Per-cell `radius = 5 + random(6)` ∈ [5, 11), frozen at spawn —
//!     the quad's corner-radius (half-diagonal); on-screen edge =
//!     `radius * √2`.
//!   * Per-cell `phase = random(11)` ∈ [0, 11) — initial pulse-cycle
//!     phase offset so the two cells aren't in lock-step.
//!   * Both cells centered on the actor's feet (actor XZ, at the
//!     actor's Y level).
//!   * Tint `(130, 130, 250)`
//!     (light blue), additive blend.
//!
//! Per-frame animation:
//!   * phase advances one step.
//!   * For the first 10 frames alpha climbs +25/frame → 10-frame ramp up.
//!   * After that alpha falls -5/frame → 50-frame slow fade, and the
//!     billboard drifts -0.2/frame upward
//!     (native -Y up).
//!   * When alpha reaches 0: reset phase, reset Y back to the
//!     actor's feet, cycle repeats.
//!
//! So each cell pulses with a ~60-frame cycle (~1s @60fps) and rises
//! ~10 units during the fade phase before resetting.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

const FRAMES_PER_SECOND: f32 = 60.0;
const FADE_THRESHOLD_FRAMES: u32 = 10;
/// 10-frame ramp + 51 fade frames (250 → 0 stepping by 5/frame includes
/// the frame where alpha hits 0). The phase resets only after alpha
/// actually reaches 0 — that's frame 60 — so the cycle period
/// is 61 frames before the next ramp starts.
const CYCLE_LENGTH_FRAMES: u32 = 61;

#[derive(Clone, Copy, Debug)]
pub struct BottomOutParams {
    pub texture: &'static str,
    /// RGB tint multiplied into the billboard (0..1).
    pub tint_rgb: [f32; 3],
    /// Constant Y offset on top of the pulse-cycle drift. `F1=0` →
    /// no offset; `F1=1` (currently unused by any EffectId) → `-5.0`.
    pub vertical_offset: f32,
}

/// `EF_BOTTOM_ROKISWEIL` → `safeline.bmp`, F1=0.
/// Tint (130, 130, 250) light blue, additive.
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
    /// Per-cell corner-radius (half-diagonal of the quad). Each cell gets
    /// an independent random in [5, 11) at spawn; on-screen edge is
    /// `radius * √2`.
    cell_sizes: [f32; CELL_COUNT],
    /// Per-cell pulse-cycle phase offset, frozen at spawn
    /// (`random(11)`).
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
            // Ramp-up phase: alpha climbs from 25 to 250 over 10 frames.
            let alpha = ((t + 1) as f32 * 25.0 / 255.0).min(250.0 / 255.0);
            CellState {
                alpha,
                y_offset: 0.0,
            }
        } else {
            // Fade phase: alpha falls 250 → 0 over 50 frames; Y drifts
            // upward by 0.2 / frame (native -Y up).
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
            // The stored value is the corner-radius (half-diagonal), so
            // the on-screen edge is `radius * √2` — not `radius * 2`.
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
            camera_target: None, caster_yaw: None,
        });
    }

    fn draws(effect: &BottomOutEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn rokisweil_emits_blue_additive_billboards_within_size_band() {
        // Sociable test: two billboards centered on the actor XZ, blue
        // additive, half-size in [5, 11). Sampled inside the ramp-up
        // window so both cells are visible regardless of phase init.
        let mut e = BottomOutEffect::new([12.0, 5.0, 34.0], ROKISWEIL);
        // Step through several frames to give both cells time to be
        // past their ramp-in / phase init.
        step(&mut e, 0.2);
        let prims = draws(&e);
        // 1 or 2 billboards depending on phase (cells with alpha==0
        // are skipped). At dt=0.2 (12 frames), at least one cell will
        // be visible.
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
            // Centered on the actor XZ
            assert!((pos[0] - 12.0).abs() < 1e-3);
            assert!((pos[2] - 34.0).abs() < 1e-3);
            // radius 5..11 (corner-radius) → edge = radius*√2 ≈ 7.07..15.56
            assert!(
                (7.0..=15.6).contains(&size[0]) && size[0] == size[1],
                "size out of band: {:?}",
                size
            );
            // Blue-leaning tint: B > R, B > G
            assert!(color[2] > color[0] && color[2] > color[1]);
        }
    }

    #[test]
    fn cell_pulses_through_full_cycle_with_y_drift_during_fade() {
        // Track cell 0's alpha + Y offset across a full 60-frame cycle.
        // Phase offset is deterministic via position_hash; assert that:
        //   * ramp-up alphas (frames 0..10 in phase) > 0
        //   * fade alphas reach 0 (or close to it) by end of cycle
        //   * Y offset is more negative (drifting up) during fade
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
        assert!(max_alpha_seen >= 0.95, "alpha should peak near 1.0; got {max_alpha_seen}");
        assert!(zero_alpha_seen, "alpha should hit 0 at end of fade");
        // Y drifts upward (Y decreases in native -Y up).
        assert!(min_y < -5.0, "Y should drift up at least 5 units; got {min_y}");
    }

    #[test]
    fn two_cells_get_distinct_random_sizes() {
        // Different salts feed the hash mixer so the two cells get
        // visibly distinct sizes. Phase offsets are integers in [0,11)
        // so occasional collisions there are expected — sizes are the
        // stronger signal.
        let e = BottomOutEffect::new([7.0, 0.0, 11.0], ROKISWEIL);
        assert!(
            (e.cell_sizes[0] - e.cell_sizes[1]).abs() > 0.05,
            "cells should have distinct random sizes",
        );
    }
}
