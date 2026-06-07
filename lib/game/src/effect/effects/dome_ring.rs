//! `EF_MAGNUM2` (Spiral Pierce, 339) and `EF_GI_EXPLOSION` (Really Big
//! Circle, 514) — cone-band ring strips built from concentric expanding
//! rings.
//!
//! Both render as the [`Frustum`] cone-band the casting family uses
//! (`saint_casting.rs` is the closest sibling): a ring at `distance` whose
//! top edge flares outward+up along `rise_angle`, decomposing
//! `max_height` into `(cos·rise outward, sin·rise up)`.
//!
//! * **Magnum2** — a *plain* cone band (no per-segment
//!   dome). One ring prim spawns every 3 frames over parent
//!   frames 25–55 (11 spawns), each with two sub-rings (`ec=0,1`) that differ
//!   only in rotation / rise / alpha cap. Each ring's shape is fixed at spawn;
//!   only its alpha pulses. The stacked, time-staggered rings
//!   read as a spiralling crown.
//! * **GiExplosion** — a *dome* cone band: four concentric
//!   arc rings (315° sweep) whose per-segment height follows
//!   the half-sine bell ([`FrustumWaveMode::SaintBell`]) and **grows** via
//!   `sin(process)`. The ring radius shrinks 0.2/frame, so as the crown rises
//!   it also converges inward — the "teepee" the reference gif shows.
//!
//! Magnum2 holds for the full parent duration; GiExplosion rises over ~90
//! frames then holds and fades over the back third. The arc seams of the four
//! GiExplosion rings are spread 90° apart so they don't overlap, and the ring
//! radius converges inward as the dome grows for the teepee silhouette. Both
//! effects are unlit additive bands so they read against the ground.
//!
//! Reference gifs: `300-350/339.gif`, `500-550/514.gif`.

use crate::effect::draw::{
    BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus, FrustumWaveMode,
};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};

pub const MAGNUM2_TEXTURE: &str = "ring_yellow.tga";
pub const GI_EXPLOSION_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[MAGNUM2_TEXTURE, GI_EXPLOSION_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// 20 ring segments.
const RING_SIDES: u32 = 20;
const RING_UV_REPEAT: f32 = 1.0;

// ───────────────────────── Magnum2 (339) ─────────────────────────

/// Parent lives 100 frames; the last ring spawns at frame 55 and lives ~11
/// frames, so the visible end is ~66 frames. Round up to the parent's 100.
const MAGNUM2_TOTAL_FRAMES: f32 = 100.0;
pub const MAGNUM2_TOTAL_DURATION_MS: u32 =
    (MAGNUM2_TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// World-unit scale. Magnum2's `distance` 5–11 and `max_height` 20
/// are large literals; downscaled uniformly so the crown stands roughly
/// sprite-tall against the gif.
const MAGNUM2_SCALE: f32 = 0.7;

const MAGNUM2_FIRST_SPAWN_FRAME: u32 = 25;
const MAGNUM2_LAST_SPAWN_FRAME: u32 = 55;
const MAGNUM2_SPAWN_PERIOD: u32 = 3;

/// One Magnum2 sub-ring. Shape is fixed at spawn; only
/// `process`/`alpha` evolve.
#[derive(Clone, Copy)]
struct MagnumRing {
    distance: f32,
    rise_deg: f32,
    max_height: f32,
    rot_start_deg: f32,
    /// `ec` selects the alpha cap: ec0 → +10/frame to 80, ec1 → +5
    /// to 40.
    ec: u8,
    process: i32,
    alpha_b: f32,
}

impl MagnumRing {
    fn alpha_cap(&self) -> f32 {
        if self.ec == 0 { 80.0 } else { 40.0 }
    }
    fn alpha_ramp(&self) -> f32 {
        if self.ec == 0 { 10.0 } else { 5.0 }
    }
    fn step(&mut self) {
        self.process += 1;
        if self.process < 9 {
            self.alpha_b = (self.alpha_b + self.alpha_ramp()).min(self.alpha_cap());
        } else {
            self.alpha_b -= 30.0;
        }
    }
    fn alive(&self) -> bool {
        self.alpha_b > 0.0
    }
}

pub struct MagnumSpiralEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    last_processed_frame: u32,
    spawn_count: u32,
    rings: Vec<MagnumRing>,
}

impl MagnumSpiralEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        Self {
            world_pos,
            age_frames: 0.0,
            last_processed_frame: 0,
            spawn_count: 0,
            rings: Vec::with_capacity(22),
        }
    }

    /// Spawn the two sub-rings for the prim launched at parent frame `frame`.
    fn spawn_at(&mut self, frame: u32) {
        let time = (frame - MAGNUM2_FIRST_SPAWN_FRAME) as f32;
        for ec in 0u8..2 {
            self.rings.push(MagnumRing {
                distance: 5.0 + time * 0.2,
                rise_deg: 70.0 - (ec as f32) * 8.0 - time * 2.0,
                max_height: 20.0 - time * 0.7,
                rot_start_deg: (ec as f32) * 9.0 + time * 6.0,
                ec,
                process: 0,
                alpha_b: 0.0,
            });
        }
    }

    fn integrate_frames(&mut self, target: u32) {
        while self.last_processed_frame < target {
            let f = self.last_processed_frame + 1;
            // Spawn cadence: frames 25,28,…,55, capped at 11 prims.
            if (MAGNUM2_FIRST_SPAWN_FRAME..=MAGNUM2_LAST_SPAWN_FRAME).contains(&f)
                && (f - MAGNUM2_FIRST_SPAWN_FRAME).is_multiple_of(MAGNUM2_SPAWN_PERIOD)
                && self.spawn_count < 11
            {
                self.spawn_at(f);
                self.spawn_count += 1;
            }
            for r in &mut self.rings {
                r.step();
            }
            self.rings.retain(|r| r.alive());
            self.last_processed_frame = f;
        }
    }
}

impl Effect for MagnumSpiralEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = (self.age_frames as u32).min(MAGNUM2_TOTAL_FRAMES as u32);
        self.integrate_frames(target);
        if self.age_frames >= MAGNUM2_TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        // The ring sits a few units up off the ground
        // (native RO -Y = up).
        let base = [
            self.world_pos[0],
            self.world_pos[1] - 3.0 * MAGNUM2_SCALE,
            self.world_pos[2],
        ];
        for r in &self.rings {
            if r.alpha_b <= 0.0 {
                continue;
            }
            let (sin_rise, cos_rise) = r.rise_deg.to_radians().sin_cos();
            let max_h = r.max_height * MAGNUM2_SCALE;
            let bottom = r.distance * MAGNUM2_SCALE;
            out.push(EffectPrimitiveDraw::Frustum {
                base,
                bottom_size: bottom,
                top_size: bottom + cos_rise * max_h,
                height: sin_rise * max_h,
                sides: RING_SIDES,
                arc_angle_deg: 360.0,
                rotation: r.rot_start_deg.to_radians(),
                uv_repeat: RING_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: 0.0,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::Sine,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: MAGNUM2_TEXTURE,
                color: [1.0, 1.0, 1.0, r.alpha_b / 255.0],
                blend: BlendKind::Additive,
            });
        }
    }
}

// ─────────────────────── GiExplosion (514) ───────────────────────

/// Parent lives 300 frames. The crown grows over ~90 frames then holds; we
/// fade it over the back third so it doesn't pop off. Pinned to the gif's ~3 s.
const GI_TOTAL_FRAMES: f32 = 180.0;
pub const GI_EXPLOSION_TOTAL_DURATION_MS: u32 =
    (GI_TOTAL_FRAMES / FRAMES_PER_SECOND * 1000.0) as u32;

/// `distance` 18.5–20 and `max_height` 15 are large literals; downscaled to
/// match the gif's big ring (~3 sprites wide).
const GI_SCALE: f32 = 0.6;
const GI_EMITTERS: usize = 4;
const GI_ARC_DEG: f32 = 315.0;
const GI_RISE_DEG: f32 = 50.0;
const GI_MAX_HEIGHT: f32 = 15.0;
/// Frames over which the dome rises (the sine growth saturates at frame 90).
const GI_GROW_FRAMES: f32 = 90.0;
const GI_FADE_START_FRAME: f32 = 120.0;

#[derive(Clone, Copy)]
struct GiRing {
    distance: f32,
    rot_start_deg: f32,
    alpha_b: f32,
}

pub struct GiExplosionEffect {
    world_pos: [f32; 3],
    age_frames: f32,
    last_processed_frame: u32,
    rings: [GiRing; GI_EMITTERS],
}

impl GiExplosionEffect {
    pub fn new(world_pos: [f32; 3]) -> Self {
        let rings = std::array::from_fn(|ec| GiRing {
            distance: 20.0 - ec as f32 * 0.5,
            // Randomised start rotation; spread the four rings
            // deterministically so the arc seams don't overlap.
            rot_start_deg: ec as f32 * 90.0,
            alpha_b: 0.0,
        });
        Self {
            world_pos,
            age_frames: 0.0,
            last_processed_frame: 0,
            rings,
        }
    }

    fn integrate_frames(&mut self, target: u32) {
        while self.last_processed_frame < target {
            for r in &mut self.rings {
                // First 30 frames: alpha ramps +8/frame to 160.
                let process = self.last_processed_frame + 1;
                if process <= 30 {
                    r.alpha_b = (r.alpha_b + 8.0).min(160.0);
                }
                r.rot_start_deg = (r.rot_start_deg + 4.0).rem_euclid(360.0);
                r.distance -= 0.2;
            }
            self.last_processed_frame += 1;
        }
    }

    /// Dome growth factor: a sine rise saturating at frame 90.
    fn grow(&self) -> f32 {
        let p = self.age_frames.min(GI_GROW_FRAMES);
        (p / GI_GROW_FRAMES * std::f32::consts::FRAC_PI_2).sin()
    }

    fn fade(&self) -> f32 {
        if self.age_frames <= GI_FADE_START_FRAME {
            1.0
        } else {
            (1.0 - (self.age_frames - GI_FADE_START_FRAME) / (GI_TOTAL_FRAMES - GI_FADE_START_FRAME))
                .clamp(0.0, 1.0)
        }
    }
}

impl Effect for GiExplosionEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age_frames += ctx.delta * FRAMES_PER_SECOND;
        let target = (self.age_frames as u32).min(GI_TOTAL_FRAMES as u32);
        self.integrate_frames(target);
        if self.age_frames >= GI_TOTAL_FRAMES {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        let grow = self.grow();
        let fade = self.fade();
        let (sin_rise, cos_rise) = GI_RISE_DEG.to_radians().sin_cos();
        // The crown rises as `max_height * sin(process)` — the bell wave gives
        // the per-segment spike profile (tall opposite the seam, zero at it).
        let max_h = GI_MAX_HEIGHT * grow * GI_SCALE;
        for r in &self.rings {
            if r.alpha_b <= 0.0 || max_h <= 0.0 {
                continue;
            }
            let bottom = r.distance * GI_SCALE;
            out.push(EffectPrimitiveDraw::Frustum {
                base: self.world_pos,
                bottom_size: bottom,
                top_size: bottom + cos_rise * max_h,
                height: sin_rise * max_h,
                sides: RING_SIDES,
                arc_angle_deg: GI_ARC_DEG,
                rotation: r.rot_start_deg.to_radians(),
                uv_repeat: RING_UV_REPEAT,
                uv_scroll: [0.0, 0.0],
                wave_amplitude: max_h,
                wave_frequency: 1.0,
                wave_phase: 0.0,
                wave_mode: FrustumWaveMode::SaintBell,
                tilt_x_rad: 0.0,
                rotation_y_rad: 0.0,
                cull_back: false,
                texture: GI_EXPLOSION_TEXTURE,
                color: [1.0, 1.0, 1.0, (r.alpha_b / 255.0) * fade],
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
            screen_w: 256.0,
            screen_h: 256.0,
            elapsed: 0.0,
        }
    }

    fn draws_of<E: Effect>(e: &E) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        e.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step<E: Effect>(e: &mut E, frames: f32) -> EffectStatus {
        e.update(&EffectUpdateCtx {
            delta: frames / FRAMES_PER_SECOND,
            camera_target: None, caster_yaw: None,
        })
    }

    #[test]
    fn magnum2_spawns_rings_over_time_then_dies() {
        let mut e = MagnumSpiralEffect::new([0.0; 3]);
        // Before frame 25 nothing has spawned.
        step(&mut e, 20.0);
        assert!(draws_of(&e).is_empty(), "no rings before frame 25");
        // Mid-life: several time-staggered Frustum rings are alive.
        step(&mut e, 20.0); // frame 40
        let prims = draws_of(&e);
        assert!(!prims.is_empty(), "rings visible mid-life");
        assert!(prims.iter().all(|p| matches!(p, EffectPrimitiveDraw::Frustum { .. })));
        // Exactly 11 prims (×2 sub-rings) spawn total.
        step(&mut e, 60.0);
        assert_eq!(e.spawn_count, 11, "11 prims spawned over frames 25–55");
        assert!(matches!(step(&mut e, 60.0), EffectStatus::Dead));
    }

    #[test]
    fn giexplosion_dome_grows_then_ring_converges_and_dies() {
        let mut e = GiExplosionEffect::new([0.0; 3]);
        step(&mut e, 1.0);
        let early = draws_of(&e);
        assert_eq!(early.len(), GI_EMITTERS, "4 concentric arc rings");
        let arc = match &early[0] {
            EffectPrimitiveDraw::Frustum { arc_angle_deg, wave_mode, .. } => {
                assert_eq!(*wave_mode, FrustumWaveMode::SaintBell);
                *arc_angle_deg
            }
            _ => panic!("expected Frustum"),
        };
        assert!((arc - GI_ARC_DEG).abs() < 1e-3, "315° arc");

        // Crown height grows from near-zero toward full over the grow window.
        let h_early = e.grow();
        step(&mut e, 60.0);
        assert!(e.grow() > h_early, "dome rises over time");

        // Ring radius converges inward (distance shrinks 0.2/frame).
        let r0 = GiExplosionEffect::new([0.0; 3]).rings[0].distance;
        assert!(e.rings[0].distance < r0, "ring converges inward");

        // Dies after total duration.
        assert!(matches!(step(&mut e, 200.0), EffectStatus::Dead));
    }
}
