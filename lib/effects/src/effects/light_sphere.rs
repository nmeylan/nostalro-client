//! `EF_LIGHTSPHERE` (348) and `EF_LIGHTSPHERE2` (381).
//!
//! * **`Lightsphere` (348)** — `LightSphere("effect\\white02.bmp")`, additive.
//!   It is **not**
//!   a ball: each entry draws two crossed degenerate quads — a thin
//!   light-blade needle radiating from the centre toward a fixed random 3D
//!   direction, its reach `distance*(1 + sin(angle))` pulsing in and out
//!   so the blade keeps stabbing outward and retracting. The original spawns 4;
//!   in-game it reads as a dense burst of light blades piercing in every
//!   direction, so we radiate a full field of them ([`BLADE_COUNT`]) over an
//!   evenly-covered sphere, each at its own phase, additive bluish
//!   `(105,105,255)`/`(55,55,225)`. The accumulated additive cores light a
//!   bright glowing centre exactly like the original.
//! * **`Lightsphere2` (381)** — the same blade burst as 348, but persistent
//!   (a buff aura the holder reaps), so it keeps pulsing while the status lasts.

use crate::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const WHITE02_TEXTURE: &str = "white02.bmp";
pub const TEXTURES: &[&str] = &[WHITE02_TEXTURE];

const FPS: f32 = 60.0;
const UNIT_UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];

// ── 348 Lightsphere — a dense burst of pulsing light blades ─────────────────
const BLADE_TOTAL_FRAMES: f32 = 600.0;
/// Number of radiating blades. The original spawns 4; we fill a full sphere so
/// it reads as light blades piercing in every direction.
pub const BLADE_COUNT: usize = 160;
/// Reach factor: starts at 1 and creeps outward via `distance *= 1.02` each
/// frame, capped at 10.
const DISTANCE_START: f32 = 1.0;
const DISTANCE_CAP: f32 = 10.0;
const DISTANCE_GROWTH_PER_FRAME: f32 = 1.02;
/// Half-width of each blade's base cross.
const BLADE_HALF_WIDTH: f32 = 0.5;
/// Centre lift (native coords have −Y up).
const BLADE_LIFT: f32 = 10.0;
/// Per-blade additive alpha.
const BLADE_ALPHA: f32 = 40.0 / 255.0;
const BLADE_FADE_IN_FRAMES: f32 = 20.0;
/// Original game fades alpha once `process > 500`, one step per frame.
const BLADE_FADE_OUT_START: f32 = 500.0;
const BLADE_FADE_OUT_FRAMES: f32 = 40.0;

const BLADE_COLOR_A: [f32; 3] = [105.0 / 255.0, 105.0 / 255.0, 255.0 / 255.0];
const BLADE_COLOR_B: [f32; 3] = [55.0 / 255.0, 55.0 / 255.0, 225.0 / 255.0];

pub const fn lightsphere_duration_ms() -> u32 {
    (BLADE_TOTAL_FRAMES / FPS * 1000.0) as u32
}

fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// One radiating light blade: a fixed outward direction plus a pulse phase that
/// drives its reach in and out (the angle advances each frame by its spin rate).
struct Blade {
    dir: [f32; 3],
    angle_deg: f32,
    spin_deg_per_frame: f32,
    color: [f32; 3],
}

pub struct LightSphereEffect {
    center: [f32; 3],
    age_frames: f32,
    distance: f32,
    blades: Vec<Blade>,
    /// `Lightsphere2` (381) keeps the same blade burst alive as a buff aura —
    /// it never self-terminates and never fades out (the holder reaps it).
    persistent: bool,
}

impl LightSphereEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self::build(world_pos, false)
    }

    /// `Lightsphere2` (381): the same blade burst, but persistent (buff aura).
    pub fn new_persistent(world_pos: [f32; 3]) -> Self {
        Self::build(world_pos, true)
    }

    fn build(world_pos: [f32; 3], persistent: bool) -> Self {
        let base_seed = (world_pos[0] * 53.0 + world_pos[2] * 29.0) as i64 as u32 ^ 0x1162_5EED;
        // Evenly cover the sphere (Fibonacci spiral) so blades pierce every
        // direction with no clumping, then randomise each blade's pulse.
        let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
        let n = BLADE_COUNT as f32;
        let blades = (0..BLADE_COUNT)
            .map(|i| {
                let fi = i as f32;
                let y = 1.0 - (fi + 0.5) / n * 2.0;
                let r = (1.0 - y * y).max(0.0).sqrt();
                let theta = golden * fi;
                let dir = [r * theta.cos(), y, r * theta.sin()];

                let h = hash_u32(base_seed.wrapping_add((i as u32).wrapping_mul(2_654_435_761)));
                Blade {
                    dir,
                    angle_deg: (h % 360) as f32,
                    spin_deg_per_frame: ((h >> 9) & 1) as f32 + 1.0, // 1 or 2 deg/frame
                    color: if (h >> 10) & 1 == 0 { BLADE_COLOR_A } else { BLADE_COLOR_B },
                }
            })
            .collect();
        Self {
            center: world_pos,
            age_frames: 0.0,
            distance: DISTANCE_START,
            blades,
            persistent,
        }
    }

    fn alpha(&self) -> f32 {
        let fade_in = (self.age_frames / BLADE_FADE_IN_FRAMES).clamp(0.0, 1.0);
        // The persistent buff aura holds full alpha; the one-shot fades out.
        let fade_out = if !self.persistent && self.age_frames > BLADE_FADE_OUT_START {
            (1.0 - (self.age_frames - BLADE_FADE_OUT_START) / BLADE_FADE_OUT_FRAMES).clamp(0.0, 1.0)
        } else {
            1.0
        };
        BLADE_ALPHA * fade_in * fade_out
    }
}

impl Effect for LightSphereEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        let frames = ctx.delta * FPS;
        self.age_frames += frames;
        if self.distance < DISTANCE_CAP {
            self.distance = (self.distance * DISTANCE_GROWTH_PER_FRAME.powf(frames)).min(DISTANCE_CAP);
        }
        for blade in &mut self.blades {
            blade.angle_deg = (blade.angle_deg + blade.spin_deg_per_frame * frames) % 360.0;
        }
        if !self.persistent && self.age_frames >= BLADE_TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let alpha = self.alpha();
        if alpha <= 0.0 {
            return;
        }
        let [cx, cy, cz] = self.center;
        let base_y = cy - BLADE_LIFT;
        let hw = BLADE_HALF_WIDTH;
        for blade in &self.blades {
            // Reach pulses in and out as the blade stabs outward (range 0..2·distance).
            let reach = self.distance * (1.0 + blade.angle_deg.to_radians().sin());
            let tip = [
                cx + reach * blade.dir[0],
                cy + reach * blade.dir[1] - BLADE_LIFT,
                cz + reach * blade.dir[2],
            ];
            let color = [blade.color[0], blade.color[1], blade.color[2], alpha];

            // Two crossed degenerate quads (tip doubled): base cross at the
            // centre along world X then world Z, apex at the blade tip. Pushed
            // behind so the caster sprite occludes the burst (blades read as
            // radiating from behind the character).
            out.push_behind(EffectPrimitiveDraw::WorldQuad {
                corners: [[cx + hw, base_y, cz], tip, tip, [cx - hw, base_y, cz]],
                uv: UNIT_UV,
                texture: WHITE02_TEXTURE,
                color,
                blend: BlendKind::Additive,
                no_depth: false,
            });
            out.push_behind(EffectPrimitiveDraw::WorldQuad {
                corners: [[cx, base_y, cz + hw], tip, tip, [cx, base_y, cz - hw]],
                uv: UNIT_UV,
                texture: WHITE02_TEXTURE,
                color,
                blend: BlendKind::Additive,
                no_depth: false,
            });
        }
    }
}

/// `Lightsphere2` (381) is the same blade burst as 348 but persistent (a buff
/// aura): kept alive by the status that spawned it; the holder reaps it at the
/// sentinel duration (the viewer clamps to 5 s).
pub const LIGHTSPHERE2_DURATION_MS: u32 = u32::MAX;

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(frames: f32) -> EffectUpdateCtx {
        EffectUpdateCtx {
            delta: frames / FPS,
            camera_target: None,
            caster_yaw: None,
        }
    }

    fn render_ctx() -> EffectRenderCtx {
        EffectRenderCtx {
            camera: Default::default(),
            screen_w: 800.0,
            screen_h: 600.0,
            elapsed: 0.0,
        }
    }

    /// Blades render BEHIND the entity, so they land in `behind`, not `primitives`.
    fn behind_draws<E: Effect>(e: &E) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        assert!(list.primitives.is_empty(), "blades draw behind the entity");
        list.behind
    }

    #[test]
    fn lightsphere_radiates_blue_additive_blades_behind_the_entity_that_reach_further_over_time() {
        let mut e = LightSphereEffect::new([0.0; 3]);
        e.update(&ctx(5.0));
        let prims = behind_draws(&e);
        // Two crossed quads per blade — a dense burst, not one sphere.
        assert_eq!(prims.len(), BLADE_COUNT * 2);
        match &prims[0] {
            EffectPrimitiveDraw::WorldQuad { color, blend, corners, .. } => {
                assert_eq!(*blend, BlendKind::Additive);
                assert!(color[2] > color[0], "bluish");
                // Tip (corner 1) is lifted off the base cross (corner 0).
                assert!((corners[1][1] - corners[0][1]).abs() > 0.0);
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
        // `distance` creeps outward toward the cap as the effect ages.
        let early = e.distance;
        for _ in 0..200 {
            e.update(&ctx(1.0));
        }
        assert!(e.distance > early);
        assert!((e.distance - DISTANCE_CAP).abs() < 1e-3, "distance caps at 10");
    }

    #[test]
    fn lightsphere2_is_the_same_blade_burst_but_persistent() {
        let mut e = LightSphereEffect::new_persistent([0.0; 3]);
        // Survives well past 348's one-shot lifetime and never fades out.
        let mut status = EffectStatus::Running;
        for _ in 0..2000 {
            status = e.update(&ctx(1.0));
        }
        assert_eq!(status, EffectStatus::Running, "persistent");
        let prims = behind_draws(&e);
        assert_eq!(prims.len(), BLADE_COUNT * 2);
        match &prims[0] {
            EffectPrimitiveDraw::WorldQuad { texture, blend, color, .. } => {
                assert_eq!(*texture, WHITE02_TEXTURE);
                assert_eq!(*blend, BlendKind::Additive);
                assert!((color[3] - BLADE_ALPHA).abs() < 1e-4, "holds peak alpha (no fade-out)");
            }
            other => panic!("expected WorldQuad, got {other:?}"),
        }
    }
}
