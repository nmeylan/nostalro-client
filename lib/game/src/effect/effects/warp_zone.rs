//! EF_WARPZONE / EF_WARPZONE2 — looping portal pad.
//! Reference: original game `WarpZone()`.
//! Visible reference: `ro-effects/effects/imgs/50-100/61.gif`.
//!
//! The effect's frame counter wraps at frame 78, so the spawn pattern repeats
//! every 78 frames (~1.3 s @ 60 fps). Each cycle emits:
//!   * frame 0 — one base disc (`alpha_down.tga`, radius 15, duration 158,
//!     fade-in over 11 frames, fade-out last 10);
//!   * frames 0, 28, 56 — an inner ring (`ring_blue.tga`, radius 14, slight
//!     shrink, duration 80, keyframed alpha curve — we
//!     approximate with the same fade-in/fade-out pattern used elsewhere).
//!
//! Orbiting orbital sparks (every 10 frames) are out of scope
//! until a Spark primitive lands (slice F).
//!
//! `EffectId::Warpzone` runs for the spec's `duration_ms` (2.5 s in the
//! generated table → 2 cycles); `EffectId::Warpzone2` runs effectively
//! forever — same emitter, infinite lifetime.

use crate::effect::draw::{BlendKind, EffectDrawList, EffectPrimitiveDraw, EffectStatus};
use crate::effect::effect_trait::{Effect, EffectRenderCtx, EffectUpdateCtx};
use crate::effect::spec::Attach;

pub const BASE_TEXTURE: &str = "alpha_down.tga";
pub const INNER_TEXTURE: &str = "ring_blue.tga";
pub const TEXTURES: &[&str] = &[BASE_TEXTURE, INNER_TEXTURE];

const FRAMES_PER_SECOND: f32 = 60.0;
/// The original game wraps its frame counter to 0 once it hits 78.
const CYCLE_FRAMES: f32 = 78.0;
const CYCLE_S: f32 = CYCLE_FRAMES / FRAMES_PER_SECOND;
/// Inner-ring cadence within a cycle (every 28 frames up to a 117-frame cap,
/// the upper bound never triggers because of the wrap at 78).
const INNER_INTERVAL_FRAMES: f32 = 28.0;
const INNER_INTERVAL_S: f32 = INNER_INTERVAL_FRAMES / FRAMES_PER_SECOND;
const BASE_DURATION_FRAMES: f32 = 158.0;
const BASE_DURATION_S: f32 = BASE_DURATION_FRAMES / FRAMES_PER_SECOND;
const INNER_DURATION_FRAMES: f32 = 80.0;
const INNER_DURATION_S: f32 = INNER_DURATION_FRAMES / FRAMES_PER_SECOND;

const BASE_RADIUS: f32 = 15.0;
const BASE_MAX_ALPHA: f32 = 128.0 / 255.0;
const BASE_FADE_IN_FRAMES: f32 = 11.0;
const BASE_FADE_OUT_AT_FRAME: f32 = BASE_DURATION_FRAMES - 10.0;

const INNER_RADIUS: f32 = 14.0;
const INNER_RADIUS_SHRINK_PER_FRAME: f32 = 0.15;
const INNER_THICKNESS: f32 = 4.0;
const INNER_MAX_ALPHA: f32 = 200.0 / 255.0;
const INNER_FADE_IN_FRAMES: f32 = 20.0;
/// Fade-out starts at `duration - duration/5 = 64`.
const INNER_FADE_OUT_AT_FRAME: f32 = INNER_DURATION_FRAMES * 0.8;
const INNER_UV_REPEAT: f32 = 4.0;

/// Returned by the factory to differentiate `Warpzone` (timed) from
/// `Warpzone2` (sustained).
#[derive(Clone, Copy)]
pub struct WarpZoneParams {
    pub total_duration_s: f32,
}

pub const PARAMS_BURST: WarpZoneParams = WarpZoneParams {
    total_duration_s: 2.5,
};
pub const PARAMS_SUSTAINED: WarpZoneParams = WarpZoneParams {
    // Effectively infinite — the holder respects the spec's u32::MAX duration.
    total_duration_s: f32::INFINITY,
};

#[derive(Clone, Copy)]
struct BaseDisc {
    age: f32,
}

impl BaseDisc {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, BASE_DURATION_FRAMES)
    }
    fn alive(&self) -> bool {
        self.age < BASE_DURATION_S
    }
    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n <= BASE_FADE_IN_FRAMES {
            BASE_MAX_ALPHA * (n / BASE_FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if n >= BASE_FADE_OUT_AT_FRAME {
            let fade = ((n - BASE_FADE_OUT_AT_FRAME)
                / (BASE_DURATION_FRAMES - BASE_FADE_OUT_AT_FRAME))
                .clamp(0.0, 1.0);
            BASE_MAX_ALPHA * (1.0 - fade)
        } else {
            BASE_MAX_ALPHA
        }
    }
}

#[derive(Clone, Copy)]
struct InnerRing {
    age: f32,
}

impl InnerRing {
    fn frame(&self) -> f32 {
        (self.age * FRAMES_PER_SECOND).clamp(0.0, INNER_DURATION_FRAMES)
    }
    fn alive(&self) -> bool {
        self.age < INNER_DURATION_S
    }
    fn radius(&self) -> f32 {
        (INNER_RADIUS - INNER_RADIUS_SHRINK_PER_FRAME * self.frame()).max(0.0)
    }
    fn alpha(&self) -> f32 {
        let n = self.frame();
        if n <= INNER_FADE_IN_FRAMES {
            INNER_MAX_ALPHA * (n / INNER_FADE_IN_FRAMES).clamp(0.0, 1.0)
        } else if n >= INNER_FADE_OUT_AT_FRAME {
            let fade = ((n - INNER_FADE_OUT_AT_FRAME)
                / (INNER_DURATION_FRAMES - INNER_FADE_OUT_AT_FRAME))
                .clamp(0.0, 1.0);
            INNER_MAX_ALPHA * (1.0 - fade)
        } else {
            INNER_MAX_ALPHA
        }
    }
}

pub struct WarpZoneEffect {
    world_pos: [f32; 3],
    params: WarpZoneParams,
    age: f32,
    next_base_at: f32,
    next_inner_at: f32,
    base_discs: Vec<BaseDisc>,
    inner_rings: Vec<InnerRing>,
}

impl WarpZoneEffect {
    pub fn new(attach: Attach, params: WarpZoneParams) -> Self {
        let world_pos = match attach {
            Attach::WorldPos(p) => p,
            Attach::Entity(_) | Attach::Projectile { .. } => [0.0; 3],
        };
        Self {
            world_pos,
            params,
            age: 0.0,
            next_base_at: 0.0,
            next_inner_at: 0.0,
            base_discs: Vec::new(),
            inner_rings: Vec::new(),
        }
    }
}

impl Effect for WarpZoneEffect {
    fn update(&mut self, ctx: &EffectUpdateCtx) -> EffectStatus {
        self.age += ctx.dt;
        for d in &mut self.base_discs {
            d.age += ctx.dt;
        }
        for r in &mut self.inner_rings {
            r.age += ctx.dt;
        }

        let still_spawning = self.age < self.params.total_duration_s;

        // Catch up scheduled spawns landed during this tick.
        while still_spawning && self.next_base_at <= self.age {
            let initial_age = (self.age - self.next_base_at).max(0.0);
            self.base_discs.push(BaseDisc { age: initial_age });
            self.next_base_at += CYCLE_S;
        }
        while still_spawning && self.next_inner_at <= self.age {
            let initial_age = (self.age - self.next_inner_at).max(0.0);
            self.inner_rings.push(InnerRing { age: initial_age });
            self.next_inner_at += INNER_INTERVAL_S;
        }

        self.base_discs.retain(|d| d.alive());
        self.inner_rings.retain(|r| r.alive());

        if !still_spawning && self.base_discs.is_empty() && self.inner_rings.is_empty() {
            EffectStatus::Dead
        } else {
            EffectStatus::Running
        }
    }

    fn collect_draws(&self, out: &mut EffectDrawList, _ctx: &EffectRenderCtx) {
        for d in &self.base_discs {
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: BASE_RADIUS,
                thickness: BASE_RADIUS, // filled solid disc
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: 1.0,
                texture: BASE_TEXTURE,
                color: [1.0, 1.0, 1.0, d.alpha()],
                blend: BlendKind::Alpha,
            });
        }
        for r in &self.inner_rings {
            let outer = r.radius();
            if outer <= 0.0 {
                continue;
            }
            let thickness = outer.min(INNER_THICKNESS);
            out.push(EffectPrimitiveDraw::GroundDisc {
                center: self.world_pos,
                radius: outer,
                thickness,
                rotation: 0.0,
                arc_angle_deg: 360.0,
                uv_repeat: INNER_UV_REPEAT,
                texture: INNER_TEXTURE,
                color: [1.0, 1.0, 1.0, r.alpha()],
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

    fn draws(effect: &WarpZoneEffect) -> Vec<EffectPrimitiveDraw> {
        let mut list = EffectDrawList::new();
        effect.collect_draws(&mut list, &render_ctx());
        list.primitives
    }

    fn step(effect: &mut WarpZoneEffect, dt: f32) -> EffectStatus {
        effect.update(&EffectUpdateCtx { dt })
    }

    fn count_base(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { texture, .. } if *texture == BASE_TEXTURE))
            .count()
    }
    fn count_inner(prims: &[EffectPrimitiveDraw]) -> usize {
        prims
            .iter()
            .filter(|p| matches!(p, EffectPrimitiveDraw::GroundDisc { texture, .. } if *texture == INNER_TEXTURE))
            .count()
    }

    #[test]
    fn spawns_a_base_disc_and_inner_at_frame_zero() {
        let mut wz = WarpZoneEffect::new(Attach::WorldPos([0.0; 3]), PARAMS_SUSTAINED);
        step(&mut wz, 0.0);
        let prims = draws(&wz);
        assert_eq!(count_base(&prims), 1);
        assert_eq!(count_inner(&prims), 1);
    }

    #[test]
    fn inner_ring_spawn_cadence_is_28_frames() {
        let mut wz = WarpZoneEffect::new(Attach::WorldPos([0.0; 3]), PARAMS_SUSTAINED);
        step(&mut wz, 0.0);
        assert_eq!(count_inner(&draws(&wz)), 1);
        // Walk forward 28 frames → 2nd inner spawn.
        step(&mut wz, INNER_INTERVAL_S + 0.01);
        assert_eq!(count_inner(&draws(&wz)), 2);
        step(&mut wz, INNER_INTERVAL_S);
        assert_eq!(count_inner(&draws(&wz)), 3);
    }

    #[test]
    fn base_disc_respawns_each_cycle() {
        let mut wz = WarpZoneEffect::new(Attach::WorldPos([0.0; 3]), PARAMS_SUSTAINED);
        step(&mut wz, 0.0);
        // After one cycle (78 frames) the previous base still lives (158-frame
        // duration) and a new one is spawned, so we see two simultaneously.
        step(&mut wz, CYCLE_S + 0.01);
        assert_eq!(count_base(&draws(&wz)), 2);
    }

    #[test]
    fn burst_variant_stops_spawning_after_duration() {
        let mut wz = WarpZoneEffect::new(Attach::WorldPos([0.0; 3]), PARAMS_BURST);
        let mut t = 0.0;
        while t < PARAMS_BURST.total_duration_s + 0.5 {
            step(&mut wz, 1.0 / 60.0);
            t += 1.0 / 60.0;
        }
        let bases_after_burst = count_base(&draws(&wz));
        // Past spec duration we may still see dying primitives — but well past
        // the longest sub-life (158 frames ≈ 2.63 s), nothing remains.
        let mut t2 = 0.0;
        while t2 < BASE_DURATION_S + 0.2 {
            step(&mut wz, 1.0 / 60.0);
            t2 += 1.0 / 60.0;
        }
        assert!(
            count_base(&draws(&wz)) < bases_after_burst.max(1),
            "burst run must wind down to zero"
        );
    }

    #[test]
    fn burst_variant_dies_after_subprimitives_finish() {
        let mut wz = WarpZoneEffect::new(Attach::WorldPos([0.0; 3]), PARAMS_BURST);
        let mut status = EffectStatus::Running;
        let mut t = 0.0;
        // Walk well past spec duration + longest sub-life.
        while t < PARAMS_BURST.total_duration_s + BASE_DURATION_S + 1.0 {
            status = step(&mut wz, 1.0 / 60.0);
            t += 1.0 / 60.0;
            if matches!(status, EffectStatus::Dead) {
                break;
            }
        }
        assert!(matches!(status, EffectStatus::Dead));
    }
}
