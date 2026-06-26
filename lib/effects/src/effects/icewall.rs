//! `EF_ICEWALL` — Wizard Ice Wall (id 74).
//!
//! The original game lays the wall as one independent effect **per ground
//! cell**: the server sends one unit packet per occupied cell and the wall's
//! line shape comes entirely from those positions. Each cell sprouts three
//! near-vertical `ice.tga` blades clustered on the cell at **random**
//! Y-rotations and a slight random tilt — there is no per-blade orientation to
//! the wall line. Persistent until the cell's disappear packet kills it.
//!
//! Blades grow upward for ~20 frames then freeze (the speed-limit rise). The
//! cluster is seeded from the cell position so a given cell always sprouts the
//! same blades (deterministic for tests and the viewer).

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::ICE_TEXTURE;
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity, rise_step};

pub const TEXTURES: &[&str] = &[ICE_TEXTURE];

const BLADE_COUNT: usize = 3;
/// Near-vertical with a few degrees of random jitter (original `latitude`
/// 87..93°).
const TILT_BASE_DEG: f32 = 90.0;
const TILT_JITTER_DEG: f32 = 3.0;
const SIZE: f32 = 1.4;
const HEIGHT: f32 = 16.0;
/// Blades scatter along the cell (original steps `length` -2/0/2; ~½ scale).
const SCATTER_STEP: f32 = 1.0;

const SPIKE_SPEED_PER_S: f32 = 0.18 * FRAMES_PER_SECOND;
/// Blade grows for 20 frames then freezes.
const SPEED_LIMIT_S: f32 = 20.0 / FRAMES_PER_SECOND;
/// Blade alpha 200/255.
const ALPHA: f32 = 200.0 / 255.0;

struct Blade {
    base: [f32; 3],
    velocity: [f32; 3],
    heading_deg: f32,
    tilt_deg: f32,
}

pub struct IceWallEffect {
    blades: Vec<Blade>,
    age: f32,
}

impl IceWallEffect {
    /// Sprout the cell's three randomly-rotated blades around `center` (the
    /// unit cell's ground world position).
    pub fn new(center: [f32; 3]) -> Self {
        let seed = position_hash(&center);
        let blades = (0..BLADE_COUNT)
            .map(|i| {
                let salt = i as u64 * 7;
                let angle = rand_in_range(seed, salt, 0.0, std::f32::consts::TAU);
                let heading_deg = rand_in_range(seed, salt + 1, 0.0, 360.0);
                let tilt_deg = rand_in_range(
                    seed,
                    salt + 2,
                    TILT_BASE_DEG - TILT_JITTER_DEG,
                    TILT_BASE_DEG + TILT_JITTER_DEG,
                );
                // Original scatters each blade along the cell by `-length·cos`
                // (steps -1/0/1 here); the cross-axis offset is intentionally 0.
                let length = -SCATTER_STEP + i as f32 * SCATTER_STEP;
                let base = [center[0], center[1], center[2] - length * angle.cos()];
                Blade {
                    base,
                    velocity: apex_velocity(tilt_deg, heading_deg, SPIKE_SPEED_PER_S),
                    heading_deg,
                    tilt_deg,
                }
            })
            .collect();
        Self { blades, age: 0.0 }
    }
}

impl Effect for IceWallEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        for blade in &mut self.blades {
            rise_step(
                &mut blade.base,
                blade.velocity,
                self.age,
                ctx.delta,
                SPEED_LIMIT_S,
            );
        }
        self.age += ctx.delta;
        // Persistent: killed by the wall-destroyed packet, never self-expires.
        EffectStatus::Running
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for blade in &self.blades {
            out.push(EffectPrimitiveDraw::QuadHorn {
                base: blade.base,
                size: SIZE,
                height: HEIGHT,
                tilt_x_deg: blade.tilt_deg,
                rotation_y_deg: blade.heading_deg,
                texture: ICE_TEXTURE,
                color: [1.0, 1.0, 1.0, ALPHA],
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

    fn draws(e: &IceWallEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    #[test]
    fn cell_sprouts_three_tall_randomly_rotated_blades_and_persists() {
        // Sociable test: one cell yields three tall near-vertical ice blades
        // clustered on the cell, each at its own random Y-rotation, persistent.
        let e = IceWallEffect::new([5.0, 0.0, 7.0]);
        let prims = draws(&e);
        assert_eq!(prims.len(), BLADE_COUNT);
        let mut headings = vec![];
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn {
                base,
                height,
                texture,
                tilt_x_deg,
                rotation_y_deg,
                ..
            } = p
            else {
                panic!("expected QuadHorn");
            };
            assert_eq!(*texture, ICE_TEXTURE);
            assert!(*height > 12.0, "blades are tall");
            assert!(
                (*tilt_x_deg - 90.0).abs() <= TILT_JITTER_DEG,
                "near-vertical"
            );
            // Clustered on the cell (only scattered a little along Z).
            assert!((base[0] - 5.0).abs() < 1e-3, "no cross-axis offset");
            assert!(
                (base[2] - 7.0).abs() <= SCATTER_STEP + 1e-3,
                "scatter bounded"
            );
            headings.push(*rotation_y_deg);
        }
        assert!(
            headings.windows(2).any(|w| (w[0] - w[1]).abs() > 1.0),
            "blades carry distinct random rotations"
        );

        // Persistent: still Running far past any one-shot lifetime.
        let mut e = e;
        for _ in 0..1200 {
            assert_eq!(
                e.update(&EffectUpdateCtx {
                    delta: 1.0 / 60.0,
                    camera_target: None,
                    caster_yaw: None
                }),
                EffectStatus::Running
            );
        }
    }

    #[test]
    fn same_cell_is_deterministic_distinct_cells_differ() {
        // Seeded from the cell position: re-spawning the same cell reproduces
        // the blades; a different cell sprouts a different cluster.
        let headings = |c: [f32; 3]| -> Vec<f32> {
            draws(&IceWallEffect::new(c))
                .into_iter()
                .map(|p| match p {
                    EffectPrimitiveDraw::QuadHorn { rotation_y_deg, .. } => rotation_y_deg,
                    _ => panic!("expected QuadHorn"),
                })
                .collect()
        };
        assert_eq!(headings([5.0, 0.0, 7.0]), headings([5.0, 0.0, 7.0]));
        assert_ne!(headings([5.0, 0.0, 7.0]), headings([9.0, 0.0, 3.0]));
    }
}
