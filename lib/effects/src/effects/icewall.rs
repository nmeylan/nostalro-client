//! `EF_ICEWALL` — Wizard Ice Wall (id 74).
//!
//! In the original game the wall is three tall vertical `ice.tga` blades in a
//! short row, persistent until the wall is destroyed. Blades are near-vertical,
//! tall and chunky, spaced about 2 units apart. Our blades are near-vertical
//! (tilt ~90°), tall (height ~16), chunky (size ~1.4) and spaced tightly.
//!
//!
//! The original game lays the row at a fixed orientation; we instead orient
//! the wall by the **cast direction** (the caster→target trail anchor) so the
//! wall stands across the targeted line, like the in-game crosshair shows.
//! The row runs perpendicular to that direction. Heights/sizes are scaled to
//! our world units (~½ of the original height) and the spacing kept tight.
//!
//! The wall is just these three blades; no extra sparkle particle accompanies
//! it, so none is emitted here.
//!

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effects::frost_diver::ICE_TEXTURE;
use crate::effects::spike_util::{FRAMES_PER_SECOND, apex_velocity, rise_step};

pub const TEXTURES: &[&str] = &[ICE_TEXTURE];

const BLADE_COUNT: usize = 3;
/// Tight spacing between blades (the original steps 2 units; ~½ scale here).
const ROW_SPACING: f32 = 1.3;
const TILT_DEG: f32 = 90.0;
const SIZE: f32 = 1.4;
const HEIGHT: f32 = 16.0;

const SPIKE_SPEED_PER_S: f32 = 0.18 * FRAMES_PER_SECOND;
/// Blade grows for 20 frames then freezes.
const SPEED_LIMIT_S: f32 = 20.0 / FRAMES_PER_SECOND;
/// Blade alpha 200/255.
const ALPHA: f32 = 200.0 / 255.0;

struct Blade {
    base: [f32; 3],
    velocity: [f32; 3],
    heading_deg: f32,
}

pub struct IceWallEffect {
    blades: Vec<Blade>,
    age: f32,
}

impl IceWallEffect {
    /// `from` is the wall's ground location; `to` provides the cast
    /// direction. When the two coincide (no direction available) the wall
    /// defaults to a row along world +X.
    pub fn new(from: [f32; 3], to: [f32; 3]) -> Self {
        let dx = to[0] - from[0];
        let dz = to[2] - from[2];
        let len = (dx * dx + dz * dz).sqrt();
        // Cast direction (defaults to +Z) and the perpendicular the row runs
        // along (defaults to +X).
        let (dir_x, dir_z) = if len > 1e-3 {
            (dx / len, dz / len)
        } else {
            (0.0, 1.0)
        };
        let (perp_x, perp_z) = (-dir_z, dir_x);
        // Blades stand facing along the cast direction.
        let heading_deg = dir_x.atan2(dir_z).to_degrees();

        let blades = (0..BLADE_COUNT)
            .map(|i| {
                let offset = (i as f32 - (BLADE_COUNT as f32 - 1.0) / 2.0) * ROW_SPACING;
                Blade {
                    base: [
                        from[0] + perp_x * offset,
                        from[1],
                        from[2] + perp_z * offset,
                    ],
                    velocity: apex_velocity(TILT_DEG, heading_deg, SPIKE_SPEED_PER_S),
                    heading_deg,
                }
            })
            .collect();
        Self { blades, age: 0.0 }
    }
}

impl Effect for IceWallEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        for blade in &mut self.blades {
            rise_step(&mut blade.base, blade.velocity, self.age, ctx.delta, SPEED_LIMIT_S);
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
                tilt_x_deg: TILT_DEG,
                rotation_y_deg: blade.heading_deg,
                texture: ICE_TEXTURE,
                color: [1.0, 1.0, 1.0, ALPHA],
                blend: BlendKind::Additive,
            });
        }
    }
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

    fn bases(e: &IceWallEffect) -> Vec<[f32; 3]> {
        draws(e)
            .into_iter()
            .map(|p| match p {
                EffectPrimitiveDraw::QuadHorn { base, .. } => base,
                _ => panic!("expected QuadHorn"),
            })
            .collect()
    }

    #[test]
    fn row_runs_perpendicular_to_cast_direction_and_persists() {
        // Sociable test: three tall ice blades, the row laid perpendicular to
        // the caster→target direction, persistent. Casting along +Z lays the
        // row along the X axis (shared Z); casting along +X lays it along Z.
        let along_z = IceWallEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 10.0]);
        let prims = draws(&along_z);
        assert_eq!(prims.len(), 3);
        for p in &prims {
            let EffectPrimitiveDraw::QuadHorn { height, texture, tilt_x_deg, .. } = p else {
                panic!("expected QuadHorn");
            };
            assert_eq!(*texture, ICE_TEXTURE);
            assert!(*height > 12.0, "blades are tall");
            assert!((*tilt_x_deg - 90.0).abs() < 15.0, "near-vertical");
        }
        // Row spans X, shares Z.
        for b in bases(&along_z) {
            assert!(b[2].abs() < 1e-3, "row shares Z when casting along Z");
        }
        let xs: Vec<f32> = bases(&along_z).iter().map(|b| b[0]).collect();
        assert!(xs.iter().any(|x| *x < -1e-3) && xs.iter().any(|x| *x > 1e-3));

        // Casting along +X rotates the row to span Z instead.
        let along_x = IceWallEffect::new([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        for b in bases(&along_x) {
            assert!(b[0].abs() < 1e-3, "row shares X when casting along X");
        }
        let zs: Vec<f32> = bases(&along_x).iter().map(|b| b[2]).collect();
        assert!(zs.iter().any(|z| *z < -1e-3) && zs.iter().any(|z| *z > 1e-3));

        // Persistent: still Running far past any one-shot lifetime.
        let mut e = along_z;
        for _ in 0..1200 {
            assert_eq!(
                e.update(&EffectUpdateCtx { delta: 1.0 / 60.0, camera_target: None, caster_yaw: None }),
                EffectStatus::Running
            );
        }
    }

    #[test]
    fn blades_are_tightly_spaced() {
        // Sociable test: adjacent blades sit ROW_SPACING apart — tighter than
        // a character width so the wall reads as a solid barrier.
        let e = IceWallEffect::new([0.0, 0.0, 0.0], [0.0, 0.0, 5.0]);
        let mut xs: Vec<f32> = bases(&e).iter().map(|b| b[0]).collect();
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((xs[1] - xs[0] - ROW_SPACING).abs() < 1e-3);
        assert!((xs[2] - xs[1] - ROW_SPACING).abs() < 1e-3);
    }
}
